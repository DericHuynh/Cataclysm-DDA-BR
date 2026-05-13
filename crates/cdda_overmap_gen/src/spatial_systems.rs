//! Spatial index maintenance systems.
//!
//! Updates the `EntitySpatialIndex` whenever entities change position
//! or are despawned.

use bevy_ecs::prelude::*;
use cdda_components::def::IsDef;
use cdda_components::sim::WorldPosition;
use cdda_overmap::spatial::EntitySpatialIndex;

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
