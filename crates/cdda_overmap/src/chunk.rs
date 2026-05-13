//! Overmap chunk — 32×32 block of terrain handles.
//!
//! Each overmap is decomposed into 36 chunks per z-level (6×6 grid).
//! Chunks are Bevy entities with `ChunkPosition` + `OvermapChunk` components.
//! This enables independent `&mut` access to disjoint chunks during generation
//! and parallel querying by the scheduler.

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::{OmPos, ZLevel};
use crate::registry::TerrainHandle;
use crate::serial::z_to_index;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of one side of a chunk in OMT units.
pub const CHUNK_DIM: usize = 32;

/// Total terrain tiles in a chunk.
pub const CHUNK_SIZE: usize = CHUNK_DIM * CHUNK_DIM; // 1024

/// Number of chunks per overmap per z-level (ceil(180/32) = 6 → 6×6 = 36).
pub const CHUNKS_PER_LAYER: usize = 6 * 6; // 36

/// Number of chunks in a full 21-layer overmap.
pub const CHUNKS_PER_OVERMAP: usize = CHUNKS_PER_LAYER * 21; // 756

/// Total OMT tiles in an overmap.
pub const OMAP_DIM: i32 = 180;

// ---------------------------------------------------------------------------
// ChunkPosition
// ---------------------------------------------------------------------------

/// Identifies where a chunk lives in the world.
///
/// # Invariants
/// - `chunk_x` ∈ 0..6
/// - `chunk_y` ∈ 0..6
/// - `z` ∈ -10..=10
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    /// Overmap x-coordinate in the world grid.
    pub om_x: i32,
    /// Overmap y-coordinate in the world grid.
    pub om_y: i32,
    /// Z-level within the overmap.
    pub z: ZLevel,
    /// Chunk column within the overmap (0..6).
    pub chunk_x: u8,
    /// Chunk row within the overmap (0..6).
    pub chunk_y: u8,
}

impl ChunkPosition {
    /// World-absolute top-left OMT coordinate of this chunk.
    pub fn omt_origin(&self) -> (i32, i32) {
        (
            self.om_x * OMAP_DIM + self.chunk_x as i32 * CHUNK_DIM as i32,
            self.om_y * OMAP_DIM + self.chunk_y as i32 * CHUNK_DIM as i32,
        )
    }

    /// The overmap position.
    pub fn om_pos(&self) -> OmPos {
        OmPos::new(self.om_x, self.om_y, self.z)
    }

    /// Pack into a u64 for fast HashMap keying.
    pub fn to_key(&self) -> u64 {
        ((self.om_x as u64 & 0xFFFF) << 48)
            | ((self.om_y as u64 & 0xFFFF) << 32)
            | ((z_to_index(self.z) as u64) << 16)
            | ((self.chunk_x as u64) << 8)
            | (self.chunk_y as u64)
    }
}

// ---------------------------------------------------------------------------
// Overmap relationship — chunk-to-overmap tracking
// ---------------------------------------------------------------------------

/// Relationship: a chunk belongs to an overmap entity.
///
/// # Mutation
/// Do not query as `&mut`. Mutate by reinserting via commands:
/// `commands.entity(chunk).insert(ChunkOfOvermap(overmap));`
#[derive(Component)]
#[relationship(relationship_target = OvermapChunks)]
pub struct ChunkOfOvermap(pub Entity);

/// Relationship target: tracks all chunks of an overmap.
#[derive(Component)]
#[relationship_target(relationship = ChunkOfOvermap, linked_spawn)]
pub struct OvermapChunks(Vec<Entity>);

impl OvermapChunks {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// ---------------------------------------------------------------------------
// OvermapChunk
// ---------------------------------------------------------------------------

/// A 32×32 block of overmap terrain tiles.
///
/// Stored as a flat array indexed by `(local_y * 32 + local_x)`.
/// Row-major layout for cache-friendly iteration.
///
/// # Mutation
/// During generation: modify via `&mut OvermapChunk` (triggers `Changed<OvermapChunk>`).
/// After generation finalization, mark immutable.
#[derive(Component, Clone)]
pub struct OvermapChunk {
    /// Flat array of terrain handles.
    /// Index: `local_y as usize * CHUNK_DIM + local_x as usize`.
    pub terrain: Box<[TerrainHandle; CHUNK_SIZE]>,
}

impl OvermapChunk {
    /// Create a chunk filled with the given terrain.
    pub fn new_filled(fill: TerrainHandle) -> Self {
        Self {
            terrain: Box::new([fill; CHUNK_SIZE]),
        }
    }

    /// Get terrain at local coordinates (0..32, 0..32).
    /// No bounds check — caller guarantees validity.
    #[inline]
    pub fn get(&self, local_x: u8, local_y: u8) -> TerrainHandle {
        self.terrain[local_y as usize * CHUNK_DIM + local_x as usize]
    }

    /// Set terrain at local coordinates.
    /// Uses `set_if_neq` semantics — only writes if the value actually changes.
    #[inline]
    pub fn set(&mut self, local_x: u8, local_y: u8, handle: TerrainHandle) {
        let idx = local_y as usize * CHUNK_DIM + local_x as usize;
        if self.terrain[idx] != handle {
            self.terrain[idx] = handle;
        }
    }

    /// Fill the entire chunk with a single terrain type.
    pub fn fill(&mut self, handle: TerrainHandle) {
        self.terrain.fill(handle);
    }

    /// Iterate over all tiles with their local coordinates.
    pub fn iter_tiles(&self) -> impl Iterator<Item = (u8, u8, TerrainHandle)> + '_ {
        (0u8..CHUNK_DIM as u8).flat_map(move |ly| {
            (0u8..CHUNK_DIM as u8).map(move |lx| {
                (lx, ly, self.get(lx, ly))
            })
        })
    }
}
