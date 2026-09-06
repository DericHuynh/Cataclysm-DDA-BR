//! # Turn scheduling — Action Point system
//!
//! Every actor has a single `ActionPoints` component (`current` + `speed`).
//! Each turn `speed` AP is granted; actions deduct from `current`.
//!
//! ## Per-turn flow
//! 1. `tick_move_points`: all actors gain `speed` AP (clamped to -2×speed debt floor)
//! 2. The runtime selects actors by live AP; turn-based player input runs first.
//! 3. Each action costs AP; craft ticks consume all available AP.
//! 4. Turn-based input reuses positive player AP before another world tick.

use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::def::IsDef;
use cdda_components::sim::{GameTime, TurnAdvanced};
use std::cmp::Ordering;

// ---------------------------------------------------------------------------
// Action cost constants
// ---------------------------------------------------------------------------

pub const MOVE_COST_WALK: i32 = 100;
pub const MOVE_COST_RUN: i32 = 80;
/// Crouching doubles the base walk cost (CDDA: `move_mode_crouch` = 2× walk).
pub const MOVE_COST_CROUCH: i32 = 200;
/// Prone (crawling) costs 6× walk (CDDA: `move_mode_prone` = 6× walk).
pub const MOVE_COST_PRONE: i32 = 600;
/// Being knocked down triples the movement cost (CDDA: `effect_downed` = 3×).
pub const MOVE_COST_DOWNED_MULTIPLIER: i32 = 3;
pub const MOVE_COST_ATTACK_BASE: i32 = 100;
pub const AP_COST_PICKUP: i32 = 100;
pub const AP_COST_WIELD: i32 = 100;
/// Legacy 100-move work unit. Actual crafting ticks consume the actor's entire
/// available budget; this constant is not a per-tick cost or cap.
pub const AP_COST_CRAFT_TICK: i32 = 100;
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

/// Phase 0: Grant AP to all living actors and rebuild the turn queue.
///
/// Runs at the start of each game turn (throttled in the app to ≤10 turns/sec).
pub fn tick_move_points(
    mut query: Query<(Entity, &mut ActionPoints), (With<IsAlive>, Without<IsDef>)>,
    mut queue: ResMut<TurnQueue>,
    mut game_time: ResMut<GameTime>,
    mut turn_writer: MessageWriter<TurnAdvanced>,
) {
    queue.actors.clear();

    for (entity, mut ap) in &mut query {
        ap.tick();
        queue.actors.push(ActorTurn {
            move_points: ap.current,
            entity,
        });
    }

    queue
        .actors
        .sort_by(|a, b| b.move_points.cmp(&a.move_points));

    queue.turn_count += 1;
    game_time.advance();

    // Emit TurnAdvanced so time-based systems can react.
    turn_writer.write(TurnAdvanced {
        turn: game_time.turn,
    });
}

/// Spend AP for an entity.
///
/// Returns `true` if the entity can still act this turn (`current >= MP_MIN_FLOOR`).
pub fn spend_move_points(entity: Entity, cost: i32, query: &mut Query<&mut ActionPoints>) -> bool {
    if let Ok(mut ap) = query.get_mut(entity) {
        ap.spend(cost);
        ap.current >= MP_MIN_FLOOR
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
