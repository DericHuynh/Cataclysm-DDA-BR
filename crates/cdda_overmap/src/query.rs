//! Efficient terrain queries over chunk entities.
//!
//! # String matching
//!
//! The original `is_ot_match` with `OtMatchType::Prefix` / `Contains` did
//! string operations in hot mapgen loops. This file replaces that with:
//!
//! - `has_flag` — O(1) bitflag check for properties like ROAD, FOREST, RIVER.
//! - `has_family` — O(1) integer comparison for terrain families (all `road_*`
//!   variants share a family ID assigned at registration time).
//!
//! `is_ot_match` is preserved for asset-loading and editor use cases where
//! string matching is acceptable, but it is explicitly marked as a cold-path
//! function and must never be called from mapgen or gameplay loops.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use cdda_core_types::core::coords::ZLevel;
use crate::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use crate::index::ChunkIndex;
use crate::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};

// ---------------------------------------------------------------------------
// Hot-path query helpers
// ---------------------------------------------------------------------------

/// Check if a terrain handle has all of the given flags set.
///
/// O(1). Use this in mapgen and gameplay loops instead of string matching.
///
/// ```rust,ignore
/// if has_flag(handle, TerrainFlags::ROAD | TerrainFlags::HIGHWAY, &registry) { ... }
/// ```
#[inline]
pub fn has_flag(handle: TerrainHandle, flags: u16, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).0 & flags == flags
}

/// Check if a terrain handle has any of the given flags set.
///
/// O(1). Equivalent to `flags_for(handle).intersects(mask)`.
#[inline]
pub fn has_any_flag(handle: TerrainHandle, flags: u16, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).0 & flags != 0
}

/// Check if a terrain handle belongs to the named family.
///
/// Slightly more expensive than `has_flag` due to one HashMap lookup on the
/// family name. For tight loops, pre-resolve the family ID with
/// `registry.family_id_by_name()` and compare integers directly.
#[inline]
pub fn has_family(handle: TerrainHandle, family: &str, registry: &TerrainRegistry) -> bool {
    registry.is_family(handle, family)
}

// ---------------------------------------------------------------------------
// is_ot_match — cold path only (asset loading, editor, debug)
// ---------------------------------------------------------------------------

/// Match strategy for `is_ot_match`.
///
/// **Do not use in mapgen or gameplay loops.** Use `has_flag` or `has_family`
/// for runtime checks. This enum exists for asset-loading and editor code
/// that needs to classify terrain from string data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtMatchType {
    /// ID must match exactly.
    Exact,
    /// ID matches exactly, OR stripping a cardinal-direction suffix matches.
    /// e.g. pattern `"road"` matches `"road_north"`.
    Type,
    /// ID matches exactly, OR starts with `pattern_`.
    Prefix,
    /// ID contains the pattern as a substring.
    Contains,
}

/// Check if a terrain handle matches a pattern string.
///
/// # Cold path only
///
/// This function performs string operations and a reverse O(N) ID lookup.
/// It must not be called in mapgen loops or during gameplay. Use it only
/// during asset loading to assign `TerrainFlags` or `family_id` values, so
/// that runtime code can use O(1) integer checks thereafter.
pub fn is_ot_match(
    pattern: &str,
    handle: TerrainHandle,
    registry: &TerrainRegistry,
    match_type: OtMatchType,
) -> bool {
    let Some(id) = registry.string_id_for(handle) else {
        return false;
    };
    match match_type {
        OtMatchType::Exact => id == pattern,
        OtMatchType::Type => {
            if id == pattern {
                return true;
            }
            if let Some(pos) = id.rfind('_') {
                let suffix = &id[pos + 1..];
                if matches!(suffix, "north" | "south" | "east" | "west") {
                    return &id[..pos] == pattern;
                }
            }
            false
        }
        OtMatchType::Prefix => {
            id == pattern
                || (id.starts_with(pattern)
                    && id.as_bytes().get(pattern.len()) == Some(&b'_'))
        }
        OtMatchType::Contains => id.contains(pattern),
    }
}

// ---------------------------------------------------------------------------
// TerrainQuery — SystemParam for spatial terrain reads
// ---------------------------------------------------------------------------

/// System param for querying overmap terrain at world-absolute OMT positions.
///
/// # Coordinate model
///
/// World-absolute OMT coordinates are continuous integers. The mapping to
/// chunk storage is:
///
/// ```text
/// om_x      = omt_x.div_euclid(OMAP_DIM)        // which overmap
/// om_y      = omt_y.div_euclid(OMAP_DIM)
/// chunk_col = omt_x.rem_euclid(OMAP_DIM).div_euclid(CHUNK_DIM as i32)   // 0..6
/// chunk_row = omt_y.rem_euclid(OMAP_DIM).div_euclid(CHUNK_DIM as i32)   // 0..6
/// local_x   = omt_x.rem_euclid(CHUNK_DIM as i32)   // 0..30
/// local_y   = omt_y.rem_euclid(CHUNK_DIM as i32)   // 0..30
/// ```
///
/// CHUNK_DIM is 30, OMAP_DIM is 180, so all divisions are exact with no
/// remainder waste at edges.
#[derive(SystemParam)]
pub struct TerrainQuery<'w, 's> {
    pub chunks: Query<'w, 's, (&'static ChunkPosition, &'static OvermapChunk)>,
    pub registry: Res<'w, TerrainRegistry>,
    pub index: Res<'w, ChunkIndex>,
}

