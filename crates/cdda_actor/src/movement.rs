//! Movement phase — resolve movement intents.
//!
//! Serial movement resolution: entities with movement intents are moved
//! sequentially to prevent collision conflicts. Each movement costs
//! `MovePoints` based on terrain, furniture, and creature state.

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::WorldPos;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Outcome of a movement attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveResult {
    /// Movement succeeded; remaining move points returned.
    Success { cost: i32, remaining_mp: i32 },
    /// Movement was blocked by an obstacle.
    Blocked { reason: MoveBlockReason },
    /// Entity does not have enough move points for this action.
    InsufficientMP { needed: i32, available: i32 },
}

/// Reasons a movement attempt can be blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveBlockReason {
    /// Target tile has terrain with movecost <= 0 (e.g. deep water, wall).
    ImpassableTerrain,
    /// Target tile is occupied by an entity with `Solid` component.
    OccupiedBySolid,
    /// Target position is outside the generated world bounds.
    OutOfBounds,
    /// No path to target (pathfinding returned empty).
    NoPath,
}

// ---------------------------------------------------------------------------
// Formulas
// ---------------------------------------------------------------------------

/// Calculate the total move point cost for a single step.
pub fn calculate_move_cost(
    terrain_cost: i32,
    furniture_mod: i32,
    creature_speed: i32,
    is_swimming: bool,
    is_prone: bool,
    bleeding: bool,
) -> i32 {
    let _ = (
        terrain_cost,
        furniture_mod,
        creature_speed,
        is_swimming,
        is_prone,
        bleeding,
    );
    todo!("move cost formula: base = (100 * terrain_cost / 100) + furniture_mod, modified by swimming/prone/bleeding")
}

// ---------------------------------------------------------------------------
// Core operations
// ---------------------------------------------------------------------------

/// Attempt to move an entity by (dx, dy, dz).
///
/// `dz ∈ {-1, 0, 1}` — z-level transitions require a valid ramp/stairs/ladder at
/// the current position; raw `dz` without a transition tile returns `Blocked::ImpassableTerrain`.
///
/// Checks passability at the target position, deducts move points, updates `WorldPosition`.
pub fn attempt_move(world: &mut World, entity: Entity, dx: i32, dy: i32, dz: i8) -> MoveResult {
    let _ = (world, entity, dx, dy, dz);
    todo!("movement attempt resolution: if dz != 0 check transition tile => check passability at target => calc cost => spend MP => update WorldPosition")
}

/// Spend move points for an entity. Returns remaining MP after deduction.
pub fn spend_move_points(world: &mut World, entity: Entity, amount: i32) -> i32 {
    let _ = (world, entity, amount);
    todo!("spend MP: query &mut MovePoints, subtract amount, return remaining")
}

/// Check whether a target position is passable for a given entity.
/// Considers terrain movecost, furniture, and solid entities.
pub fn is_passable(world: &World, entity: Entity, position: WorldPos) -> bool {
    // STUB: passability always returns true until terrain queries are wired
    let _ = (world, entity, position);
    true
}

// ---------------------------------------------------------------------------
// Phase orchestrator
// ---------------------------------------------------------------------------

/// Serial movement resolution.
///
/// Entities with movement intents are moved sequentially
/// to prevent collision conflicts.
pub fn movement_phase(world: &mut World) {
    let _ = world;
}
