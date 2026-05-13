//! Efficient terrain queries over chunk entities.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use cdda_core_types::core::coords::ZLevel;
use crate::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use crate::index::ChunkIndex;
use crate::registry::{TerrainHandle, TerrainRegistry};

/// Match strategy for `is_ot_match`, ported from CDDA's `is_ot_match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtMatchType {
    /// ID must match exactly.
    Exact,
    /// ID matches exactly, OR removing a `_north`/`_south`/`_east`/`_west` suffix matches.
    Type,
    /// ID matches exactly, OR ID starts with pattern followed by `_`.
    Prefix,
    /// ID contains the pattern as a substring.
    Contains,
}

/// Check if a terrain handle matches a pattern using CDDA's `is_ot_match` logic.
///
/// `pattern` is a terrain string ID (e.g. `"forest"`, `"sub_station"`, `"river"`).
/// `handle` is the terrain handle to test.
/// `registry` provides the string ID lookup for the handle.
/// `match_type` controls the matching strategy.
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
                if suffix == "north" || suffix == "south" || suffix == "east" || suffix == "west" {
                    return &id[..pos] == pattern;
                }
            }
            false
        }
        OtMatchType::Prefix => {
            if id == pattern {
                return true;
            }
            id.starts_with(pattern) && id.as_bytes().get(pattern.len()) == Some(&b'_')
        }
        OtMatchType::Contains => id.contains(pattern),
    }
}

/// System param for querying overmap terrain at world-absolute OMT positions.
#[derive(SystemParam)]
pub struct TerrainQuery<'w, 's> {
    pub chunks: Query<'w, 's, (&'static ChunkPosition, &'static OvermapChunk)>,
    pub registry: Res<'w, TerrainRegistry>,
    pub index: Res<'w, ChunkIndex>,
}

impl<'w, 's> TerrainQuery<'w, 's> {
    /// Get the terrain handle at a world-absolute OMT position.
    pub fn at(&self, x: i32, y: i32, z: i32) -> TerrainHandle {
        let chunk_x = x.div_euclid(CHUNK_DIM as i32);
        let chunk_y = y.div_euclid(CHUNK_DIM as i32);
        let om_x = chunk_x.div_euclid(6);
        let om_y = chunk_y.div_euclid(6);
        let local_cx = chunk_x.rem_euclid(6) as u8;
        let local_cy = chunk_y.rem_euclid(6) as u8;
        let local_tile_x = x.rem_euclid(CHUNK_DIM as i32) as u8;
        let local_tile_y = y.rem_euclid(CHUNK_DIM as i32) as u8;

        let key_pos = ChunkPosition {
            om_x,
            om_y,
            z: ZLevel::new(z as i8),
            chunk_x: local_cx,
            chunk_y: local_cy,
        };
        if let Some(entity) = self.index.get(&key_pos) {
            if let Ok((_, chunk)) = self.chunks.get(entity) {
                return chunk.get(local_tile_x, local_tile_y);
            }
        }
        TerrainHandle::NULL
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

    /// Symbol for a terrain handle (first char of string ID as fallback).
    pub fn symbol_for(&self, handle: TerrainHandle) -> String {
        if handle == TerrainHandle::NULL { return " ".to_string(); }
        let id = self.registry.string_id_for(handle).unwrap_or("?");
        id.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "?".to_string())
    }

    /// Human-readable name for a terrain handle.
    pub fn name_for(&self, handle: TerrainHandle) -> String {
        self.registry.string_id_for(handle).unwrap_or("unknown").to_string()
    }
}
