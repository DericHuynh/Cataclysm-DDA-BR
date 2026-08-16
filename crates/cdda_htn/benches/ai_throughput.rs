//! Throughput benchmark for the `cdda_htn` planner, run **through Bevy ECS**.
//!
//! Stresses the planner the way a real CDDA frame would: a population of AI
//! actors living as **real Bevy entities**, each carrying a multi-node mining
//! HTN (transcribed from the canonical `bevy_htn` miner example). A Bevy system
//! iterates the entity query every "tick" and produces a `Plan` component per
//! entity — so the benchmark measures the exact same planner-through-ECS path
//! the game would take, including query iteration and component writes.
//!
//! This is upstream of the reference `bevy_htn` example, which drives only a
//! handful of `Dude`s; here we scale to **200k** simultaneous entities.

use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;
use bevy_ecs::system::Res;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
use cdda_htn::parse_htn;
use cdda_htn::planner::HtnPlanner;
use cdda_htn::HtnDomain;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use ustr::Ustr;

/// The world map location a miner can occupy — mirrors `Location` in the
/// reference miner example.
#[derive(Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Default)]
enum Location {
    #[default]
    Outside,
    House,
    Ore,
    Smelter,
    Mushroom,
    Merchant,
}

/// The per-actor AI plan state, stored as a Bevy [`Component`].
#[derive(Reflect, Component, Clone, Debug, Default)]
struct MinerState {
    gold: i32,
    has_ore: bool,
    has_metal: bool,
    energy: i32,
    hunger: i32,
    location: Location,
}

/// The output of the planner: written back onto each entity (like the reference
/// `bevy_bae`/`bevy_htn` `Plan` component). Carries just the (interned) task
/// names so the benchmark forces a real component write while exercising the
/// `ustr`-based plan path.
#[derive(Component, Debug, Default)]
struct Plan(Vec<Ustr>);

/// Running count of actors planned so far, written by [`run_ai`]. Using a single
/// atomics resource keeps the `par_iter_mut` closure free of commands; the
/// planner work per entity dominates the (rare) cross-thread contention on this
/// counter, so a thread-local accumulation pass is not worth its setup cost.
#[derive(Resource, Default)]
struct AiProcessed(std::sync::atomic::AtomicUsize);

/// Immutable domain+registry shared by every AI system run. Both derive
/// `Resource` and are registered once.
#[derive(Resource)]
struct HtnResources {
    domain: HtnDomain,
    registry: TypeRegistry,
}

impl HtnResources {
    fn new() -> Self {
        let mut registry = TypeRegistry::default();
        registry.register::<MinerState>();
        registry.register::<Location>();
        let domain = parse_htn(MINER_HTN).expect("parse miner HTN");
        Self { domain, registry }
    }
}

/// A multi-node mining HTN, same shape as `bevy_htn`'s `assets/miner.htn`:
/// a recursive root compound task with 4 methods, two nested compound tasks
/// (ore→metal, metal→gold) and 7 primitives with preconditions/effects.
const MINER_HTN: &str = r#"
schema {
    version: 0.1.0
}

compound_task "EarnGold" {
    method "Got enough gold" {
        preconditions: [gold >= 3]
        subtasks: []
    }
    method "Convert metal to gold" {
        subtasks: [TurnMetalIntoGold, EarnGold]
    }
    method "Convert ore to gold" {
        subtasks: [TurnOreIntoMetal, EarnGold]
    }
    method "Get some ore" {
        preconditions: [has_ore == false]
        subtasks: [GoToOre, MineOre, EarnGold]
    }
}

compound_task "TurnOreIntoMetal" {
    method "Get to smelter" {
        preconditions: [has_ore == true, location != Location::Smelter]
        subtasks: [GoToSmelter, TurnOreIntoMetal]
    }
    method "Smelt" {
        preconditions: [has_ore == true, location == Location::Smelter]
        subtasks: [SmeltOre, GoToOutside]
    }
}

compound_task "TurnMetalIntoGold" {
    method "Get to merchant" {
        preconditions: [has_metal == true, location != Location::Merchant]
        subtasks: [GoToMerchant, TurnMetalIntoGold]
    }
    method "Sell" {
        preconditions: [has_metal == true, location == Location::Merchant]
        subtasks: [SellMetal, GoToOutside]
    }
}

