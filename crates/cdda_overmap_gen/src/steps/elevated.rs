//! Elevated terrain generation (bridges, z > 0).
//!
//! Port of C++ `overmap::generate_over()` (overmap.cpp L1153-1204).
//!
//! Algorithm:
//! 1. Build terrain grids for z=0 (ground) and all z-levels below (for water column).
//! 2. For every OMT tile at z=0 with BRIDGE flag:
//!    a. Place bridge road surface at z=1.
//!    b. Scan downward: while tile at z below is water, place bridge support column.
//! 3. Write elevated terrain back to chunks.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use std::collections::HashMap;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// generate_over — system entry point
// ---------------------------------------------------------------------------

/// Generate elevated terrain (bridges) above the overmap.
///
/// Port of C++ `overmap::generate_over()` (overmap.cpp L1153-1204).
pub fn generate_over(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
) {
    // --- Resolve terrain handles -------------------------------------------
    let bridge_road_ns = registry
        .handle_by_id("bridge_road_ns")
        .or_else(|| registry.handle_by_id("bridge_road"))
        .map(|h| h.0);
    let bridge_pillar = registry
        .handle_by_id("bridge_pillar")
        .or_else(|| registry.handle_by_id("support_column"))
        .or_else(|| registry.handle_by_id("bridge_column"))
        .map(|h| h.0);

    let Some(bridge_surface) = bridge_road_ns else {
        info!("generate_over: no bridge_road terrain in registry, skipping");
        return;
    };

    // --- Build terrain grids for all z-levels -------------------------------
    let omap_size = OMAP_DIM as usize;

    // Collect grids by z-level
    let mut grids_by_z: HashMap<i32, Vec<TerrainHandle>> = HashMap::new();

    // First pass: initialize grids
    let ground_grid = vec![TerrainHandle::NULL; omap_size * omap_size];
    grids_by_z.insert(0, ground_grid.clone());

    for (_entity, chunk_pos, chunk) in &chunks {
        let z = chunk_pos.z.0 as i32;
        let grid = grids_by_z
            .entry(z)
            .or_insert_with(|| vec![TerrainHandle::NULL; omap_size * omap_size]);

        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as i32 {
            for lx in 0..CHUNK_DIM as i32 {
                let gx = ox + lx;
                let gy = oy + ly;
                if gx >= 0 && gx < OMAP_DIM && gy >= 0 && gy < OMAP_DIM {
                    grid[(gy as usize) * omap_size + (gx as usize)] = chunk.get(lx as u8, ly as u8);
                }
            }
        }
    }

    // --- Get a z=0 grid reference -------------------------------------------
    let ground_grid = grids_by_z
        .get(&0)
        .cloned()
        .unwrap_or_else(|| vec![TerrainHandle::NULL; omap_size * omap_size]);

    let ter_at_z = |z: i32, x: i32, y: i32| -> TerrainHandle {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            if let Some(grid) = grids_by_z.get(&z) {
                grid[(y as usize) * omap_size + (x as usize)]
            } else {
                TerrainHandle::NULL
            }
        } else {
            TerrainHandle::NULL
        }
    };

    let water_flags =
        TerrainFlags::from_bits(TerrainFlags::LAKE | TerrainFlags::OCEAN | TerrainFlags::RIVER);

    // --- Scan for BRIDGE tiles and place elevated road + columns ------------
    // tile_writes: (z, x, y, handle)
    let mut tile_writes: Vec<(i32, i32, i32, TerrainHandle)> = Vec::new();
    let mut bridge_count = 0usize;
    let mut pillar_count = 0usize;

    for y in 0..OMAP_DIM {
        for x in 0..OMAP_DIM {
            let ground_handle = ground_grid[(y as usize) * omap_size + (x as usize)];
            let flags = registry.flags_for(ground_handle);

            if !flags.contains(TerrainFlags::BRIDGE) {
                continue;
            }

            // Place bridge road surface at z=1
            tile_writes.push((1, x, y, TerrainHandle(bridge_surface)));
            bridge_count += 1;

            // Scan downward for water column to place pillars
            for z_below in (0i32..=(-10i32)).rev() {
                let below_handle = ter_at_z(z_below, x, y);
                if below_handle == TerrainHandle::NULL {
                    break;
                }
                let below_flags = registry.flags_for(below_handle);

                if below_flags.intersects(water_flags) {
                    // Place support pillar
                    if let Some(pillar) = bridge_pillar {
                        tile_writes.push((z_below, x, y, TerrainHandle(pillar)));
                        pillar_count += 1;
                    }
                } else {
                    // Hit non-water — stop the pillar column
                    break;
                }
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        bridges = bridge_count,
        pillars = pillar_count,
        "generate_over: elevated terrain computed"
    );

    // --- Write to chunks ----------------------------------------------------
    flush_tile_writes(&chunks, &par_commands, &tile_writes);
}

// ---------------------------------------------------------------------------
// Helper: flush tile writes to chunks via par_iter (multi-z aware)
// ---------------------------------------------------------------------------

fn flush_tile_writes(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: &ParallelCommands,
    tile_writes: &[(i32, i32, i32, TerrainHandle)], // (z, x, y, handle)
) {
    if tile_writes.is_empty() {
        return;
    }
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z = chunk_pos.z.0 as i32;

        let local_ox = (chunk_pos.chunk_x as i32) * (CHUNK_DIM as i32);
        let local_oy = (chunk_pos.chunk_y as i32) * (CHUNK_DIM as i32);

        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wz, wx, wy, handle) in tile_writes {
            if wz != z {
                continue;
            }
            let lx = wx - local_ox;
            let ly = wy - local_oy;
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
}
