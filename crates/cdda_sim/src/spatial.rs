//! Spatial index for dynamic entities — 3D grid-based spatial partitioning.
//!
//! Maps WorldPos (x, y, z) → Set<Entity> for rapid lookups by position.
//! Cells include z-level so entities on different floors don't collide.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use cdda_core::coords::WorldPos;
use std::collections::HashMap;

/// The spatial cell size in world tiles (must be a power of two for fast division).
/// Applied independently on x and y axes.
const CELL_SIZE: i32 = 16;

/// Z-cell size: how many z-levels per cell.
/// Unlike horizontal, z is compact so one z-level per cell is fine.
/// This means entities on z=0 and z=1 are in different cells.
const Z_CELL_SIZE: i32 = 1;

/// A dense 3D spatial index mapping world positions to entities.
///
/// Keyed by `(cell_x, cell_y, cell_z)` where cell_z = z.0 as i32 / Z_CELL_SIZE.
/// This ensures entities on different z-levels are in separate cells and
/// won't appear in the same radius query unless dz is explicitly included.
#[derive(Debug, Clone, Default, Resource)]
pub struct EntitySpatialIndex {
    /// Map from 3D cell position to set of entities.
    cells: HashMap<(i32, i32, i32), Vec<Entity>>,
    /// Map from entity to the 3D cell it currently occupies.
    entity_cells: HashMap<Entity, (i32, i32, i32)>,
}

impl EntitySpatialIndex {
    /// Create an empty spatial index.
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Compute the 3D cell coordinates for a world position.
    #[inline]
    fn cell(pos: WorldPos) -> (i32, i32, i32) {
        (
            pos.x / CELL_SIZE,
            pos.y / CELL_SIZE,
            pos.z.0 as i32 / Z_CELL_SIZE,
        )
    }

    /// Update an entity's position in the index.
    pub fn update_position(&mut self, entity: Entity, pos: WorldPos) {
        let new_cell = Self::cell(pos);

        // Remove from old cell if present
        if let Some(old_cell) = self.entity_cells.get(&entity) {
            if *old_cell == new_cell {
                return; // No change
            }
            if let Some(cell_entities) = self.cells.get_mut(old_cell) {
                cell_entities.retain(|e| *e != entity);
            }
        }

        // Insert into new cell
        self.cells.entry(new_cell).or_default().push(entity);
        self.entity_cells.insert(entity, new_cell);
    }

    /// Remove an entity from the index (on despawn).
    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(cell) = self.entity_cells.remove(&entity) {
            if let Some(cell_entities) = self.cells.get_mut(&cell) {
                cell_entities.retain(|e| *e != entity);
            }
        }
    }

    /// Query all entities within a given 3D radius of a position.
    ///
    /// Returns entities in all cells whose Chebyshev distance to the center
    /// cell is within `cell_radius`. Because z-cell-size = 1, this naturally
    /// finds entities on adjacent z-levels when the radius covers them.
    pub fn query_radius(&self, center: WorldPos, radius: f64) -> Vec<Entity> {
        let center_cell = Self::cell(center);
        let cell_radius = ((radius / CELL_SIZE as f64).ceil() as i32) + 1;
        // For z, use the horizontal cell_radius too — a large radius on the
        // surface should also reach a few z-levels up/down if they're close.
        let z_radius = cell_radius;

        let mut result = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -z_radius..=z_radius {
                    let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2 + dz);
                    if let Some(cell_entities) = self.cells.get(&cell) {
                        result.extend(cell_entities);
                    }
                }
            }
        }
        result
    }

    /// Query entities in 2D only (same z-level).
    /// Useful for systems that don't care about vertical separation.
    pub fn query_radius_2d(&self, center: WorldPos, radius: f64) -> Vec<Entity> {
        let center_cell = Self::cell(center);
        let cell_radius = ((radius / CELL_SIZE as f64).ceil() as i32) + 1;

        let mut result = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2);
                if let Some(cell_entities) = self.cells.get(&cell) {
                    result.extend(cell_entities);
                }
            }
        }
        result
    }

    /// Number of tracked entities.
    pub fn entity_count(&self) -> usize {
        self.entity_cells.len()
    }

    /// Number of occupied cells.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}
