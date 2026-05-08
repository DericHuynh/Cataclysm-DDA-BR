//! # Turn scheduling — Action Point system
//!
//! Reference: Section 8 — The Turn & Action Point System
//!
//! Every actor has:
//! - `Speed` (base 100) — MovePoints gained per turn
//! - `MovePoints` — current action point pool
//! - `TurnQueue` — priority queue sorted by highest MP first
//!
//! ## Per-turn flow
//! 1. `tick_move_points`: all actors gain `Speed` MP (clamped to -2×Speed debt floor)
//! 2. Actors act in MP order (highest first)
//! 3. Each action costs MP (walk=100, attack=100, etc.)
//! 4. When all actors below threshold, next turn begins

use crate::core::components::def::IsDef;
use bevy_ecs::prelude::*;
use crate::core::components::actor::{IsAlive, MovePoints, Speed};
use std::cmp::Ordering;

// ---------------------------------------------------------------------------
// Action cost constants
// ---------------------------------------------------------------------------

pub const MOVE_COST_WALK: i32 = 100;
pub const MOVE_COST_RUN: i32 = 80;
pub const MOVE_COST_CROUCH: i32 = 150;
pub const MOVE_COST_ATTACK_BASE: i32 = 100;
pub const MOVE_COST_PICKUP: i32 = 100;
pub const MOVE_COST_RELOAD_BASE: i32 = 100;
pub const MP_MIN_FLOOR: i32 = 25; // below this, actor stops acting

// ---------------------------------------------------------------------------
// TurnQueue resource
// ---------------------------------------------------------------------------

/// An actor's slot in the turn queue, sorted by move points descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorTurn {
    pub move_points: i32,
    pub entity: Entity,
}

impl Ord for ActorTurn {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse: highest MP first
        self.move_points.cmp(&other.move_points)
    }
}

impl PartialOrd for ActorTurn {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority queue of actors for the current turn, sorted by MP descending.
#[derive(Resource, Debug, Clone)]
pub struct TurnQueue {
    /// Actor slots, rebuilt each turn from all living entities with Speed+MovePoints.
    pub actors: Vec<ActorTurn>,
    /// Global turn counter.
    pub turn_count: u64,
}

impl Default for TurnQueue {
    fn default() -> Self {
        Self {
            actors: Vec::new(),
            turn_count: 0,
        }
    }
}

impl TurnQueue {
    /// Pop the actor with the highest move points.
    pub fn pop_highest(&mut self) -> Option<ActorTurn> {
        // Find max by linear scan — the vec is short (dozens, not thousands).
        // A BinaryHeap would be faster but requires draining per-turn anyway.
        let best_idx = self
            .actors
            .iter()
            .enumerate()
            .max_by_key(|(_, a)| a.move_points)
            .map(|(i, _)| i)?;

        Some(self.actors.swap_remove(best_idx))
    }

    /// Peek at the highest MP value without popping.
    pub fn highest_mp(&self) -> i32 {
        self.actors.iter().map(|a| a.move_points).max().unwrap_or(0)
    }

    /// Check if any actor still has enough MP to act.
    pub fn has_actors_ready(&self) -> bool {
        self.highest_mp() >= MP_MIN_FLOOR
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Phase 0: Grant move points to all actors and rebuild the turn queue.
///
/// Runs at the start of each game turn.
///
/// Reference: Section 8 — "All actors gain Speed move points"
pub fn tick_move_points(
    mut query: Query<(Entity, &mut MovePoints, &Speed), (With<IsAlive>, Without<IsDef>)>,
    mut queue: ResMut<TurnQueue>,
    mut game_time: ResMut<crate::sim::state::GameTime>,
) {
    // Clear old queue
    queue.actors.clear();

    for (entity, mut mp, speed) in &mut query {
        // Grant MP, enforce debt floor: cannot exceed 2 turns of debt
        let debt_floor = -(speed.0 * 2).max(50); // at least -50
        mp.0 = (mp.0 + speed.0).max(debt_floor);

        // Add to queue
        queue.actors.push(ActorTurn {
            move_points: mp.0,
            entity,
        });
    }

    // Sort descending by MP (highest first)
    queue
        .actors
        .sort_by(|a, b| b.move_points.cmp(&a.move_points));

    // Advance global turn counter
    queue.turn_count += 1;
    game_time.advance();
}

/// Spend move points for an entity.
///
/// Systems call this after resolving an action (movement, attack, etc.).
/// Returns `true` if the entity can still act this turn (MP >= MP_MIN_FLOOR).
///
/// Reference: Section 8 — "Each action costs move points"
pub fn spend_move_points(entity: Entity, cost: i32, query: &mut Query<&mut MovePoints>) -> bool {
    if let Ok(mut mp) = query.get_mut(entity) {
        mp.0 -= cost;
        mp.0 >= MP_MIN_FLOOR
    } else {
        false
    }
}

/// Apply terrain movement cost multiplier.
///
/// CDDA terrain has a `movecost` field (default 100 = normal floor).
/// An actor's base move cost is multiplied by (terrain_cost / 100).
///
/// Reference: Section 8 — Terrain movement cost multiplier
pub fn effective_move_cost(base_cost: i32, terrain_cost: i32) -> i32 {
    if terrain_cost <= 0 {
        return i32::MAX; // impassable
    }
    (base_cost * terrain_cost) / 100
}

/// Debug/info system: logs turn queue state (useful for testing).
pub fn debug_turn_queue(queue: Res<TurnQueue>) {
    if queue.turn_count > 0 && queue.turn_count % 10 == 0 {
        tracing::info!(
            "Turn {}: {} actors in queue, highest MP = {}",
            queue.turn_count,
            queue.actors.len(),
            queue.highest_mp()
        );
    }
}