primitive_task "GoToOre" {
    effects: [location = Location::Ore]
    operator: GoToOreOperator
}
primitive_task "MineOre" {
    preconditions: [energy > 10, hunger < 75, location == Location::Ore]
    effects: [has_ore = true, location = Location::Outside]
    operator: MineOreOperator
}
primitive_task "GoToSmelter" {
    effects: [location = Location::Smelter]
    operator: GoToSmelterOperator
}
primitive_task "SmeltOre" {
    preconditions: [energy > 10, hunger < 75, location == Location::Smelter, has_ore == true]
    effects: [has_ore = false, has_metal = true]
    operator: SmeltOreOperator
}
primitive_task "GoToMerchant" {
    effects: [location = Location::Merchant]
    operator: GoToMerchantOperator
}
primitive_task "SellMetal" {
    preconditions: [energy > 10, hunger < 75, location == Location::Merchant, has_metal == true]
    effects: [gold += 1, has_metal = false]
    operator: SellMetalOperator
}
primitive_task "GoToOutside" {
    effects: [location = Location::Outside]
    operator: GoToOutsideOperator
}
"#;

/// The AI system: for every miner entity, plan the root task and write the
/// result into its `Plan` component.
///
/// Runs the query in **parallel** via [`Query::par_iter_mut`], the per-frame AI
/// cost. Each closure builds its own [`HtnPlanner`] (an immutable
/// `domain + registry` view, no shared mutable state), so the population plans
/// concurrently; the `par_iter_mut` batch is scheduled across the
/// multi-threaded `ComputeTaskPool`.
fn run_ai(
    resources: Res<HtnResources>,
    mut q: Query<(&MinerState, &mut Plan)>,
    processed: Res<AiProcessed>,
) {
    processed.0.store(0, std::sync::atomic::Ordering::Relaxed);
    q.par_iter_mut().for_each(|(state, mut plan)| {
        let mut planner = HtnPlanner::new(&resources.domain, &resources.registry);
        let planned = planner.plan("EarnGold", state);
        plan.0 = planned.task_names().to_vec();
        processed
            .0
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
}

/// Spawn `n` miner entities with varied state, then return a `(world, schedule)`
/// ready to run the AI system — mirroring how a production Bevy app runs it via
/// the multi-threaded [`Schedule`] executor (which initializes the
/// `ComputeTaskPool`).
fn spawn_world(n: usize) -> (World, Schedule) {
    let mut res = World::new();
    res.insert_resource(HtnResources::new());
    res.insert_resource(AiProcessed::default());
    res.spawn_batch((0..n).map(|i| {
        (
            MinerState {
                gold: (i % 5) as i32,
                has_ore: i % 3 == 0,
                has_metal: i % 7 == 0,
                energy: 80 - (i % 40) as i32,
                hunger: 20 + (i % 60) as i32,
                location: Location::Outside,
            },
            // Pre-insert the output component so the AI system can write into it
            // directly in parallel (steady-state: every actor already has a plan).
            Plan(Vec::new()),
        )
    }))
    .count();

    let mut schedule = Schedule::default();
    schedule.add_systems(run_ai);
    (res, schedule)
}

pub fn miner_planner(c: &mut Criterion) {
    let single_state = MinerState::default();

    let mut group = c.benchmark_group("cdda_htn_bevy_ecs");

    // Full-frame throughput: run the AI system over the whole entity population
    // through a Bevy `Schedule`, exactly as production does.
    for n in [10_000usize, 50_000, 200_000] {
        let (mut world, mut schedule) = spawn_world(n);
        group.throughput(criterion::Throughput::Elements(n as u64));
        group.bench_function(format!("frame_{n}_miner_entities"), |b| {
            b.iter(|| {
                schedule.run(&mut world);
                let processed = world
                    .resource::<AiProcessed>()
                    .0
                    .load(std::sync::atomic::Ordering::Relaxed);
                warn_if_processed(processed, n);
            });
        });
    }

    // Single-actor latency done straight through the planner (no ECS overhead).
    let resources = HtnResources::new();
    group.bench_function("plan_one_actor_latency", |b| {
        b.iter(|| {
            let mut planner = HtnPlanner::new(&resources.domain, &resources.registry);
            black_box(planner.plan("EarnGold", &single_state));
        });
    });

    group.finish();
}

#[track_caller]
fn warn_if_processed(actual: usize, expected: usize) {
    assert_eq!(actual, expected, "AI system missed entities");
}

criterion_group!(benches, miner_planner);
criterion_main!(benches);