impl<'w, 's> TerrainQuery<'w, 's> {
    /// Get the terrain handle at a world-absolute OMT position.
    ///
    /// Returns `TerrainHandle::NULL` if the chunk is not loaded.
    pub fn at(&self, omt_x: i32, omt_y: i32, z: i32) -> TerrainHandle {
        let (chunk_pos, local_x, local_y) = Self::decompose(omt_x, omt_y, z);
        if let Some(entity) = self.index.get(&chunk_pos) {
            if let Ok((_, chunk)) = self.chunks.get(entity) {
                return chunk.get(local_x, local_y);
            }
        }
        TerrainHandle::NULL
    }

    /// Get the flags for the terrain at a world-absolute OMT position.
    pub fn flags_at(&self, omt_x: i32, omt_y: i32, z: i32) -> TerrainFlags {
        self.registry.flags_for(self.at(omt_x, omt_y, z))
    }

    /// Check if the terrain at a position has all of the given flags.
    #[inline]
    pub fn has_flag_at(&self, omt_x: i32, omt_y: i32, z: i32, flags: u16) -> bool {
        has_flag(self.at(omt_x, omt_y, z), flags, &self.registry)
    }

    /// Check if the terrain at a position belongs to the named family.
    #[inline]
    pub fn is_family_at(&self, omt_x: i32, omt_y: i32, z: i32, family: &str) -> bool {
        has_family(self.at(omt_x, omt_y, z), family, &self.registry)
    }

    /// Collect all terrain handles within a viewport rectangle.
    pub fn viewport_grid(
        &self,
        center_x: i32,
        center_y: i32,
        z: i32,
        half_width: usize,
        half_height: usize,
    ) -> Vec<Vec<TerrainHandle>> {
        let x0 = center_x - half_width as i32;
        let y0 = center_y - half_height as i32;
        let rows = half_height * 2 + 1;
        let cols = half_width * 2 + 1;

        let mut grid = vec![vec![TerrainHandle::NULL; cols]; rows];
        for row in 0..rows {
            for col in 0..cols {
                grid[row][col] = self.at(x0 + col as i32, y0 + row as i32, z);
            }
        }
        grid
    }

    /// Human-readable string ID for a terrain handle.
    ///
    /// O(N) reverse lookup — for debugging and UI display only.
    pub fn name_for(&self, handle: TerrainHandle) -> &str {
        self.registry.string_id_for(handle).unwrap_or("unknown")
    }

    // -- Coordinate decomposition --------------------------------------------

    /// Decompose a world-absolute OMT position into a `ChunkPosition` plus
    /// local coordinates within that chunk.
    ///
    /// With `CHUNK_DIM = 30` and `OMAP_DIM = 180 = 6 * 30`, all divisions are
    /// exact; there are no partial edge chunks.
    #[inline]
    fn decompose(omt_x: i32, omt_y: i32, z: i32) -> (ChunkPosition, u8, u8) {
        // Which overmap owns this OMT?
        let om_x = omt_x.div_euclid(crate::chunk::OMAP_DIM);
        let om_y = omt_y.div_euclid(crate::chunk::OMAP_DIM);

        // OMT position within the overmap (0..180).
        let omt_local_x = omt_x.rem_euclid(crate::chunk::OMAP_DIM);
        let omt_local_y = omt_y.rem_euclid(crate::chunk::OMAP_DIM);

        // Which chunk within the overmap (0..6)?
        let chunk_col = omt_local_x / CHUNK_DIM as i32;
        let chunk_row = omt_local_y / CHUNK_DIM as i32;

        // Local OMT coordinates within the chunk (0..30).
        let local_x = (omt_local_x % CHUNK_DIM as i32) as u8;
        let local_y = (omt_local_y % CHUNK_DIM as i32) as u8;

        let chunk_pos = ChunkPosition {
            om_x,
            om_y,
            z: ZLevel::new(z as i8),
            chunk_x: chunk_col as u8,
            chunk_y: chunk_row as u8,
        };

        (chunk_pos, local_x, local_y)
    }
}

// ---------------------------------------------------------------------------
// SubMapQuery — world-submap-coordinate queries
// ---------------------------------------------------------------------------

/// Convert a world-absolute submap coordinate to the OMT that contains it.
///
/// Each OMT is 2×2 submaps. Returns `(omt_x, omt_y)`.
#[inline]
pub fn submap_to_omt(submap_x: i32, submap_y: i32) -> (i32, i32) {
    (
        submap_x.div_euclid(crate::chunk::SUBMAPS_PER_OMT_AXIS as i32),
        submap_y.div_euclid(crate::chunk::SUBMAPS_PER_OMT_AXIS as i32),
    )
}

/// Convert a world-absolute OMT coordinate to the origin submap (NW corner).
///
/// Returns `(submap_x, submap_y)` of the NW submap in that OMT.
#[inline]
pub fn omt_to_submap_origin(omt_x: i32, omt_y: i32) -> (i32, i32) {
    (
        omt_x * crate::chunk::SUBMAPS_PER_OMT_AXIS as i32,
        omt_y * crate::chunk::SUBMAPS_PER_OMT_AXIS as i32,
    )
}

/// Return all four submap world coordinates for a given OMT.
///
/// Order: `[NW, NE, SW, SE]` matching `OvermapChunk::submap_offsets_for`.
#[inline]
pub fn omt_submaps(omt_x: i32, omt_y: i32) -> [(i32, i32); 4] {
    let (sx, sy) = omt_to_submap_origin(omt_x, omt_y);
    [(sx, sy), (sx + 1, sy), (sx, sy + 1), (sx + 1, sy + 1)]
}
