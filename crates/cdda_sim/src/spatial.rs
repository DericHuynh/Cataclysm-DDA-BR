//! Spatial index for dynamic entities.
//!
//! Grid-based spatial partitioning for O(1) radius queries.
//! Maps WorldPos → Set<Entity> for rapid lookups by position.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use cdda_core::coords::WorldPos;
use std::collections::HashMap;

/// The spatial cell size in world tiles (must be a power of two for fast division).
const CELL_SIZE: i32 = 16;

/// A dense spatial index mapping world positions to entities.
///
/// This is a Bevy `Resource` that systems update and query.
/// On each tick:
/// 1. `update_positions` system reads `Changed<WorldPosition>` and updates the index.
/// 2. Query systems call `query_radius` for O(1) spatial lookups.
#[derive(Debug, Clone, Default, Resource)]
pub struct EntitySpatialIndex {
    /// Map from cell position to set of entities.
    cells: HashMap<(i32, i32), Vec<Entity>>,
    /// Map from entity to the cell it currently occupies.
    entity_cells: HashMap<Entity, (i32, i32)>,
}

impl EntitySpatialIndex {
    /// Create an empty spatial index.
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Compute the cell coordinates for a world position.
    #[inline]
    fn cell(pos: WorldPos) -> (i32, i32) {
        (pos.x / CELL_SIZE, pos.y / CELL_SIZE)
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

    /// Query all entities within a given radius of a position.
    /// Returns entities in the current and adjacent cells (Manhattan-adjacent).
    pub fn query_radius(&self, center: WorldPos, radius: f64) -> Vec<Entity> {
        let center_cell = Self::cell(center);
        let cell_radius = ((radius / CELL_SIZE as f64).ceil() as i32) + 1;

        let mut result = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell = (center_cell.0 + dx, center_cell.1 + dy);
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
