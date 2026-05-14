//! Step 7b: Forest trailhead placement.
//!
//! Port of CDDA master's `overmap::place_forest_trailheads()`.
//!
//! # Algorithm
//!
//! 1. Scan all z=0 chunks for forest trail tiles (TerrainFlags::ROAD on forest tiles).
//! 2. Identify trail endpoints: trail tiles with exactly 1 adjacent trail tile.
//! 3. Place `forest_trailhead` terrain at each endpoint.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use tracing::info;

/// Place trailhead terrain at forest trail endpoints.
///
/// Scans z=0 terrain for road tiles within forests and marks
/// dead-end trail tiles (exactly one adjacent trail connection) as
/// trailheads, matching C++ `place_forest_trailheads()` behavior.
pub fn place_forest_trailheads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
) {
    // Resolve trailhead terrain handle.
    let trailhead = match registry.handle_by_id("forest_trailhead") {
        Some(h) => h,
        None => {
            info!("forest_trailhead not in registry — skipping trailhead placement");
            return;
        }
    };

    // --- Build a dense 180×180 grid of terrain type indices for z=0 ---
    let mut grid = [[0u32; 180]; 180];
    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    grid[gx][gy] = chunk.get(lx, ly).type_index();
                }
            }
        }
    }

    // --- Identify trail tiles: ROAD flag AND on forest-type terrain ---
    let forest_index = registry.forest_index;
    let forest_thick_index = registry.forest_thick_index;
    let forest_water_index = registry.forest_water_index;

    let is_forest_type = |ct: u32| -> bool {
        ct == forest_index || ct == forest_thick_index || ct == forest_water_index
    };

    // A tile is a "trail tile" if it has the ROAD flag and its terrain is
    // on forest-type ground (trails are roads placed over forest).
    let mut is_trail = [[false; 180]; 180];
    for x in 0..OMAP_DIM as usize {
        for y in 0..OMAP_DIM as usize {
            let handle = TerrainHandle::new(grid[x][y], 0);
            let flags = registry.flags_for(handle);
            is_trail[x][y] = flags.contains(TerrainFlags::ROAD) && is_forest_type(grid[x][y]);
        }
    }

    // --- Find endpoints: trail tiles with exactly 1 trail neighbor ---
    let mut trailhead_writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();

    for x in 1..OMAP_DIM as usize - 1 {
        for y in 1..OMAP_DIM as usize - 1 {
            if !is_trail[x][y] {
                continue;
            }

            let mut adj_trail = 0u8;
            for (dx, dy) in &[(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let nx = (x as i32 + dx) as usize;
                let ny = (y as i32 + dy) as usize;
                if is_trail[nx][ny] {
                    adj_trail += 1;
                }
            }

            // CDDA: trail endpoints have exactly 1 adjacent trail tile.
            if adj_trail == 1 {
                trailhead_writes.push((x as i32, y as i32, trailhead));
            }
        }
    }

    if trailhead_writes.is_empty() {
        return;
    }

    // --- Write trailhead terrain back to chunks via par_iter ---
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, handle) in &trailhead_writes {
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
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
        "Forest trailheads placed: {} endpoints",
        trailhead_writes.len()
    );
}
