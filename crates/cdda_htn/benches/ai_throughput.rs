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
/// `bevy_bae`/`bevy_htn` `Plan` component). Carries just the task names so the
/// benchmark forces a real component write.
#[derive(Component, Debug, Default)]
struct Plan(Vec<String>);

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

/// The AI system: for every miner entity, plan the root task and store the
/// result as a `Plan` component. This is the per-frame AI cost.
fn run_ai(
    resources: Res<HtnResources>,
    mut q: Query<(Entity, &MinerState, Option<&mut Plan>)>,
    mut commands: Commands,
) -> usize {
    let mut processed = 0;
    for (entity, state, plan) in q.iter_mut() {
        processed += 1;
        let mut planner = HtnPlanner::new(&resources.domain, &resources.registry);
        let planned = planner.plan("EarnGold", state);
        let names = planned.task_names().to_vec();
        if let Some(mut p) = plan {
            p.0 = names;
        } else {
            commands.entity(entity).insert(Plan(names));
        }
    }
    processed
}

/// Spawn `n` miner entities with varied state, then return the world.
fn spawn_world(n: usize) -> World {
    let mut res = World::new();
    res.insert_resource(HtnResources::new());
    res.spawn_batch((0..n).map(|i| MinerState {
        gold: (i % 5) as i32,
        has_ore: i % 3 == 0,
        has_metal: i % 7 == 0,
        energy: 80 - (i % 40) as i32,
        hunger: 20 + (i % 60) as i32,
        location: Location::Outside,
    }))
    .count();
    res
}

pub fn miner_planner(c: &mut Criterion) {
    let single_state = MinerState::default();

    let mut group = c.benchmark_group("cdda_htn_bevy_ecs");

    // Full-frame throughput: run the AI system over the whole entity population.
    for n in [10_000usize, 50_000, 200_000] {
        let mut world = spawn_world(n);
        let ai = world.register_system(run_ai);
        group.throughput(criterion::Throughput::Elements(n as u64));
        group.bench_function(format!("frame_{n}_miner_entities"), |b| {
            b.iter(|| {
                black_box(world.run_system(ai).expect("run AI system"));
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

criterion_group!(benches, miner_planner);
criterion_main!(benches);
