//! Spatial index maintenance system.
//!
//! Updates the EntitySpatialIndex whenever entities change position.

use crate::components::WorldPosition;
use crate::def_components::IsDef;
use crate::spatial::EntitySpatialIndex;
use bevy_ecs::prelude::*;

/// Update the spatial index for all entities whose position changed this tick.
pub fn update_spatial_index(
    mut spatial: ResMut<EntitySpatialIndex>,
    query: Query<(Entity, &WorldPosition), (Changed<WorldPosition>, Without<IsDef>)>,
) {
    for (entity, pos) in &query {
        spatial.update_position(entity, pos.0);
    }
}

/// Remove despawned entities from the spatial index.
pub fn cleanup_spatial_index(
    mut spatial: ResMut<EntitySpatialIndex>,
    mut removals: RemovedComponents<WorldPosition>,
) {
    for entity in removals.read() {
        spatial.remove_entity(entity);
    }
}
