//! Pins for the explicit-handle graph frontend (`cdda_htn::graph`): the
//! data-driven construction API that compiles JSON/DSL definitions into the
//! same baked network the function frontend produces.
//!
//! The load-bearing pin is the identity model: two primitives compiled from
//! the **same closure expression** (the shape every kernel-style factory
//! produces) must bake as **distinct tasks** because their handles differ —
//! a closure-`TypeId` identity scheme collapses them (the captured values do
//! not change the closure's type).

use cdda_htn::ecs::{htn_ai_system, HtnAgent, HtnConfig};
use cdda_htn::graph::{BakedGraph, GraphBuilder, PrimitiveBuilder};
use cdda_htn::planner::HtnPlanner;
use cdda_htn::selection::HtnSearchStrategy;
use cdda_htn::state::PlanState;
use cdda_htn::HtnError;
use bevy_ecs::prelude::*;

#[derive(Component, Clone, Default, Debug, PartialEq)]
struct Fuel(i32);
#[derive(Component, Clone, Default, Debug, PartialEq)]
struct Steps(u32);
#[derive(Component, Clone, Default, Debug, PartialEq)]
struct Hunger(u32);

/// A kernel-shaped factory: every call compiles the SAME closure expression
/// `move |fuel: &mut Fuel| fuel.0 -= cost`, differing only in the captured
/// value. Under function-identity recording both would share one `TypeId`;
/// under handle identity they are separate nodes.
fn define_burn(p: &mut PrimitiveBuilder<'_>, cost: i32) {
    p.effect(move |fuel: &mut Fuel| fuel.0 -= cost).cost(cost as f32);
}

#[test]
fn parameterized_native_calls_from_one_closure_stay_distinct() {
    let mut graph = GraphBuilder::new();
    let root = graph.reserve("root");
    let burn_small = graph.reserve("burn:small");
    let burn_big = graph.reserve("burn:big");

    graph.define_primitive(burn_small, |p| define_burn(p, 1));
    graph.define_primitive(burn_big, |p| define_burn(p, 2));
    graph.define_compound(root, |c| {
        c.method().then(burn_small).then(burn_big);
    });

    let baked = graph.build(root).expect("distinct handles bake distinctly");
    let small_idx = baked.index(burn_small).expect("small mapped");
    let big_idx = baked.index(burn_big).expect("big mapped");
    assert_ne!(
        small_idx, big_idx,
        "differently parameterized calls must bake as distinct tasks"
    );

    // The plan walks both nodes in order, and executing it applies each
    // node's own captured parameter (1 then 2, not one of them twice).
    let mut planner = HtnPlanner::new(baked.domain());
    let state = PlanState::build(&baked.domain().components)
        .set(Fuel(10))
        .finish();
    let plan = planner.plan(baked.root_index(), &state).expect("plan");
    assert_eq!(plan.steps(), &[small_idx as u32, big_idx as u32]);

    let mut state = state;
    for &step in plan.steps() {
        if let Some(cdda_htn::domain::Task::Primitive(p)) =
            baked.domain().tasks.get(step as usize)
        {
            for e in &p.effects {
                e.apply(&mut state);
            }
        }
    }
    assert_eq!(state.get::<Fuel>().unwrap().0, 7, "each node used its own compiled value");
}

#[test]
fn forward_references_and_recursion_bake_and_terminate() {
    let mut graph = GraphBuilder::new();
    // Reserved before any definition exists — forward references must work.
    let root = graph.reserve("count:down");
    let tick = graph.reserve("count:tick");

    graph.define_primitive(tick, |p| {
        p.precondition(|s: &Steps| s.0 > 0)
            .effect(|s: &mut Steps| s.0 -= 1);
    });
    // A recursive compound referencing itself and a sibling defined earlier.
    graph.define_compound(root, |c| {
        c.method()
            .precondition(|s: &Steps| s.0 > 0)
            .then(tick)
            .then(root);
        c.method().precondition(|s: &Steps| s.0 == 0);
    });

    let baked = graph.build(root).expect("recursive graph bakes");
    let mut planner = HtnPlanner::new(baked.domain());
    let tick_idx = baked.index(tick).unwrap() as u32;

    let state = PlanState::build(&baked.domain().components)
        .set(Steps(3))
        .finish();
    let plan = planner.plan(baked.root_index(), &state).expect("terminates");
    assert_eq!(
        plan.steps(),
        &[tick_idx, tick_idx, tick_idx],
        "recursion expands to one primitive occurrence per remaining step"
    );

    let done = PlanState::build(&baked.domain().components)
        .set(Steps(0))
        .finish();
    let empty = planner.plan(baked.root_index(), &done).expect("terminal branch");
    assert!(empty.steps().is_empty(), "terminal empty method decomposes to nothing");
}

