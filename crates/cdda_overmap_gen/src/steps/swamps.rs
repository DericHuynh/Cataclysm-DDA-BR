//! Swamp placement — verbatim port of C++ `overmap::place_swamps()`
//! (overmap.cpp L2111-2166).
//!
//! ## Algorithm
//!
//! 1. Build terrain grid from z=0 chunks, tracking river tiles.
//! 2. Build a floodplain array: for each river tile, buffer by a random radius
//!    in `[buffer_min, buffer_max]` using `closest_points_first`, incrementing a
//!    counter for every tile within Chebyshev radius.
//! 3. For each FOREST or FOREST_THICK tile:
//!    - `should_flood`: floodplain counter > 0 AND `!one_in(counter)` AND
//!      floodplain noise > `swamp_noise_threshold_adjacent`
//!    - `should_isolated_swamp`: floodplain noise > `swamp_noise_threshold_isolated`
//!      (uses the same noise layer as floodplain — C++ L2155)
//! 4. Place `forest_water` if either condition is true.

use bevy_ecs::prelude::*;
use cdda_sim::noise::floodplain_noise_at;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::connections::{closest_points_first, inbounds_omt};
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

/// Build a dense `[[u32; 180]; 180]` grid and a river-tile boolean mask from z=0 chunks.
fn build_omt_grid_with_rivers(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    registry: &TerrainRegistry,
) -> (
    [[u32; 180]; 180],
    [[bool; 180]; 180],
    Vec<(Entity, ChunkPosition)>,
) {
    let mut grid = [[0u32; 180]; 180];
    let mut is_river = [[false; 180]; 180];
    let mut z0_chunks: Vec<(Entity, ChunkPosition)> = Vec::with_capacity(36);

    for (entity, pos, chunk) in chunks.iter() {
        if pos.z.0 != 0 {
            continue;
        }
        z0_chunks.push((entity, *pos));

        let (origin_x, origin_y) = pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let omt_x = origin_x + lx as i32;
                let omt_y = origin_y + ly as i32;
                if omt_x >= 0 && omt_x < OMAP_DIM && omt_y >= 0 && omt_y < OMAP_DIM {
                    let handle = chunk.get(lx, ly);
                    grid[omt_y as usize][omt_x as usize] = handle.0;
                    if registry.flags_for(handle).contains(TerrainFlags::RIVER) {
                        is_river[omt_y as usize][omt_x as usize] = true;
                    }
                }
            }
        }
    }

    (grid, is_river, z0_chunks)
}

// ---------------------------------------------------------------------------
// place_swamps — system entry point
// ---------------------------------------------------------------------------

/// Place swamp (forest_water) terrain on the overmap.
///
/// Verbatim port of C++ `overmap::place_swamps()` (overmap.cpp L2111-2166).
pub fn place_swamps(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.overmap_forest || !settings.place_swamps {
        info!(
            "place_swamps: skipped — overmap_forest={} place_swamps={}",
            settings.overmap_forest, settings.place_swamps
        );
        return;
    }

    let settings_forest = &settings.forest;

    let buffer_distance_min = settings_forest.river_floodplain_buffer_distance_min;
    let buffer_distance_max = settings_forest.river_floodplain_buffer_distance_max;
    let swamp_adj_threshold = settings_forest.swamp_noise_threshold_adjacent;
    let swamp_iso_threshold = settings_forest.swamp_noise_threshold_isolated;

    let global_base_x = config.om_x * OMAP_DIM;
    let global_base_y = config.om_y * OMAP_DIM;

    // --- Build grid -----------------------------------------------------------
    let (mut grid, is_river, z0_chunks) = build_omt_grid_with_rivers(&chunks, &registry);

    let forest_raw = core_terrains.forest.0;
    let forest_thick_raw = core_terrains.forest_thick.0;
    let forest_water_raw = core_terrains.forest_water.0;

    // Seed RNG for buffer radius (C++ uses `rng(…)` with seed+3).
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 3);

    // --- Build floodplain — C++ L2119-2131 ------------------------------------
    // For each river tile, buffer by a random Chebyshev radius and increment
    // a counter for every tile within that buffer.
    let mut floodplain = [[0i32; 180]; 180];

    for y in 0..OMAP_DIM as usize {
        for x in 0..OMAP_DIM as usize {
            if !is_river[y][x] {
                continue;
            }

            let pos = (x as i32, y as i32);
            let radius = rng.range_i32(buffer_distance_min, buffer_distance_max);

            // closest_points_first returns all points within Chebyshev radius,
            // sorted by distance. We increment the counter for each in-bounds point.
            for p in closest_points_first(pos, radius) {
                if inbounds_omt(p) {
                    floodplain[p.1 as usize][p.0 as usize] += 1;
                }
            }
        }
    }

    // --- Place swamps — C++ L2135-2164 ----------------------------------------
    let mut swamp_count: usize = 0;

    for y in 0..OMAP_DIM as usize {
        for x in 0..OMAP_DIM as usize {
            let terrain = grid[y][x];

            // C++ L2142-2144: only consider existing forest tiles
            if terrain != forest_raw && terrain != forest_thick_raw {
                continue;
            }

            let fp_count = floodplain[y][x];

            let global_x = global_base_x + x as i32;
            let global_y = global_base_y + y as i32;
            let noise = floodplain_noise_at(global_x, global_y, config.noise_seed);

            // C++ L2149-2150: should_flood — adjacent to water body
            let should_flood = fp_count > 0 && !rng.one_in(fp_count) && noise > swamp_adj_threshold;

            // C++ L2154-2155: should_isolated_swamp — regardless of floodplain
            let should_isolated_swamp = noise > swamp_iso_threshold;

            // C++ L2156-2158
            if should_flood || should_isolated_swamp {
                grid[y][x] = forest_water_raw;
                swamp_count += 1;
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        swamps = swamp_count,
        "place_swamps: terrain computed"
    );

    // --- Write back to chunks --------------------------------------------------
    write_back_grid(&grid, &z0_chunks, &mut commands);
}

// ---------------------------------------------------------------------------
// Write-back
// ---------------------------------------------------------------------------

/// Write the modified grid back to z=0 chunk entities via `Commands`.
fn write_back_grid(
    grid: &[[u32; 180]; 180],
    z0_chunks: &[(Entity, ChunkPosition)],
    commands: &mut Commands,
) {
    for &(entity, pos) in z0_chunks {
        let (origin_x, origin_y) = pos.omt_origin();
        let mut new_terrain = [TerrainHandle::NULL; CHUNK_SIZE];
        let mut any_changed = false;

        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let omt_x = origin_x + lx as i32;
                let omt_y = origin_y + ly as i32;
                let idx = ly * CHUNK_DIM + lx;
                if omt_x >= 0 && omt_x < OMAP_DIM as i32 && omt_y >= 0 && omt_y < OMAP_DIM as i32 {
                    new_terrain[idx] = TerrainHandle(grid[omt_y as usize][omt_x as usize]);
                    any_changed = true;
                }
            }
        }

        if any_changed {
            commands.entity(entity).insert(OvermapChunk {
                terrain: Box::new(new_terrain),
            });
        }
    }
}
