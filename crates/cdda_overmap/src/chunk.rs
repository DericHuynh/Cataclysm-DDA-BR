//! Overmap chunk — 30×30 block of OMT positions, each backed by 2×2 submaps.
//!
//! # CDDA coordinate hierarchy
//!
//! ```text
//! Map tile      — finest unit, 1×1 tile
//! Submap        — 12×12 map tiles
//! OMT           — 1 overmap tile = 2×2 submaps = 24×24 map tiles
//! OvermapChunk  — 30×30 OMTs (storage partition only; not a CDDA concept)
//! Overmap       — 180×180 OMTs = 6×6 chunks per z-level
//! ```
//!
//! Each `OvermapChunk` is a Bevy entity with `ChunkPosition` + `OvermapChunk`
//! components. Chunks own the terrain handles for their 30×30 OMTs. Each OMT
//! position in a chunk corresponds to exactly 4 submaps arranged as:
//!
//! ```text
//! [sw, se]   (submap_x + submap_y * SUBMAPS_PER_ROW)
//! [nw, ne]
//! ```
//!
//! where `nw` = submap at local offset (0,0) in OMT-local submap coords.

use crate::registry::TerrainHandle;
use crate::serial::z_to_index;
use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::{OmPos, ZLevel};

// ---------------------------------------------------------------------------
// Submap constants
// ---------------------------------------------------------------------------

/// Width/height of a submap in map tiles.
pub const SUBMAP_DIM: usize = 12;

/// Total map tiles in one submap.
pub const SUBMAP_SIZE: usize = SUBMAP_DIM * SUBMAP_DIM; // 144

/// Submaps per OMT along one axis (an OMT is 2×2 submaps).
pub const SUBMAPS_PER_OMT_AXIS: usize = 2;

/// Total submaps per OMT.
pub const SUBMAPS_PER_OMT: usize = SUBMAPS_PER_OMT_AXIS * SUBMAPS_PER_OMT_AXIS; // 4

/// Width of one OMT in map tiles.
pub const OMT_DIM_TILES: usize = SUBMAP_DIM * SUBMAPS_PER_OMT_AXIS; // 24

// ---------------------------------------------------------------------------
// Chunk (storage partition) constants
// ---------------------------------------------------------------------------

/// OMTs per chunk side. 180 / 6 = 30 exactly — no edge waste.
pub const CHUNK_DIM: usize = 30;

/// Total OMT slots in one chunk.
pub const CHUNK_SIZE: usize = CHUNK_DIM * CHUNK_DIM; // 900

/// Chunks per overmap per z-level (6×6 = 36).
pub const CHUNKS_PER_LAYER: usize = 6 * 6; // 36

/// Chunks per full 21-layer overmap.
pub const CHUNKS_PER_OVERMAP: usize = CHUNKS_PER_LAYER * 21; // 756

/// Width of the overmap in OMTs.
pub const OMAP_DIM: i32 = 180;

/// Width of the overmap in submaps.
pub const OMAP_DIM_SUBMAPS: i32 = OMAP_DIM * SUBMAPS_PER_OMT_AXIS as i32; // 360

// ---------------------------------------------------------------------------
// ChunkPosition
// ---------------------------------------------------------------------------

/// Identifies where a storage chunk lives in the world.
///
/// # Invariants
/// - `chunk_x` ∈ 0..6  (column within the owning overmap)
/// - `chunk_y` ∈ 0..6  (row within the owning overmap)
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
    /// World-absolute OMT coordinate of the top-left corner of this chunk.
    #[inline]
    pub fn omt_origin(&self) -> (i32, i32) {
        (
            self.om_x * OMAP_DIM + self.chunk_x as i32 * CHUNK_DIM as i32,
            self.om_y * OMAP_DIM + self.chunk_y as i32 * CHUNK_DIM as i32,
        )
    }

    /// World-absolute submap coordinate of the top-left corner of this chunk.
    ///
    /// Each OMT is 2×2 submaps, so the submap origin is `omt_origin * 2`.
    #[inline]
    pub fn submap_origin(&self) -> (i32, i32) {
        let (ox, oy) = self.omt_origin();
        (ox * SUBMAPS_PER_OMT_AXIS as i32, oy * SUBMAPS_PER_OMT_AXIS as i32)
    }

    /// The overmap position of the owning overmap.
    #[inline]
    pub fn om_pos(&self) -> OmPos {
        OmPos::new(self.om_x, self.om_y, self.z)
    }

    /// Pack into a u64 for fast HashMap keying.
    ///
    /// Layout: `[om_x: 16][om_y: 16][z_index: 8][chunk_x: 8][chunk_y: 8]` (low bits unused)
    #[inline]
    pub fn to_key(&self) -> u64 {
        ((self.om_x as u64 & 0xFFFF) << 48)
            | ((self.om_y as u64 & 0xFFFF) << 32)
            | ((z_to_index(self.z) as u64) << 16)
            | ((self.chunk_x as u64) << 8)
            | (self.chunk_y as u64)
    }
}

// ---------------------------------------------------------------------------
// ChunkState — generation lifecycle, lives alongside ChunkPosition
// ---------------------------------------------------------------------------