#[test]
fn handle_index_mapping_and_label_introspection_agree() {
    let mut graph = GraphBuilder::new();
    let root = graph.reserve("cdda:root");
    let eat = graph.reserve(format!("cdda:eat_{}", 1)); // owned label
    let wander = graph.reserve("cdda:wander");

    graph.define_primitive(eat, |p| { p.effect(|h: &mut Hunger| h.0 = 0); });
    graph.define_primitive(wander, |p| { p.effect(|h: &mut Hunger| h.0 += 1); });
    graph.define_compound(root, |c| {
        c.method()
            .named("eat-first".to_string())
            .precondition(|h: &Hunger| h.0 >= 40)
            .then(eat);
        c.method().then(wander);
    });

    let baked = graph.build(root).unwrap();
    let eat_idx = baked.index(eat).unwrap();
    // Labels are interned display metadata: introspection by name resolves to
    // the same node the handle maps to.
    let by_name_idx = baked
        .domain()
        .tasks
        .iter()
        .position(|t| t.name() == "cdda:eat_1")
        .expect("interned label lands in the domain");
    assert_eq!(by_name_idx, eat_idx, "label lookup and handle mapping agree");
    // The domain root is the graph root, not task 0 of reservation order.
    assert_eq!(baked.root_index(), baked.index(root).unwrap());
    assert_eq!(baked.root_handle(), root);
}

#[test]
fn undefined_nodes_are_rejected() {
    let mut graph = GraphBuilder::new();
    let root = graph.reserve("root");
    let ghost = graph.reserve("ghost");
    graph.define_compound(root, |c| {
        c.method().then(ghost);
    });
    let err = graph.build(root).expect_err("undefined node must fail");
    match err {
        HtnError::Builder { errors } => {
            assert!(
                errors.iter().any(|e| e.contains("ghost") && e.contains("never defined")),
                "error names the undefined label: {errors:?}"
            );
        }
        other => panic!("expected Builder error, got {other:?}"),
    }
}

#[test]
fn duplicate_definitions_are_rejected() {
    let mut graph = GraphBuilder::new();
    let root = graph.reserve("root");
    let task = graph.reserve("twice");
    graph.define_primitive(task, |p| { p.cost(1.0); });
    graph.define_compound(task, |c| {
        c.method();
    });
    graph.define_compound(root, |c| {
        c.method().then(task);
    });
    let err = graph.build(root).expect_err("double define must fail");
    match err {
        HtnError::Builder { errors } => assert!(
            errors.iter().any(|e| e.contains("defined twice")),
            "error reports the double definition: {errors:?}"
        ),
        other => panic!("expected Builder error, got {other:?}"),
    }
}

#[test]
fn foreign_handles_are_rejected() {
    // A `define` with another builder's handle records the error; the build
    // that owns the graph reports it.
    let mut a = GraphBuilder::new();
    let root_a = a.reserve("a:root");
    let mut b = GraphBuilder::new();
    let node_b = b.reserve("b:node");

    a.define_primitive(node_b, |p| { p.cost(1.0); });
    a.define_compound(root_a, |c| {
        c.method();
    });
    let err = a.build(root_a).expect_err("foreign define handle");
    match err {
        HtnError::Builder { errors } => assert!(
            errors.iter().any(|e| e.contains("different GraphBuilder")),
            "foreign define handle reported: {errors:?}"
        ),
        other => panic!("expected Builder error, got {other:?}"),
    }

    // A method subtask referencing a foreign handle is reported too.
    let mut c = GraphBuilder::new();
    let root_c = c.reserve("c:root");
    c.define_compound(root_c, |c| {
        c.method().then(node_b);
    });
    let err = c.build(root_c).expect_err("foreign subtask handle");
    match err {
        HtnError::Builder { errors } => assert!(
            errors.iter().any(|e| e.contains("different GraphBuilder")),
            "foreign subtask handle reported: {errors:?}"
        ),
        other => panic!("expected Builder error, got {other:?}"),
    }

    // And a foreign root.
    let mut d = GraphBuilder::new();
    let root_d = d.reserve("d:root");
    d.define_compound(root_d, |c| {
        c.method();
    });
    assert!(d.build(node_b).is_err(), "foreign root handle is an error");
}

