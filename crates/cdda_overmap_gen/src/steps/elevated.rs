//! Step 8: Generate elevated layers (bridges, railroad bridges).
//!
//! Port of CDDA master's `overmap::generate_over()` (overmap.cpp L1153-1204).
//!
//! Scans ground-level (z=0) for bridge tiles and places elevated bridge
//! road at z=1 with support columns descending through water.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use tracing::info;

/// Generate elevated layers (bridges).
///
/// Currently only z=1 is generated (matching CDDA master which only
/// places bridge surfaces at z=1).
///
/// # Algorithm
///
/// 1. Build a dense grid of ground-level (z=0) terrain.
/// 2. For every OMT tile flagged [`TerrainFlags::BRIDGE`]:
///    - Place `bridge_road_ns` (or `bridge_road` fallback) at z=1.
///    - Scan downward from z=0 through z=-10: while the tile at that
///      z-level is water, place a bridge support column.
pub fn generate_over(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
) {
    // CDDA only generates bridge surfaces at z=1.
    let z: i8 = 1;

    let mut bridge_points: Vec<(i32, i32)> = Vec::new();

    // ------------------------------------------------------------------
    // Build a dense 180×180 grid of ground-level terrain type indices.
    // ------------------------------------------------------------------
    let mut grid_ground = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    grid_ground[gx][gy] = chunk.get(lx, ly).0;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Scan every OMT tile at z=0 for bridge flags.
    // ------------------------------------------------------------------
    for x in 0..OMAP_DIM as usize {
        for y in 0..OMAP_DIM as usize {
            let ground = TerrainHandle(grid_ground[x][y]);
            let flags = registry.flags_for(ground);

            if !flags.contains(TerrainFlags::BRIDGE) {
                continue;
            }

            // Place bridge road surface at z=1.
            let bridge_road = registry
                .handle_by_id("bridge_road_ns")
                .or_else(|| registry.handle_by_id("bridge_road"))
                .unwrap_or(TerrainHandle::NULL);
            place_in_chunk_z(&mut chunks, x as i32, y as i32, z, bridge_road);
            bridge_points.push((x as i32, y as i32));

            // Place support columns downward through water.
            // Range: 0 down to -10.  Rust inclusive-range syntax requires
            // start <= end, so we write (-10..=0).rev().
            for sz in (-10i8..=0i8).rev() {
                if !is_water_at(&chunks, x as i32, y as i32, sz, &registry) {
                    break;
                }
                let bridge_support = registry
                    .handle_by_id("bridge_ns")
                    .or_else(|| registry.handle_by_id("bridge"))
                    .unwrap_or(TerrainHandle::NULL);
                place_in_chunk_z(&mut chunks, x as i32, y as i32, sz, bridge_support);
            }
        }
    }

    info!(
        "Elevated generated: {} bridge tiles for overmap ({}, {})",
        bridge_points.len(),
        config.om_x,
        config.om_y
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check whether the OMT tile at `(omt_x, omt_y, z)` is a water tile
/// (river, lake, or ocean).
fn is_water_at(
    chunks: &Query<(&ChunkPosition, &mut OvermapChunk)>,
    omt_x: i32,
    omt_y: i32,
    z: i8,
    registry: &TerrainRegistry,
) -> bool {
    for (chunk_pos, chunk) in chunks {
        if chunk_pos.z.0 != z {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = omt_x - ox;
        let ly = omt_y - oy;
        if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
            let flags = registry.flags_for(chunk.get(lx as u8, ly as u8));
            return flags.contains(TerrainFlags::RIVER)
                || flags.contains(TerrainFlags::LAKE)
                || flags.contains(TerrainFlags::OCEAN);
        }
    }
    false
}

/// Place a terrain handle at world-absolute OMT coordinates in the chunk
/// that contains the point at the given z-level.
fn place_in_chunk_z(
    chunks: &mut Query<(&ChunkPosition, &mut OvermapChunk)>,
    omt_x: i32,
    omt_y: i32,
    z: i8,
    handle: TerrainHandle,
) {
    for (chunk_pos, mut chunk) in chunks {
        if chunk_pos.z.0 != z {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = omt_x - ox;
        let ly = omt_y - oy;
        if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
            chunk.set(lx as u8, ly as u8, handle);
            return;
        }
    }
}
