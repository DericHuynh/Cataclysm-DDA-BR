//! State hashing and divergence detection (devtools only).
//!
//! The hash covers MEANINGFUL GAMEPLAY STATE, not just entity membership:
//! per entity the stable `SimId` (Entity bits only as a test fallback),
//! world position, AP, health, stack count, plus the stable id of its
//! containment/wield/wear owner. Definition entities (`IsDef`) are
//! excluded — they are data, not simulation state. Rows are sorted by
//! stable id, so spawn order does not change the digest.
//!
//! Recording vs replay: the live hash history (`StateHashLog`) is always
//! maintained, but the EXPECTED log (`SessionLog.state_hashes`) is only
//! appended in recording mode (no `ReplayState` resource). A replay never
//! mutates the log it is being compared against.

use crate::replay::ReplayState;
use crate::session_log::SessionLog;
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use cdda_components::actor::{ActionPoints, Health};
use cdda_components::def::IsDef;
use cdda_components::item::{InsideContainer, StackCount, WieldedBy, WornOn};
use cdda_components::sim::{GameTime, WorldPosition};
use cdda_core_types::sim_id::SimId;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Resource, Default, Debug, Clone)]
pub struct StateHashLog {
    pub hashes: Vec<(u64, u64)>, // (turn, hash)
}

/// Digest one world's gameplay state. Returns (turn, digest).
///
/// Entities without a `SimId` fall back to `Entity::to_bits()`, which is only
/// stable within one world — production entities must carry SimIds for
/// cross-run determinism (the replay contract requires unique SimIds).
pub fn compute_state_hash(world: &mut World) -> (u64, u64) {
    let turn = world.resource::<GameTime>().turn;

    // Pass 1: stable identity per non-definition entity.
    let mut id_of: HashMap<Entity, u64> = HashMap::new();
    {
        let mut q = world.query_filtered::<(Entity, Option<&SimId>), Without<IsDef>>();
        for (entity, id) in q.iter(world) {
            let sid = id.map(|id| id.0).unwrap_or_else(|| entity.to_bits());
            id_of.insert(entity, sid);
        }
    }

    // Pass 2: per-entity value state.
    let mut rows: Vec<(u64, u64)> = Vec::with_capacity(id_of.len());
    {
        let mut q = world.query_filtered::<(
            Entity,
            Option<&WorldPosition>,
            Option<&ActionPoints>,
            Option<&Health>,
            Option<&StackCount>,
        ), Without<IsDef>>();
        for (entity, pos, ap, hp, stack) in q.iter(world) {
            let mut h = DefaultHasher::new();
            if let Some(pos) = pos {
                let pos = pos.get();
                pos.x.hash(&mut h);
                pos.y.hash(&mut h);
                pos.z.0.hash(&mut h);
            }
            ap.map(|ap| ap.current).hash(&mut h);
            if let Some(hp) = hp {
                (hp.current, hp.max).hash(&mut h);
            }
            stack.map(|s| s.get()).hash(&mut h);
            rows.push((id_of[&entity], h.finish()));
        }
    }

    // Pass 3: ownership edges (containment / wield / wear) by stable ids.
    let mut edges: Vec<(u64, u64)> = Vec::new();
    {
        let mut contained = world.query_filtered::<(Entity, &InsideContainer), Without<IsDef>>();
        for (item, inside) in contained.iter(world) {
            if let Some(owner) = id_of.get(&inside.0) {
                edges.push((id_of[&item], *owner));
            }
        }
        let mut wielded = world.query_filtered::<(Entity, &WieldedBy), Without<IsDef>>();
        for (item, wielded_by) in wielded.iter(world) {
            if let Some(owner) = id_of.get(&wielded_by.0) {
                edges.push((id_of[&item], *owner));
            }
        }
        let mut worn = world.query_filtered::<(Entity, &WornOn), Without<IsDef>>();
        for (item, worn_on) in worn.iter(world) {
            if let Some(owner) = id_of.get(&worn_on.wearer) {
                edges.push((id_of[&item], *owner));
            }
        }
    }

    rows.sort_unstable();
    edges.sort_unstable();
    let mut h = DefaultHasher::new();
    turn.hash(&mut h);
    rows.len().hash(&mut h);
    for row in &rows {
        row.0.hash(&mut h);
        row.1.hash(&mut h);
    }
    edges.len().hash(&mut h);
    for edge in &edges {
        edge.0.hash(&mut h);
        edge.1.hash(&mut h);
    }
    (turn, h.finish())
}

/// Runs after the logical simulation (GameSet::Render in the app wiring):
/// the digest always reflects COMMITTED state, never mid-turn scratch.
pub fn hash_simulation_state(world: &mut World) {
    if cfg!(not(feature = "devtools")) {
        return;
    }

    #[cfg(feature = "devtools")]
    {
        let (turn, digest) = compute_state_hash(world);
        world
            .resource_mut::<StateHashLog>()
            .hashes
            .push((turn, digest));
        // The expected log is immutable during replay: only recording appends.
        if world.get_resource::<ReplayState>().is_none() {
            world
                .resource_mut::<SessionLog>()
                .state_hashes
                .push((turn, digest));
        }
    }
}

/// During replay, compares the live hash for the current turn against the
/// recorded (immutable) log and fires `SimulationDiverged` on mismatch.
pub fn check_divergence(
    hash_log: Res<StateHashLog>,
    session_log: Res<SessionLog>,
    game_time: Res<GameTime>,
    mut divergence_writer: bevy_ecs::message::MessageWriter<SimulationDiverged>,
) {
    let turn = game_time.turn;
    let live = hash_log.hashes.iter().rev().find(|(t, _)| *t == turn);
    let recorded = session_log.state_hashes.iter().find(|(t, _)| *t == turn);
    if let (Some((_, live_hash)), Some((_, recorded_hash))) = (live, recorded) {
        if live_hash != recorded_hash {
            divergence_writer.write(SimulationDiverged {
                turn,
                detail: format!(
                    "State hash mismatch at turn {turn}: live={live_hash:x}, recorded={recorded_hash:x}"
                ),
            });
        }
    }
}

#[derive(Message, Debug, Clone)]
pub struct SimulationDiverged {
    pub turn: u64,
    pub detail: String,
}
