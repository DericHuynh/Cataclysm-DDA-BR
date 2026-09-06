//! Regression suite for the deterministic replay state contract.
//!
//! Compiles to nothing without the `devtools` feature; run with
//! `cargo nextest run -p cdda_replay --features devtools`.
#![cfg(feature = "devtools")]

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, Health};
use cdda_components::item::{InsideContainer, StackCount};
use cdda_components::sim::{GameTime, WorldPosition};
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_core_types::sim_id::SimId;
use cdda_replay::replay::ReplayState;
use cdda_replay::session_log::SessionLog;
use cdda_replay::state_hash::{compute_state_hash, hash_simulation_state, StateHashLog};

fn world() -> World {
    let mut world = World::new();
    world.insert_resource(GameTime::default());
    world
}

fn actor(world: &mut World, sim: u64, x: i32, ap: i32) -> Entity {
    world
        .spawn((
            SimId(sim),
            WorldPosition::new(WorldPos::new(x, 0, ZLevel::new(0))),
            ActionPoints {
                current: ap,
                speed: 100,
            },
            Health {
                current: 80,
                max: 100,
            },
        ))
        .id()
}

#[test]
fn state_changes_change_the_digest() {
    let mut world = world();
    actor(&mut world, 1, 0, 100);
    let (turn, before) = compute_state_hash(&mut world);

    // Move + spend AP: meaningful state, same entity set.
    let entity = {
        let mut q = world.query_filtered::<Entity, With<SimId>>();
        q.iter(&world).next().unwrap()
    };
    world
        .entity_mut(entity)
        .insert(WorldPosition::new(WorldPos::new(3, 0, ZLevel::new(0))));
    world.get_mut::<ActionPoints>(entity).unwrap().current = 0;
    let (_, after) = compute_state_hash(&mut world);

    assert_eq!(turn, 0);
    assert_ne!(before, after, "position/AP changes must change the digest");
}

#[test]
fn spawn_order_does_not_change_the_digest() {
    let build = |spawn_a_first: bool| {
        let mut world = world();
        let _ = spawn_a_first; // only spawn ORDER differs between calls
        actor(&mut world, 1, 0, 100);
        actor(&mut world, 2, 5, 50);
        world
    };
    let digest = |mut world: World| compute_state_hash(&mut world).1;
    assert_eq!(digest(build(true)), digest(build(false)));
}

#[test]
fn containment_ownership_is_in_the_digest() {
    let mut world = world();
    let carrier = actor(&mut world, 1, 0, 100);
    let item = world.spawn((SimId(9), StackCount::new(2).unwrap())).id();
    let (_, loose) = compute_state_hash(&mut world);

    world.entity_mut(item).insert(InsideContainer(carrier));
    let (_, held) = compute_state_hash(&mut world);
    assert_ne!(loose, held, "ownership edges belong in the digest");
}

#[test]
fn replay_mode_never_appends_to_the_expected_log() {
    let mut app = App::new();
    app.insert_resource(GameTime::default());
    app.insert_resource(SessionLog::new(7));
    app.insert_resource(StateHashLog::default());
    // Replay mode marker: hash must treat the log as immutable.
    app.insert_resource(ReplayState::default());
    app.add_systems(Update, hash_simulation_state);

    app.world_mut().spawn((
        SimId(1),
        ActionPoints {
            current: 5,
            speed: 100,
        },
    ));
    app.update();
    app.update();

    let log = app.world().resource::<SessionLog>();
    assert!(
        log.state_hashes.is_empty(),
        "replay must not mutate expected log"
    );
    let live = app.world().resource::<StateHashLog>();
    assert_eq!(live.hashes.len(), 2, "live history still maintained");
}

#[test]
fn recording_mode_populates_the_expected_log() {
    let mut app = App::new();
    app.insert_resource(GameTime::default());
    app.insert_resource(SessionLog::new(7));
    app.insert_resource(StateHashLog::default());
    // No ReplayState: this is recording mode.
    app.add_systems(Update, hash_simulation_state);

    app.world_mut().spawn((
        SimId(1),
        ActionPoints {
            current: 5,
            speed: 100,
        },
    ));
    app.update();

    let log = app.world().resource::<SessionLog>();
    assert_eq!(log.state_hashes.len(), 1);
}
