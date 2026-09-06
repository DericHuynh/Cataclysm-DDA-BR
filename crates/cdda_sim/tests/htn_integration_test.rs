//! End-to-end pins for `cdda_sim::ai::htn`: JSON-defined `htn_compound`s +
//! native kernels → compiled planner domain → an agent that plans, submits
//! correlated action requests, and advances its plan ONLY on matching
//! `Completed` outcomes — the simulation decides what actually happens.
//!
//! Test flow per simulated round (systems called directly, no App):
//! `drive_htn_system` (submits at most one intent) → `collect_intents`
//! (stamps the correlated request id) → `resolve_intents` (terminal verdict).

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::ai::{PlannerHtn, PlannerNone};
use cdda_components::def::ItemCategory;
use cdda_components::intent::{
    ActionIntent, ActionOutcome, ActionOutcomeState, ActionRequestCounter, ActionRequestId,
    IntentQueue,
};
use cdda_components::item::{DefOrigin, InsideContainer};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_data::interner::ItemTypeRegistry;
use cdda_data::loader::Loader;
use cdda_sim::ai::htn::{
    compile_domain, drive_htn_system, HtnAgentState, HtnBrain, HtnRuntime, KernelRegistry,
};
use cdda_sim::intent::systems::{collect_intents, resolve_intents};
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const HTN_DATA: &str = r#"[
    {"type": "ITEM", "id": "tool_hammer", "category": "tools"},
    {"type": "ITEM", "id": "food_ration", "category": "food"},
    {"type": "ITEM_CATEGORY", "id": "tools"},
    {"type": "ITEM_CATEGORY", "id": "food"},

    {"type": "htn_compound", "id": "core:grab", "parameters": ["target"],
     "methods": [
        {"id": "take",
         "when": [{"predicate": "cdda:adjacent", "args": {"param": "target"}}],
         "steps": [{"operator": "cdda:pickup", "args": {"param": "target"}}]},
        {"id": "walk",
         "when": [{"predicate": "cdda:has_items", "args": {"scope": "nearby", "param": "target"}}],
         "steps": [
            {"operator": "cdda:approach", "args": {"param": "target"}},
            {"task": "core:grab", "args": {"param": "target"}}
         ]}
     ]},

    {"type": "htn_compound", "id": "core:stock_up",
     "methods": [
        {"id": "have_tool",
         "when": [{"predicate": "cdda:has_items", "args": {"item_category": "tools", "scope": "carried"}}]},
        {"id": "fetch_tool",
         "when": [{"predicate": "cdda:has_items", "args": {"item_category": "tools", "scope": "nearby"}}],
         "steps": [{"task": "core:grab", "args": {"target": {"item_category": "tools"}}}]}
     ]},

    {"type": "htn_compound", "id": "core:idle",
     "methods": [{"id": "wait", "steps": [{"operator": "cdda:wait"}]}]}
]"#;

fn load_registry(dir: &std::path::Path) -> cdda_data::registry::DefRegistry {
    let mut loader = Loader::new(vec![dir.to_path_buf()]);
    loader.load().expect("htn data loads")
}

/// A world with a hammer at (3, 0) and one at (3, 2); agent at (0, 0).
fn world_with_hammers() -> (TestBed, Entity, Vec<Entity>) {
    let mut test = TestBed::new();
    test.insert_resource(IntentQueue::default());
    test.insert_resource(ActionRequestCounter::default());
    let z = ZLevel::new(0);

    let mut intern = ItemTypeRegistry::default();
    let hammer_token = intern.intern("tool_hammer");
    let ration_token = intern.intern("food_ration");
    test.insert_resource(intern);

    let agent = test.spawn((
        PlannerHtn,
        HtnBrain {
            root: "core:stock_up".into(),
            view_radius: 12,
        },
        HtnAgentState::default(),
        ActionPoints {
            current: 10_000,
            speed: 100,
        },
        IsAlive,
        WorldPosition(WorldPos::new(0, 0, z)),
    ));
    // A second, inert entity proves the HTN driver ignores non-HTN agents.
    test.spawn((PlannerNone, IsAlive, WorldPosition(WorldPos::new(9, 9, z))));

    let h1 = test.spawn((
        DefOrigin(hammer_token.0),
        ItemCategory("tools".into()),
        WorldPosition(WorldPos::new(3, 0, z)),
    ));
    let h2 = test.spawn((
        DefOrigin(hammer_token.0),
        ItemCategory("tools".into()),
        WorldPosition(WorldPos::new(3, 2, z)),
    ));
    // A distractor the planner must ignore (no method looks at food).
    test.spawn((
        DefOrigin(ration_token.0),
        ItemCategory("food".into()),
        WorldPosition(WorldPos::new(1, 1, z)),
    ));

    (test, agent, vec![h1, h2])
}