/// Lifecycle state of a chunk entity.
///
/// Systems downstream of generation (serialization, rendering, AI) should
/// gate on `ChunkState::Ready` to avoid operating on partial data.
///
/// Transitions: `Generating` → `Finalizing` → `Ready`
///
/// Never mutate `OvermapChunk` terrain data when state is `Ready` unless
/// re-entering `Generating` (e.g. for runtime edits). This lets
/// `Changed<OvermapChunk>` be meaningful rather than firing constantly
/// during multi-pass generation.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChunkState {
    /// Actively being written by mapgen passes. `Changed<OvermapChunk>` is
    /// expected to fire frequently. Downstream systems should ignore this chunk.
    #[default]
    Generating,
    /// Mapgen complete; awaiting finalization (connection passes, post-processing).
    Finalizing,
    /// Fully generated and consistent. Safe for downstream systems to read.
    Ready,
}

// ---------------------------------------------------------------------------
// Overmap relationship — chunk-to-overmap tracking
// ---------------------------------------------------------------------------

/// Relationship: this chunk belongs to the given overmap entity.
///
/// Do not query as `&mut`. Mutate by reinserting via commands:
/// `commands.entity(chunk).insert(ChunkOfOvermap(overmap));`
#[derive(Component)]
#[relationship(relationship_target = OvermapChunks)]
pub struct ChunkOfOvermap(pub Entity);

/// Relationship target: the set of all chunk entities belonging to an overmap.
///
/// The inner `Vec` is maintained automatically by `ChunkOfOvermap` hooks.
/// Never mutate it directly.
#[derive(Component, Default)]
#[relationship_target(relationship = ChunkOfOvermap, linked_spawn)]
pub struct OvermapChunks(Vec<Entity>);

impl OvermapChunks {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ---------------------------------------------------------------------------
// OvermapChunk — 30×30 grid of terrain handles (one per OMT slot)
// ---------------------------------------------------------------------------

/// A 30×30 block of overmap terrain handles, one per OMT position.
///
/// Row-major layout: index = `local_y * CHUNK_DIM + local_x`.
/// Each slot holds the terrain handle for that OMT. The four submaps that
/// compose each OMT are identified by deriving their world submap coordinates
/// from the `ChunkPosition::submap_origin` + `(local_x*2 + sx, local_y*2 + sy)`.
///
/// Submap content (the 12×12 map tiles within each submap) is stored
/// separately, keyed by world submap coordinates, not in this struct.
///
/// # Mutation discipline
/// Only mutate via `&mut OvermapChunk` when `ChunkState::Generating` or
/// `ChunkState::Finalizing`. After transition to `Ready`, treat as immutable.
#[derive(Component, Clone)]
pub struct OvermapChunk {
    /// Flat array of terrain handles, one per OMT position.
    /// Index: `local_y as usize * CHUNK_DIM + local_x as usize`.
    pub terrain: Box<[TerrainHandle; CHUNK_SIZE]>,
}

impl OvermapChunk {
    /// Create a chunk with every OMT slot filled with the given handle.
    pub fn new_filled(fill: TerrainHandle) -> Self {
        Self {
            terrain: Box::new([fill; CHUNK_SIZE]),
        }
    }

    /// Get the terrain handle at local OMT coordinates (0..30, 0..30).
    ///
    /// No bounds check — caller guarantees `local_x < 30` and `local_y < 30`.
    #[inline]
    pub fn get(&self, local_x: u8, local_y: u8) -> TerrainHandle {
        self.terrain[local_y as usize * CHUNK_DIM + local_x as usize]
    }

    /// Set the terrain handle at local OMT coordinates.
    ///
    /// No-ops if the value is unchanged, avoiding spurious `Changed` detection.
    #[inline]
    pub fn set(&mut self, local_x: u8, local_y: u8, handle: TerrainHandle) {
        let idx = local_y as usize * CHUNK_DIM + local_x as usize;
        if self.terrain[idx] != handle {
            self.terrain[idx] = handle;
        }
    }

    /// Fill every OMT slot with a single terrain type.
    pub fn fill(&mut self, handle: TerrainHandle) {
        self.terrain.fill(handle);
    }

    /// Iterate over all OMT slots with their local coordinates.
    ///
    /// Yields `(local_x, local_y, handle)` in row-major order.
    pub fn iter_tiles(&self) -> impl Iterator<Item = (u8, u8, TerrainHandle)> + '_ {
        (0u8..CHUNK_DIM as u8).flat_map(move |ly| {
            (0u8..CHUNK_DIM as u8).map(move |lx| (lx, ly, self.get(lx, ly)))
        })
    }

    /// Iterate over the four submap local offsets for an OMT slot.
    ///
    /// Returns `(submap_local_x, submap_local_y)` pairs within the chunk's
    /// submap grid (which is 60×60 submaps for a 30×30 OMT chunk).
    /// Use `ChunkPosition::submap_origin()` to get world-absolute submap coords.
    #[inline]
    pub fn submap_offsets_for(local_x: u8, local_y: u8) -> [(u8, u8); SUBMAPS_PER_OMT] {
        let sx = local_x as u8 * SUBMAPS_PER_OMT_AXIS as u8;
        let sy = local_y as u8 * SUBMAPS_PER_OMT_AXIS as u8;
        [
            (sx,     sy    ),   // NW submap
            (sx + 1, sy    ),   // NE submap
            (sx,     sy + 1),   // SW submap
            (sx + 1, sy + 1),   // SE submap
        ]
    }
}
