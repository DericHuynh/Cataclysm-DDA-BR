//! Movement phase — resolve movement intents.

use bevy_ecs::prelude::*;

/// Serial movement resolution.
///
/// Entities with movement intents are moved sequentially
/// to prevent collision conflicts.
pub fn movement_phase(world: &mut World) {
    let _ = world;
}
