//! Step 4c: Place forest trails by flood-filling contiguous forest regions
//! and connecting interior points via `connect_closest_points`.
//!
//! Port of CDDA master's `overmap::place_forest_trails()` (overmap.cpp L1875-1998).
//!
//! # Algorithm
//!
//! 1. Iterate over every OMT tile. If it's a forest tile and not yet visited,
//!    flood-fill to find the contiguous forest region.
//! 2. Skip regions smaller than `forest_trail_min_size`.
//! 3. Random chance (`forest_trail_chance`) to create trails in this forest.
//! 4. Find N/S/E/W extrema and an approximate center of the region.
//! 5. Pick random interior points proportional to the forest size.
//! 6. Optionally include border (extrema) points.
//! 7. Call `connect_closest_points` with `ConnectionType::ForestTrail`.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, point_flood_fill_4, square_dist, ConnectionType,
};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_overmap::Rng;
use tracing::info;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check whether a terrain type index corresponds to a forest-type tile.
fn is_forest_index(ct: u32, registry: &TerrainRegistry) -> bool {
    ct == registry.forest_index
        || ct == registry.forest_thick_index
        || ct == registry.forest_water_index
}

// ---------------------------------------------------------------------------
// build_trail_segment — places trail tiles along the line between two points
// ---------------------------------------------------------------------------

/// Place trail tiles along the straight-line path from `from` to `to`.
fn build_trail_segment(
    from: (i32, i32),
    to: (i32, i32),
    z: i32,
    _connection_type: ConnectionType,
    _registry: &TerrainRegistry,
    grid: &mut [[bool; 180]],
) {
    if z != 0 {
        return;
    }
    let path = cdda_overmap::connections::line_between(from, to);
    for &(x, y) in &path {
        if !inbounds_omt((x, y)) {
            continue;
        }
        grid[x as usize][y as usize] = true;
    }
}

// ---------------------------------------------------------------------------
// place_forest_trails system
// ---------------------------------------------------------------------------

