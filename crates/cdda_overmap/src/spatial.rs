//! 3D spatial index for dynamic entities — grid-based spatial partitioning.
//!
//! Maps `WorldPos` → `Set<Entity>` for fast radius queries. Kept as an
//! independent `Resource` rather than being stored on `OvermapChunk` entities
//! for two reasons:
//!
//! 1. Dynamic entities (NPCs, vehicles, items) move continuously and can
//!    straddle chunk boundaries. Coupling their storage to chunk components
//!    means every move potentially touches two `OvermapChunk` components and
//!    fires `Changed<OvermapChunk>`, which pollutes generation change detection.
//!
//! 2. Proximity queries ("what entities are near this position?") are
//!    independent of chunk alignment. The spatial grid uses its own cell size
//!    optimized for entity density, not the 30-OMT chunk partitioning.
//!
//! # Sync strategy
//!
//! - **Position changes**: `sync_spatial_index` runs in `PostUpdate`, querying
//!   `Changed<WorldPos>` to update any entity that moved this frame.
//! - **Despawns**: an `OnRemove<WorldPos>` observer removes the entity
//!   immediately when its position component is dropped.
//!
//! Register both in your plugin:
//! ```rust,ignore
//! app.add_systems(PostUpdate, sync_spatial_index)
//!    .add_observer(remove_from_spatial_index);
//! ```

use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::Remove;
use bevy_ecs::prelude::*;
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::WorldPos;
use std::collections::HashMap;

/// Cell size in world tiles. Power of two for fast division.
const CELL_SIZE: i32 = 16;

/// Z granularity: one cell per z-level.
const Z_CELL_SIZE: i32 = 1;

/// Spatial index mapping grid cells to entity sets.
///
/// Each cell is identified by `(cell_x, cell_y, cell_z)` where
/// `cell_x = world_x.div_euclid(CELL_SIZE)`, etc.
#[derive(Debug, Clone, Default, Resource)]
pub struct EntitySpatialIndex {
    cells: HashMap<(i32, i32, i32), Vec<Entity>>,
    /// Reverse map: which cell each entity is currently in.
    /// Required for O(1) removal on position change.
    entity_cells: HashMap<Entity, (i32, i32, i32)>,
}

impl EntitySpatialIndex {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    #[inline]
    fn cell(pos: WorldPos) -> (i32, i32, i32) {
        (
            pos.x.div_euclid(CELL_SIZE),
            pos.y.div_euclid(CELL_SIZE),
            pos.z.0 as i32 / Z_CELL_SIZE,
        )
    }

    /// Update an entity's position in the index.
    ///
    /// No-ops if the entity hasn't changed cells since the last call.
    /// Called by `sync_spatial_index` for every `Changed<WorldPos>` entity.
    pub fn update_position(&mut self, entity: Entity, pos: WorldPos) {
        let new_cell = Self::cell(pos);
        if let Some(&old_cell) = self.entity_cells.get(&entity) {
            if old_cell == new_cell {
                return;
            }
            if let Some(cell_entities) = self.cells.get_mut(&old_cell) {
                cell_entities.retain(|e| *e != entity);
            }
        }
        self.cells.entry(new_cell).or_default().push(entity);
        self.entity_cells.insert(entity, new_cell);
    }

