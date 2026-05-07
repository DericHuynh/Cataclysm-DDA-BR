//! Vision system — line of sight, visibility checks, and sight events.
//!
//! Calculates which entities are visible to which observers each turn.
//! Powers AI sight detection and player field-of-view rendering.
//! Uses the `EntitySpatialIndex` for efficient spatial proximity queries
//! and terrain opacity data from `WorldMap` for line-of-sight blocking.

use bevy_ecs::prelude::*;
use cdda_actor::components::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Update visibility state for all creatures this turn.
///
/// For each creature: determine vision range, compute set of visible
/// entities, emit `SightEvent` for newly seen entities.
pub fn update_vision(world: &mut World) {
    let _ = world;
}

/// Calculate the effective vision range for a creature.
///
/// Factors: base day/night range from `Vision`, current light level,
/// time of day, night vision mutations, and weather conditions.
pub fn calculate_vision_range(
    creature_vision: &Vision,
    time_of_day: &str,
    light_level: u32,
    has_night_vision: bool,
) -> i32 {
    let _ = (creature_vision, time_of_day, light_level, has_night_vision);
    todo!("vision range formula: day/night base × light level multiplier")
}

/// Check whether `observer` can see `target`.
///
/// Performs a line-of-sight check (Bresenham / ray casting) against
/// terrain opacity and checks that target is within vision range.
pub fn can_see(world: &World, observer: Entity, target: Entity) -> bool {
    let _ = (world, observer, target);
    todo!("line of sight + range check: query WorldPosition, trace through terrain")
}

/// Return all entities currently visible to `observer`.
///
/// Uses the `EntitySpatialIndex` for radius queries, then filters
/// by line-of-sight and vision range.
pub fn visible_entities(world: &World, observer: Entity) -> Vec<Entity> {
    let _ = (world, observer);
    todo!("all entities observer can see: spatial radius query + LOS filter")
}
