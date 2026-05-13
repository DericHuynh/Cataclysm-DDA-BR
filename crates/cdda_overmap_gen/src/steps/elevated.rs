//! Step 8: Generate elevated layers (bridges, railroad bridges).
//!
//! Port of CDDA master's `overmap::generate_over()` (overmap.cpp L1153-1204).
//!
//! Scans ground-level (z=0) for bridge tiles and places elevated bridge
//! road at z=1 with support columns descending through water.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
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
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
) {
    // CDDA only generates bridge surfaces at z=1.
    let z: i8 = 1;

    let mut bridge_points: Vec<(i32, i32)> = Vec::new();

    // ------------------------------------------------------------------
    // Build dense grids for z=0 (ground) and all z-levels we check for water.
    // We need z=0 through z=-10 for the water column scan.
    // ------------------------------------------------------------------
    let mut grid_ground = [[0u32; 180]; 180];
    let mut water_grids: [[[u32; 180]; 180]; 11] = [[[0u32; 180]; 180]; 11]; // indices 0..=10 map to z=0..=-10

    for (_entity, chunk_pos, chunk) in &chunks {
        let gz = chunk_pos.z.0;
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= 180 || gy >= 180 {
                    continue;
                }
                let raw = chunk.get(lx, ly).0;
                if gz == 0 {
                    grid_ground[gx][gy] = raw;
                }
                // Map z=-10..=0 to indices 10..=0
                if gz >= -10 && gz <= 0 {
                    let wi = (-gz) as usize;
                    water_grids[wi][gx][gy] = raw;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Collect all writes: (omt_x, omt_y, z, handle)
    // ------------------------------------------------------------------
    let mut writes: Vec<(i32, i32, i8, TerrainHandle)> = Vec::new();

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
            writes.push((x as i32, y as i32, z, bridge_road));
            bridge_points.push((x as i32, y as i32));

            // Place support columns downward through water.
            // Range: 0 down to -10.
            for sz in (-10i8..=0i8).rev() {
                let wi = (-sz) as usize;
                let h = TerrainHandle(water_grids[wi][x][y]);
                let wflags = registry.flags_for(h);
                let is_water = wflags.contains(TerrainFlags::RIVER)
                    || wflags.contains(TerrainFlags::LAKE)
                    || wflags.contains(TerrainFlags::OCEAN);
                if !is_water {
                    break;
                }
                let bridge_support = registry
                    .handle_by_id("bridge_ns")
                    .or_else(|| registry.handle_by_id("bridge"))
                    .unwrap_or(TerrainHandle::NULL);
                writes.push((x as i32, y as i32, sz, bridge_support));
            }
        }
    }

    // ------------------------------------------------------------------
    // Write-back: apply all collected writes via par_iter
    // ------------------------------------------------------------------
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z_chunk = chunk_pos.z.0;
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, wz, handle) in &writes {
            if wz != z_chunk {
                continue;
            }
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                if new_terrain[idx] != handle {
                    new_terrain[idx] = handle;
                    modified = true;
                }
            }
        }

        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: new_terrain,
                });
            });
        }
    });

    info!(
        "Elevated generated: {} bridge tiles for overmap ({}, {})",
        bridge_points.len(),
        config.om_x,
        config.om_y
    );
}
