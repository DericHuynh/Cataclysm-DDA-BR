//! 3D spatial index for dynamic entities — grid-based spatial partitioning.
//!
//! Maps WorldPos (x, y, z) → Set<Entity> for rapid radius queries.
//! Cells include z-level so entities on different floors don't collide.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use cdda_core_types::core::coords::WorldPos;
use std::collections::HashMap;

/// Cell size in world tiles (must be power of two for fast division).
const CELL_SIZE: i32 = 16;

/// Z-cell size: one z-level per cell.
const Z_CELL_SIZE: i32 = 1;

/// Maps world positions to entities for efficient radius queries.
#[derive(Debug, Clone, Default, Resource)]
pub struct EntitySpatialIndex {
    cells: HashMap<(i32, i32, i32), Vec<Entity>>,
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

    pub fn update_position(&mut self, entity: Entity, pos: WorldPos) {
        let new_cell = Self::cell(pos);
        if let Some(old_cell) = self.entity_cells.get(&entity) {
            if *old_cell == new_cell {
                return;
            }
            if let Some(cell_entities) = self.cells.get_mut(old_cell) {
                cell_entities.retain(|e| *e != entity);
            }
        }
        self.cells.entry(new_cell).or_default().push(entity);
        self.entity_cells.insert(entity, new_cell);
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        if let Some(cell) = self.entity_cells.remove(&entity) {
            if let Some(cell_entities) = self.cells.get_mut(&cell) {
                cell_entities.retain(|e| *e != entity);
            }
        }
    }

    pub fn query_radius(&self, center: WorldPos, radius: f64) -> Vec<Entity> {
        let center_cell = Self::cell(center);
        let cell_radius = ((radius / CELL_SIZE as f64).ceil() as i32) + 1;
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

    pub fn entity_count(&self) -> usize {
        self.entity_cells.len()
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}
