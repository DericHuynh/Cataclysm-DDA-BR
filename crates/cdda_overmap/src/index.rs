//! ChunkIndex — O(1) HashMap lookup for chunk entities.
//!
//! Maintained by Bevy `OnAdd` / `OnRemove` observers rather than manual
//! systems. This is critical for correctness and performance:
//!
//! - `OnRemove` fires **before** the component is dropped, so the observer
//!   can still read `ChunkPosition` to compute the key and remove it in O(1).
//! - The previous approach using `RemovedComponents` only provided the entity
//!   ID after the component was gone, forcing an O(N) scan of the entire map
//!   to find the matching key — a spike on every despawn.
//!
//! # Registration
//!
//! Add both observers in your plugin's `build` method:
//!
//! ```rust,ignore
//! app.add_observer(ChunkIndex::on_chunk_added)
//!    .add_observer(ChunkIndex::on_chunk_removed);
//! ```

use crate::chunk::ChunkPosition;
use bevy_ecs::lifecycle::{Add, Remove};
use bevy_ecs::prelude::*;
use std::collections::HashMap;

/// O(1) chunk entity lookup keyed by packed `ChunkPosition`.
///
/// Keys are `u64` packed by `ChunkPosition::to_key()`.
#[derive(Resource, Debug, Default)]
pub struct ChunkIndex {
    map: HashMap<u64, Entity>,
}

impl ChunkIndex {
    /// Insert a mapping. Called by the `OnAdd` observer.
    pub fn insert(&mut self, pos: &ChunkPosition, entity: Entity) {
        self.map.insert(pos.to_key(), entity);
    }

    /// Remove a mapping by position. Called by the `OnRemove` observer.
    ///
    /// O(1) — uses the key derived from `ChunkPosition` directly.
    pub fn remove(&mut self, pos: &ChunkPosition) {
        self.map.remove(&pos.to_key());
    }

    /// Look up a chunk entity by position.
    #[inline]
    pub fn get(&self, pos: &ChunkPosition) -> Option<Entity> {
        self.map.get(&pos.to_key()).copied()
    }

    /// Number of tracked chunks.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    // -- Observers ------------------------------------------------------------

    /// Observer: insert newly-added chunk into the index.
    ///
    /// Fires after `ChunkPosition` is inserted on an entity.
    pub fn on_chunk_added(
        trigger: On<Add, ChunkPosition>,
        query: Query<&ChunkPosition>,
        mut index: ResMut<ChunkIndex>,
    ) {
        if let Ok(pos) = query.get(trigger.entity) {
            index.insert(pos, trigger.entity);
        }
    }

    /// Observer: remove a chunk from the index when its `ChunkPosition` is
    /// removed or the entity is despawned.
    ///
    /// `Remove` fires **before** the component is dropped, so we can still
    /// read `ChunkPosition` here for an O(1) key derivation.
    pub fn on_chunk_removed(
        trigger: On<Remove, ChunkPosition>,
        query: Query<&ChunkPosition>,
        mut index: ResMut<ChunkIndex>,
    ) {
        if let Ok(pos) = query.get(trigger.entity) {
            index.remove(pos);
        }
    }
}
