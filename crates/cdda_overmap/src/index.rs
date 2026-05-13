//! ChunkIndex — O(1) HashMap lookup for chunk entities.
//!
//! Maintained by a system watching `Added<ChunkPosition>` / `RemovedComponents<ChunkPosition>`.
//! Generates a `u64` key from `(om_x, om_y, z, chunk_x, chunk_y)` for instant lookup.

use bevy_ecs::prelude::*;
use std::collections::HashMap;
use crate::chunk::ChunkPosition;

/// O(1) chunk entity lookup by position.
///
/// Keyed by a packed u64: `(om_x: i16, om_y: i16, z_index: u8, chunk_x: u8, chunk_y: u8)`.
#[derive(Resource, Debug, Default)]
pub struct ChunkIndex {
    map: HashMap<u64, Entity>,
}

impl ChunkIndex {
    /// Insert a chunk position → entity mapping.
    pub fn insert(&mut self, pos: &ChunkPosition, entity: Entity) {
        self.map.insert(pos.to_key(), entity);
    }

    /// Remove a chunk entity.
    pub fn remove(&mut self, pos: &ChunkPosition) {
        self.map.remove(&pos.to_key());
    }

    /// Look up a chunk entity by its position.
    pub fn get(&self, pos: &ChunkPosition) -> Option<Entity> {
        self.map.get(&pos.to_key()).copied()
    }

    /// Number of tracked chunks.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if no chunks are tracked.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// System: add newly-spawned chunks to the index.
pub fn index_added_chunks(
    mut index: ResMut<ChunkIndex>,
    added: Query<(Entity, &ChunkPosition), Added<ChunkPosition>>,
) {
    for (entity, pos) in &added {
        index.insert(pos, entity);
    }
}

/// System: remove despawned chunks from the index.
pub fn index_removed_chunks(
    mut index: ResMut<ChunkIndex>,
    mut removals: RemovedComponents<ChunkPosition>,
    // We need the old ChunkPosition — but RemovedComponents only gives Entity.
    // Instead, reconstruct from the index by scanning.
) {
    // Collect entities being removed
    let removed_entities: Vec<Entity> = removals.read().collect();
    if removed_entities.is_empty() {
        return;
    }
    // Find and remove keys for these entities
    let to_remove: Vec<u64> = index.map.iter()
        .filter(|(_, &e)| removed_entities.contains(&e))
        .map(|(&k, _)| k)
        .collect();
    for key in to_remove {
        index.map.remove(&key);
    }
}