#[test]
fn root_must_be_compound() {
    let mut graph = GraphBuilder::new();
    let root = graph.reserve("root");
    graph.define_primitive(root, |p| { p.cost(1.0); });
    let err = graph.build(root).expect_err("primitive root");
    match err {
        HtnError::Builder { errors } => assert!(
            errors.iter().any(|e| e.contains("must be a compound task")),
            "root-kind error reported: {errors:?}"
        ),
        other => panic!("expected Builder error, got {other:?}"),
    }
}

#[test]
fn builder_selection_cost_and_partial_order_through_handles() {
    use cdda_htn::domain::SelectionPolicy;

    let mut graph = GraphBuilder::new();
    let root = graph.reserve("root");
    let cheap = graph.reserve("cheap");
    let dear = graph.reserve("dear");
    let unordered_a = graph.reserve("unordered:a");
    let unordered_b = graph.reserve("unordered:b");

    graph.define_primitive(cheap, |p| { p.effect(|f: &mut Fuel| f.0 -= 1).cost(1.0); });
    graph.define_primitive(dear, |p| { p.effect(|f: &mut Fuel| f.0 -= 1).cost(5.0); });
    graph.define_primitive(unordered_a, |p| { p.effect(|f: &mut Fuel| f.0 -= 1).cost(1.0); });
    graph.define_primitive(unordered_b, |p| { p.effect(|f: &mut Fuel| f.0 -= 1).cost(1.0); });

    graph.define_compound(root, |c| {
        c.select(SelectionPolicy::FirstMatch);
        // CostBounded must prefer the cheaper alternative.
        c.method().then(dear);
        c.method().then(cheap);
        // A partially-ordered member set through handles.
        let mut set = c.method();
        set.subtask(unordered_a);
        set.subtask(unordered_b);
    });

    let baked = graph.build(root).expect("well-formed");
    let mut planner = HtnPlanner::new(baked.domain());
    planner.set_strategy(HtnSearchStrategy::CostBounded);
    let state = PlanState::build(&baked.domain().components)
        .set(Fuel(0))
        .finish();
    let plan = planner.plan(baked.root_index(), &state).expect("plan");
    assert_eq!(
        plan.steps(),
        &[baked.index(cheap).unwrap() as u32],
        "CostBounded picks the cheaper alternative"
    );
}

#[test]
fn graph_built_domain_runs_through_the_stock_driver() {
    let mut graph = GraphBuilder::new();
    let root = graph.reserve("charge");
    let gather = graph.reserve("gather");

    graph.define_primitive(gather, |p| {
        p.precondition(|battery: &Battery| battery.0 < 3)
            .effect(|battery: &mut Battery| battery.0 += 1);
    });
    graph.define_compound(root, |c| {
        c.method().then(gather).then(root); // recursive until charged
        c.method().precondition(|battery: &Battery| battery.0 >= 3);
    });

    let baked: BakedGraph = graph.build(root).expect("bakes");
    let mut world = World::new();
    world.insert_resource(HtnConfig::new(baked.into_domain()));
    let entity = world.spawn((Battery(0), HtnAgent::default())).id();

    // One primitive per tick until full; recursion in a data-defined graph
    // executes exactly like a hand-written one.
    for expected in 1..=3 {
        htn_ai_system(&mut world);
        assert_eq!(
            world.get::<Battery>(entity).unwrap().0,
            expected,
            "effect committed per driver tick"
        );
    }
    htn_ai_system(&mut world);
    assert!(
        world.get::<HtnAgent>(entity).unwrap().plan().is_none(),
        "terminal branch reached; plan dropped"
    );
    assert_eq!(world.get::<Battery>(entity).unwrap().0, 3);
}

#[derive(Component, Clone, Default, Debug, PartialEq)]
struct Battery(i32);