    /// Remove an entity from the index entirely.
    ///
    /// Called by the `OnRemove<WorldPos>` observer.
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(cell) = self.entity_cells.remove(&entity) {
            if let Some(cell_entities) = self.cells.get_mut(&cell) {
                cell_entities.retain(|e| *e != entity);
            }
        }
    }

    /// Return all entities within `radius` world tiles (3D Chebyshev cell search).
    pub fn query_radius(&self, center: WorldPos, radius: f64) -> Vec<Entity> {
        let center_cell = Self::cell(center);
        let cell_radius = (radius / CELL_SIZE as f64).ceil() as i32 + 1;
        let z_radius = cell_radius;

        let mut result = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -z_radius..=z_radius {
                    let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2 + dz);
                    if let Some(ents) = self.cells.get(&cell) {
                        result.extend_from_slice(ents);
                    }
                }
            }
        }
        result
    }

    /// Return all entities within `radius` world tiles on the same z-level (2D search).
    pub fn query_radius_2d(&self, center: WorldPos, radius: f64) -> Vec<Entity> {
        let center_cell = Self::cell(center);
        let cell_radius = (radius / CELL_SIZE as f64).ceil() as i32 + 1;

        let mut result = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2);
                if let Some(ents) = self.cells.get(&cell) {
                    result.extend_from_slice(ents);
                }
            }
        }
        result
    }

    /// The cell an entity is currently indexed under (diagnostics/tests).
    pub fn cell_of(&self, entity: Entity) -> Option<(i32, i32, i32)> {
        self.entity_cells.get(&entity).copied()
    }

    pub fn entity_count(&self) -> usize {
        self.entity_cells.len()
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

// ---------------------------------------------------------------------------
// Sync systems / observers
// ---------------------------------------------------------------------------

/// System: sync the spatial index for every entity whose position changed.
///
/// Gameplay entities carry the `WorldPosition` wrapper (what movement writes);
/// raw `WorldPos` remains supported for entities that use the bare coordinate
/// type directly. Run this in `PostUpdate` so that all position mutations from
/// game systems are captured before rendering or AI queries.
pub fn sync_spatial_index(
    mut index: ResMut<EntitySpatialIndex>,
    changed: Query<(Entity, &WorldPosition), Changed<WorldPosition>>,
    changed_raw: Query<(Entity, &WorldPos), (Changed<WorldPos>, Without<WorldPosition>)>,
) {
    for (entity, pos) in &changed {
        index.update_position(entity, pos.get());
    }
    for (entity, &pos) in &changed_raw {
        index.update_position(entity, pos);
    }
}

/// Observer: remove an entity from the index when its position component is
/// dropped (either position flavor).
///
/// `Remove` fires before the component is gone, but we only need the entity
/// ID here, so the timing doesn't matter.
pub fn remove_from_spatial_index(
    trigger: On<Remove, WorldPosition>,
    mut index: ResMut<EntitySpatialIndex>,
) {
    index.remove_entity(trigger.entity);
}

/// Observer twin for entities that used the raw `WorldPos` component.
pub fn remove_raw_pos_from_spatial_index(
    trigger: On<Remove, WorldPos>,
    mut index: ResMut<EntitySpatialIndex>,
) {
    index.remove_entity(trigger.entity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdda_core_types::core::coords::ZLevel;

    /// The sync system must track the **gameplay** position component
    /// (`WorldPosition`, what movement writes), not only the raw `WorldPos`.
    #[test]
    fn sync_tracks_world_position_and_raw_pos_alike() {
        let mut world = World::new();
        world.init_resource::<EntitySpatialIndex>();

        let z = ZLevel::new(0);
        let mover = world.spawn(WorldPosition::new(WorldPos::new(1, 2, z))).id();
        let raw = world.spawn(WorldPos::new(40, 0, z)).id();

        let mut sys = IntoSystem::into_system(sync_spatial_index);
        sys.initialize(&mut world);
        sys.run((), &mut world);

        let index = world.resource::<EntitySpatialIndex>();
        assert_eq!(index.entity_count(), 2, "both position flavors indexed");
        assert_eq!(
            index.query_radius_2d(WorldPos::new(1, 2, z), 0.0),
            vec![mover],
            "WorldPosition entity indexed at its gameplay position"
        );
        assert_eq!(
            index.query_radius_2d(WorldPos::new(40, 0, z), 0.0),
            vec![raw],
            "raw WorldPos entity still indexed"
        );

        // Movement writes WorldPosition → the next sync moves the entity.
        world.get_mut::<WorldPosition>(mover).unwrap().set(WorldPos::new(30, 2, z));
        sys.run((), &mut world);
        let index = world.resource::<EntitySpatialIndex>();
        assert_eq!(
            index.cell_of(mover),
            Some((1, 0, 0)),
            "moved entity re-indexed into the cell of its new position"
        );
        assert_eq!(index.cell_of(raw), Some((2, 0, 0)), "raw entity untouched");
    }
}
