//! Step 2d: Place swamps using floodplain buffering + noise.
//!
//! Port of CDDA master's `overmap::place_swamps()` (overmap.cpp L2111-2166).
//!
//! Swamps are placed on FOREST tiles adjacent to rivers (floodplain)
//! or as isolated marshland in low-lying areas.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainFlags, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_noise;
use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use tracing::info;

/// Place `forest_water` on FOREST tiles that meet swamp criteria.
///
/// Algorithm:
/// 1. Build a floodplain array: buffer each river tile by a random radius
///    in `[river_floodplain_buffer_dist_min, river_floodplain_buffer_dist_max]`,
///    incrementing a counter for every tile within radius.
/// 2. For each FOREST tile:
///    - `should_flood`: floodplain[x][y] > 0 && !one_in(floodplain[x][y])
///      && floodplain_noise > swamp_noise_threshold_adjacent
///    - `should_isolated_swamp`: forest_noise > swamp_noise_threshold_isolated
///    - Place `forest_water` if either condition is true.
pub fn place_swamps(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.place_swamps {
        info!("Swamps disabled for overmap ({}, {})", config.om_x, config.om_y);
        return;
    }

    let forest_index = registry.forest_index;
    let forest_thick_index = registry.forest_thick_index;
    let seed = config.noise_seed;
    let mut rng = XorShiftRng::new(seed as u64 + 3);

    // Phase 1: build a dense terrain array and floodplain buffer.
    let mut terrain = [[0u32; 180]; 180];
    let mut floodplain = [[0u32; 180]; 180];

    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    let h = chunk.get(lx, ly);
                    terrain[gx][gy] = h.type_index();
                }
            }
        }
    }

    // Buffer each river tile to build the floodplain.
    let buffer_min = settings.river_floodplain_buffer_dist_min;
    let buffer_max = settings.river_floodplain_buffer_dist_max;

    for x in 0..180 {
        for y in 0..180 {
            let handle = TerrainHandle::new(terrain[x][y], 0);
            if !registry.flags_for(handle).contains(TerrainFlags::RIVER) {
                continue;
            }
            let dist = rng.range_i32(buffer_min, buffer_max);
            if dist <= 0 {
                continue;
            }
            for dx in -dist..=dist {
                for dy in -dist..=dist {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < 180 && ny >= 0 && ny < 180 {
                        floodplain[nx as usize][ny as usize] += 1;
                    }
                }
            }
        }
    }

    // Phase 2: place swamp on qualifying FOREST tiles.
    let forest_water = registry
        .handle_by_id("forest_water")
        .unwrap_or(TerrainHandle::new(0, 0));
    let swamp_adj_threshold = settings.swamp_noise_threshold_adjacent;
    let swamp_iso_threshold = settings.swamp_noise_threshold_isolated;

    for (chunk_pos, mut chunk) in &mut chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let h = chunk.get(lx, ly);
                let ct = h.type_index();

                // Only consider FOREST or FOREST_THICK tiles.
                if ct != forest_index && ct != forest_thick_index {
                    continue;
                }

                let wx = ox + lx as i32;
                let wy = oy + ly as i32;
                if wx < 0 || wx >= 180 || wy < 0 || wy >= 180 {
                    continue;
                }
                let gx = wx as usize;
                let gy = wy as usize;

                let fp_val = floodplain[gx][gy];
                let should_flood = fp_val > 0
                    && !rng.one_in(fp_val as i32)
                    && cdda_noise::floodplain_noise_at(wx, wy, seed) > swamp_adj_threshold;

                let should_isolated =
                    cdda_noise::forest_noise_at(wx, wy, seed) > swamp_iso_threshold;

                if should_flood || should_isolated {
                    chunk.set(lx, ly, forest_water);
                }
            }
        }
    }

    info!(
        "Swamps placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