/// Place forest trails by flood-filling contiguous forest regions and
/// connecting interior points via MST-based pathfinding.
///
/// # Algorithm (port of `overmap::place_forest_trails`)
///
/// 1. Flood-fill every unvisited forest tile to find contiguous forest regions.
/// 2. Skip regions smaller than `forest_trail_min_size`.
/// 3. `one_in(forest_trail_chance)` chance to build trails in this forest.
/// 4. Find N/S/E/W extrema and approximate center of the region.
/// 5. Pick random interior points proportional to forest size.
/// 6. Optionally include border (extrema) points.
/// 7. Call `connect_closest_points` with `ConnectionType::ForestTrail`.
pub fn place_forest_trails(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 11);

    // Get the trail terrain handles.
    let trail_handle = registry
        .handle_by_id("forest_trail")
        .unwrap_or(TerrainHandle::NULL);
    let trail_ns_handle = registry.rotate(trail_handle, 0);
    let trail_ew_handle = registry.rotate(trail_handle, 1);
    let trail_nesw_handle = registry
        .handle_by_id("forest_trail_nesw")
        .unwrap_or_else(|| registry.rotate(trail_handle, 3));

    // --- Build a dense 180×180 grid of terrain type indices ---
    // This avoids dual borrow issues: we read all terrain once, then
    // flood-fill from the grid, then write back to chunks.
    let mut terrain_grid = [[0u32; 180]; 180];
    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..32 {
            for lx in 0u8..32 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    terrain_grid[gx][gy] = chunk.terrain[idx].type_index();
                }
            }
        }
    }

    let mut visited = [[false; 180]; 180];
    let mut trail_grid = [[false; 180]; 180];
    let bounds = (0i32, 0i32, OMAP_DIM, OMAP_DIM);

    let mut region_count = 0usize;
    let mut trail_count = 0usize;

    for gy in 0i32..180 {
        for gx in 0i32..180 {
            if visited[gx as usize][gy as usize] {
                continue;
            }
            if !inbounds_omt((gx, gy))
                || !is_forest_index(terrain_grid[gx as usize][gy as usize], &registry)
            {
                continue;
            }

            // Flood-fill to find the contiguous forest region.
            let forest_points = point_flood_fill_4((gx, gy), bounds, |p| {
                is_forest_index(terrain_grid[p.0 as usize][p.1 as usize], &registry)
            });

            // Mark all points in this region as visited.
            for &(fx, fy) in &forest_points {
                visited[fx as usize][fy as usize] = true;
            }

            region_count += 1;

            // Skip regions that are too small.
            if forest_points.len() < settings.forest_trail_min_size {
                continue;
            }

            // Random chance to skip this forest.
            if !rng.one_in(settings.forest_trail_chance) {
                continue;
            }

            // Find N/S/E/W extrema.
            let northmost: (i32, i32) = *forest_points
                .iter()
                .min_by_key(|p| p.1)
                .unwrap_or(&(gx, gy));
            let southmost: (i32, i32) = *forest_points
                .iter()
                .max_by_key(|p| p.1)
                .unwrap_or(&(gx, gy));
            let westmost: (i32, i32) = *forest_points
                .iter()
                .min_by_key(|p| p.0)
                .unwrap_or(&(gx, gy));
            let eastmost: (i32, i32) = *forest_points
                .iter()
                .max_by_key(|p| p.0)
                .unwrap_or(&(gx, gy));

            // Approximate center.
            let center = (
                westmost.0 + (eastmost.0 - westmost.0) / 2,
                northmost.1 + (southmost.1 - northmost.1) / 2,
            );

            // Find the actual point in the forest closest to the center.
            let actual_center = *forest_points
                .iter()
                .min_by(|&&a, &&b| square_dist(a, center).cmp(&square_dist(b, center)))
                .unwrap_or(&(gx, gy));

            // Determine how many random points to add.
            let max_random_points = (settings.forest_trail_random_point_min
                + forest_points.len() as i32 / settings.forest_trail_random_point_size_scalar)
                .min(settings.forest_trail_random_point_max);

            // Start with the center point.
            let mut chosen_points: Vec<(i32, i32)> = Vec::new();
            chosen_points.push(actual_center);

            // Add random points from the forest.
            let mut shuffled = forest_points.clone();
            for i in (1..shuffled.len()).rev() {
                let j = rng.random_usize(i + 1);
                shuffled.swap(i, j);
            }
            let mut random_count = 0i32;
            for &random_point in &shuffled {
                if random_count >= max_random_points {
                    break;
                }
                random_count += 1;
                // Avoid duplicates.
                if random_point != actual_center {
                    chosen_points.push(random_point);
                }
            }

            // Optionally include border points.
            if settings.forest_trail_border_point_chance > 0
                && rng.one_in(settings.forest_trail_border_point_chance)
            {
                chosen_points.push(northmost);
            }
            if settings.forest_trail_border_point_chance > 0
                && rng.one_in(settings.forest_trail_border_point_chance)
            {
                chosen_points.push(southmost);
            }
            if settings.forest_trail_border_point_chance > 0
                && rng.one_in(settings.forest_trail_border_point_chance)
            {
                chosen_points.push(westmost);
            }
            if settings.forest_trail_border_point_chance > 0
                && rng.one_in(settings.forest_trail_border_point_chance)
            {
                chosen_points.push(eastmost);
            }

            // Deduplicate.
            chosen_points.sort();
            chosen_points.dedup();

            tracing::trace!(
                "Forest region at ({}, {}) size={}: {} chosen trail points",
                gx,
                gy,
                forest_points.len(),
                chosen_points.len()
            );

            if chosen_points.len() < 2 {
                continue;
            }

            // Build trail network within this forest.
            connect_closest_points(
                &chosen_points,
                0,
                ConnectionType::ForestTrail,
                &mut rng,
                |from, to, z, ct| {
                    build_trail_segment(from, to, z, ct, &registry, &mut trail_grid);
                },
            );

            trail_count += 1;
        }
    }

    // --- Write trail grid back to chunks ---
    // Only overwrite forest-type tiles (forest, forest_thick, forest_water).
    let forest_index = registry.forest_index;
    let forest_thick_index = registry.forest_thick_index;
    let forest_water_index = registry.forest_water_index;
    let trail_ns_idx = trail_ns_handle;
    let trail_ew_idx = trail_ew_handle;
    let trail_nesw_idx = trail_nesw_handle;

    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..32 {
            for lx in 0u8..32 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= 180 || gy >= 180 || !trail_grid[gx][gy] {
                    continue;
                }

                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let ct = chunk.terrain[idx].type_index();

                // Only overwrite forest-type tiles.
                if ct != forest_index && ct != forest_thick_index && ct != forest_water_index {
                    continue;
                }

                // Determine trail tile orientation based on neighbors.
                let north = gy > 0 && trail_grid[gx][gy - 1];
                let south = gy + 1 < 180 && trail_grid[gx][gy + 1];
                let east = gx + 1 < 180 && trail_grid[gx + 1][gy];
                let west = gx > 0 && trail_grid[gx - 1][gy];

                let has_ns = north || south;
                let has_ew = east || west;

                let handle = if has_ns && has_ew {
                    trail_nesw_idx
                } else if has_ew {
                    trail_ew_idx
                } else {
                    trail_ns_idx
                };
                new_terrain[idx] = handle;
                modified = true;
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
        "Forest trails placed: {} regions, {} trails built for overmap ({}, {})",
        region_count, trail_count, config.om_x, config.om_y
    );
}
