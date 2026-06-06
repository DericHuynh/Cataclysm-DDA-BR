//! Forest trailhead placement — marks trail endpoints near roads.
//!
//! Verbatim port of C++ `overmap::place_forest_trailheads()` (overmap.cpp L2000-2049).
//!
//! ## Algorithm
//!
//! 1. Early exit if `city_size <= 0`.
//! 2. Build terrain grid from z=0 chunks.
//! 3. For each tile where `is_ot_match("forest_trail_end", oter, prefix)`:
//!    a. `one_in(trailhead_chance)` check.
//!    b. Check if within `trailhead_road_distance` of a road (using closest_points_first).
//!    c. If both true: place `forest_trailhead` terrain at that position, preserved rotation.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::closest_points_first;
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

type OmtGrid = [[u32; OMAP_DIM as usize]; OMAP_DIM as usize];

fn build_omt_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
) -> (OmtGrid, Vec<(Entity, ChunkPosition)>) {
    let mut grid = [[0u32; OMAP_DIM as usize]; OMAP_DIM as usize];
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
                    grid[omt_y as usize][omt_x as usize] = chunk.get(lx, ly).0;
                }
            }
        }
    }

    (grid, z0_chunks)
}

// ---------------------------------------------------------------------------
// place_forest_trailheads — system entry point
// ---------------------------------------------------------------------------

/// Place forest trailheads at trail endpoints that are near roads.
///
/// Port of C++ `overmap::place_forest_trailheads()` (overmap.cpp L2000-2049).
pub fn place_forest_trailheads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    // Early exit — trailheads only make sense near cities
    if settings.city.city_size <= 0 {
        info!("place_forest_trailheads: skipped — city_size <= 0");
        return;
    }

    if !settings.forest_trail {
        info!("place_forest_trailheads: skipped — forest_trail=false");
        return;
    }

    let trail_settings = &settings.forest_trail_settings;
    info!("place_forest_trailheads: starting trailhead placement");

    // --- Build terrain grid --------------------------------------------------
    let (mut grid, _z0_chunks) = build_omt_grid(&chunks);
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 23);

    let trailhead_handle = registry.handle_by_id("forest_trailhead").map(|h| h.0);

    let Some(trailhead_raw) = trailhead_handle else {
        info!("place_forest_trailheads: no forest_trailhead terrain registered, skipping");
        return;
    };

    let mut placed = 0usize;

    // --- Scan all tiles for forest_trail_end ---------------------------------
    for y in 0..OMAP_DIM {
        for x in 0..OMAP_DIM {
            let handle = TerrainHandle(grid[y as usize][x as usize]);

            // Check if this is a forest_trail_end (prefix match)
            if !is_ot_match("forest_trail_end", handle, &registry, OtMatchType::Prefix) {
                continue;
            }

            // Random chance check
            if !rng.one_in(trail_settings.trailhead_chance) {
                continue;
            }

            // --- Check for nearby road ---------------------------------------
            let road_distance = trail_settings.trailhead_road_distance;
            let nearby_points = closest_points_first((x, y), road_distance);

            let mut near_road = false;
            for &pt in &nearby_points {
                if pt.0 < 0 || pt.0 >= OMAP_DIM || pt.1 < 0 || pt.1 >= OMAP_DIM {
                    continue;
                }
                let nh = TerrainHandle(grid[pt.1 as usize][pt.0 as usize]);
                if registry.flags_for(nh).contains(TerrainFlags::ROAD)
                    || registry.flags_for(nh).contains(TerrainFlags::HIGHWAY)
                {
                    near_road = true;
                    break;
                }
            }

            if !near_road {
                continue;
            }

            // --- Place forest_trailhead, preserving rotation -----------------
            let rotation = handle.rotation();
            let rotated_trailhead = if rotation != 0 {
                // Apply rotation to the trailhead terrain
                registry.rotate(TerrainHandle(trailhead_raw), rotation).0
            } else {
                trailhead_raw
            };

            grid[y as usize][x as usize] = rotated_trailhead;
            placed += 1;
        }
    }

    info!(
        trailheads_placed = placed,
        "place_forest_trailheads: complete"
    );

    // --- Write terrain changes back to chunks via par_iter --------------------
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let local_ox = (chunk_pos.chunk_x as i32) * (CHUNK_DIM as i32);
        let local_oy = (chunk_pos.chunk_y as i32) * (CHUNK_DIM as i32);

        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let wx = local_ox + lx as i32;
                let wy = local_oy + ly as i32;
                if wx >= 0 && wx < OMAP_DIM && wy >= 0 && wy < OMAP_DIM {
                    let idx = ly * CHUNK_DIM + lx;
                    let new_handle = TerrainHandle(grid[wy as usize][wx as usize]);
                    if new_terrain[idx] != new_handle && new_handle != TerrainHandle::NULL {
                        new_terrain[idx] = new_handle;
                        modified = true;
                    }
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