fn compiled_runtime(registry: &cdda_data::registry::DefRegistry) -> HtnRuntime {
    let mut kernels = KernelRegistry::new();
    cdda_sim::ai::htn::kernels::register_default_kernels(&mut kernels);
    let compiled = compile_domain(registry, &kernels).expect("htn domain compiles");
    HtnRuntime::new(compiled, cdda_sim::ai::htn::observe::ItemCatalog::default())
}

/// One simulation round: HTN driver → collector → resolver.
fn round(test: &mut TestBed) {
    drive_htn_system(test.world_mut());
    test.run_system(collect_intents);
    test.run_system(resolve_intents);
}

// ---------------------------------------------------------------------------
// Compilation pins
// ---------------------------------------------------------------------------

#[test]
fn unknown_kernel_symbols_and_references_are_compile_errors() {
    let dir = std::env::temp_dir().join(format!("htn_err_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("htn.json"),
        r#"[
            {"type": "ITEM", "id": "tool_hammer", "category": "tools"},
            {"type": "ITEM_CATEGORY", "id": "tools"},
            {"type": "htn_compound", "id": "core:bad",
             "methods": [
                {"id": "m", "steps": [
                    {"operator": "cdda:teleport", "args": {}}
                ]},
                {"id": "m2", "when": [{"predicate": "cdda:levitating", "args": {"minimum": 1}}]},
                {"id": "m3", "steps": [{"task": "core:never_defined"}]},
                {"id": "m4", "steps": [{"operator": "cdda:pickup", "args": {"item_category": "weapons"}}]}
             ]}
        ]"#,
    )
    .unwrap();

    let registry = load_registry(&dir);
    let mut kernels = KernelRegistry::new();
    cdda_sim::ai::htn::kernels::register_default_kernels(&mut kernels);
    let errors = match compile_domain(&registry, &kernels) {
        Err(errors) => errors,
        Ok(_) => panic!("bad defs must not compile"),
    };

    let all = errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        all.contains("unknown operator kernel `cdda:teleport`"),
        "{all}"
    );
    assert!(
        all.contains("unknown predicate kernel `cdda:levitating`"),
        "{all}"
    );
    assert!(
        all.contains("unknown htn_compound task reference `core:never_defined`"),
        "{all}"
    );
    assert!(all.contains("unknown item category `weapons`"), "{all}");
    // Diagnostics carry the def and method location.
    assert!(all.contains("core:bad"), "{all}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parameterized_calls_specialize_into_distinct_nodes() {
    let dir = std::env::temp_dir().join(format!("htn_spec_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("htn.json"),
        r#"[
            {"type": "ITEM", "id": "tool_hammer", "category": "tools"},
            {"type": "ITEM", "id": "food_ration", "category": "food"},
            {"type": "ITEM_CATEGORY", "id": "tools"},
            {"type": "ITEM_CATEGORY", "id": "food"},
            {"type": "htn_compound", "id": "core:grab", "parameters": ["target"],
             "methods": [{"id": "fetch", "steps": [
                {"operator": "cdda:approach", "args": {"param": "target"}},
                {"operator": "cdda:pickup", "args": {"param": "target"}}]}]},
            {"type": "htn_compound", "id": "core:both",
             "methods": [{"id": "two", "steps": [
                {"task": "core:grab", "args": {"target": {"item_category": "tools"}}},
                {"task": "core:grab", "args": {"target": {"item": "food_ration"}}}]}]}
        ]"#,
    )
    .unwrap();

    let registry = load_registry(&dir);
    let mut kernels = KernelRegistry::new();
    cdda_sim::ai::htn::kernels::register_default_kernels(&mut kernels);
    let compiled = compile_domain(&registry, &kernels).expect("compiles");

    // Node census. `core:grab` is a template (declares parameters): it has
    // NO unbound node — only its two call-site specializations exist, each
    // with its own approach+pickup operators. The two call sites must NOT
    // share nodes even though `core:grab` is one definition.
    // Census: `both` (1) + grab[tools] (1) + grab[item] (1) + 4 operators = 7.
    assert_eq!(compiled.graph.domain().tasks.len(), 7);
    // Compiled def ids resolve to baked indexes; templates do not.
    assert!(compiled.root_index("core:both").is_some());
    assert!(
        compiled.root_index("core:grab").is_none(),
        "parameterized defs have no unbound root"
    );
    // And the execution table covers exactly the four operator nodes.
    assert_eq!(compiled.exec_table.len(), 4);

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// End-to-end behavior pins
// ---------------------------------------------------------------------------

#[test]
fn htn_agent_fetches_a_nearby_tool_through_correlated_requests() {
    let dir = std::env::temp_dir().join(format!("htn_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("htn.json"), HTN_DATA).unwrap();
    let registry = load_registry(&dir);
    let runtime = compiled_runtime(&registry);

    let (mut test, agent, hammers) = world_with_hammers();
    test.insert_resource(runtime);
    let h1 = hammers[0];

    let mut carried = false;
    for i in 0..30 {
        round(&mut test);
        carried = test
            .world()
            .get::<InsideContainer>(h1)
            .map(|c| c.0 == agent)
            .unwrap_or(false);
        let pos = test.world().get::<WorldPosition>(agent).unwrap().get();
        let st = test.world().get::<HtnAgentState>(agent).unwrap();
        eprintln!(
            "round {i}: pos=({},{}) carried={carried} cursor={} plan={:?} outcome={:?}",
            pos.x,
            pos.y,
            st.cursor,
            st.plan.as_ref().map(|p| p.len()),
            test.world()
                .get::<ActionOutcome>(agent)
                .map(|o| (o.request.0, format!("{:?}", o.state)))
        );
        if carried {
            break;
        }
    }
    assert!(carried, "the agent must end up carrying the nearest hammer");

    // The approach steps actually moved the agent toward the hammer.
    let pos = test.world().get::<WorldPosition>(agent).unwrap().get();
    assert_eq!(pos.x, 2, "agent walked to adjacency");
    assert_eq!(pos.y, 0);

    // The picked-up hammer no longer sits on the ground.
    assert!(
        test.world().get::<WorldPosition>(h1).is_none(),
        "carried items must not keep a ground position"
    );

    // Terminal state: the agent is satisfied (empty plan), not wedged.
    let state = test.world().get::<HtnAgentState>(agent).unwrap();
    assert!(state.plan.is_none());
    let _ = hammers[1]; // the second hammer stays on the ground
    assert!(test.world().get::<WorldPosition>(hammers[1]).is_some());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rejected_and_failed_outcomes_trigger_replanning_not_wedging() {
    let dir = std::env::temp_dir().join(format!("htn_reject_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("htn.json"), HTN_DATA).unwrap();
    let registry = load_registry(&dir);
    let runtime = compiled_runtime(&registry);

    let (mut test, agent, _hammers) = world_with_hammers();
    test.insert_resource(runtime);

    // Force a Failed outcome for the agent's next request: give the agent a
    // wait plan, submit it, then let the resolver report failure (simulating
    // an unsupported operation) — the agent must replan, not wedge.
    drive_htn_system(test.world_mut());
    // Replace the submitted approach with an unsupported intent to force a
    // Failed verdict.
    test.world_mut()
        .entity_mut(agent)
        .insert(ActionIntent::UseItem {
            item: Entity::PLACEHOLDER,
        });
    test.run_system(collect_intents);
    test.run_system(resolve_intents);

    let request = test.world().get::<ActionRequestId>(agent).copied().unwrap();
    let outcome = test.world().get::<ActionOutcome>(agent).copied().unwrap();
    assert!(outcome.matches(request));
    assert_eq!(outcome.state, ActionOutcomeState::Failed);

    // Next round: the driver processes the Failed outcome and replans.
    drive_htn_system(test.world_mut());
    let state = test.world().get::<HtnAgentState>(agent).unwrap();
    assert!(
        state.processed == Some(request.0),
        "the failure was correlated and processed"
    );
    assert!(state.plan.is_none(), "failed requests drop the plan");

    // And the agent recovers: subsequent rounds still fetch the tool.
    let hammers: Vec<Entity> = {
        let world = test.world_mut();
        world
            .query_filtered::<Entity, With<DefOrigin>>()
            .iter(world)
            .collect()
    };
    let h1 = hammers[0];
    let mut carried = false;
    for _ in 0..30 {
        round(&mut test);
        carried = test
            .world()
            .get::<InsideContainer>(h1)
            .map(|c| c.0 == agent)
            .unwrap_or(false);
        if carried {
            break;
        }
    }
    assert!(
        carried,
        "agent recovers after a failure and completes the goal"
    );
}

#[test]
fn wait_plans_complete_through_the_correlation_path() {
    // NOTE: outcome processing happens on the NEXT driver call after the
    // resolver wrote the verdict (the driver runs before the collector in a
    // round), so this test drives one extra round before asserting.
    let dir = std::env::temp_dir().join(format!("htn_wait_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("htn.json"), HTN_DATA).unwrap();
    let registry = load_registry(&dir);
    let runtime = compiled_runtime(&registry);

    let mut test = TestBed::new();
    test.insert_resource(IntentQueue::default());
    test.insert_resource(ActionRequestCounter::default());
    let z = ZLevel::new(0);
    let agent = test.spawn((
        PlannerHtn,
        HtnBrain {
            root: "core:idle".into(),
            view_radius: 6,
        },
        HtnAgentState::default(),
        ActionPoints {
            current: 100,
            speed: 100,
        },
        IsAlive,
        WorldPosition(WorldPos::new(0, 0, z)),
    ));
    test.insert_resource(runtime);

    round(&mut test);

    // Wait was submitted, collected, and resolved within the round.
    let request = test.world().get::<ActionRequestId>(agent).copied().unwrap();
    let outcome = test.world().get::<ActionOutcome>(agent).copied().unwrap();
    assert!(outcome.matches(request));
    assert_eq!(outcome.state, ActionOutcomeState::Completed);

    // The next driver call processes the verdict (cursor past the end).
    drive_htn_system(test.world_mut());
    let request = test.world().get::<ActionRequestId>(agent).copied().unwrap();
    let outcome = test.world().get::<ActionOutcome>(agent).copied().unwrap();
    assert!(outcome.matches(request));
    assert_eq!(outcome.state, ActionOutcomeState::Completed);
    assert_eq!(
        test.world().get::<ActionPoints>(agent).unwrap().current,
        0,
        "wait charged its AP through the authoritative resolver"
    );
    let state = test.world().get::<HtnAgentState>(agent).unwrap();
    assert_eq!(state.processed, Some(request.0));
    assert!(state.plan.is_none(), "single-step plan completed");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn planner_drivers_preserve_submitted_activity_controls() {
    use cdda_components::{
        activity::{ActivityProgress, Waiting},
        ai::{PlannerBehaviourTree, PlannerGoap},
    };
    use cdda_sim::runtime::{step_simulation, SimulationPlugin};
    let mut loader = Loader::new(vec![]);
    loader.ingest_values(vec![(
        "control-fixture.json".into(),
        serde_json::from_str(HTN_DATA).unwrap(),
    )]);
    let registry = loader.resolve().unwrap();
    let mut app = bevy_app::App::new();
    app.add_plugins(SimulationPlugin)
        .insert_resource(compiled_runtime(&registry));
    let w = app.world_mut();
    let actors: Vec<_> = (0..3)
        .map(|i| {
            w.spawn((
                ActionPoints {
                    current: 0,
                    speed: 100,
                },
                WorldPosition(WorldPos::new(i * 10, 0, ZLevel::new(0))),
                ActivityProgress::new(200),
                Waiting { turns: 2 },
                ActionIntent::InterruptActivity,
            ))
            .id()
        })
        .collect();
    w.entity_mut(actors[0]).insert(PlannerBehaviourTree);
    w.entity_mut(actors[1]).insert(PlannerGoap);
    w.entity_mut(actors[2]).insert((
        PlannerHtn,
        HtnBrain {
            root: "core:idle".into(),
            view_radius: 10,
        },
        HtnAgentState::default(),
    ));
    step_simulation(w);
    for actor in actors {
        assert!(
            w.get::<Waiting>(actor).is_none(),
            "planner must not overwrite interruption"
        );
        assert!(w.get::<ActivityProgress>(actor).is_none());
        assert_eq!(
            w.get::<ActionOutcome>(actor).unwrap().state,
            ActionOutcomeState::Completed
        );
    }
}
