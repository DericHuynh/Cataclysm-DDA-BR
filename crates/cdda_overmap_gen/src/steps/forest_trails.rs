//! Step 4c: Place forest trails by flood-filling contiguous forest regions
//! and connecting interior points via `connect_closest_points`.
//!
//! Verbatim port of CDDA master's `overmap::place_forest_trails()` (overmap.cpp L1875-1998).
//!
//! # Algorithm
//!
//! 1. Build a dense `[[u32; 180]; 180]` terrain-grid from chunk entities at z=0.
//! 2. Iterate every OMT tile (`i`..`j`). If it's a forest tile and not yet visited,
//!    flood-fill (`point_flood_fill_4`) to find the contiguous forest region.
//! 3. Skip regions smaller than `forest_trail_min_size`.
//! 4. `one_in(forest_trail_chance)` — random skip.
//! 5. Find N/S/E/W extrema and compute the approximate centre of the region.
//! 6. Find the actual forest point closest to that centre.
//! 7. Pick random interior points proportional to the forest size (shuffle, cap).
//! 8. Optionally include border (extrema) points.
//! 9. Call `connect_closest_points` with `ConnectionType::ForestTrail`.
//! 10. Write trail tiles back to chunks, orienting NS/EW/NESW via neighbour checks.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt_margin, line_between, point_flood_fill_4, square_dist,
    ConnectionType,
};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_overmap::Rng;
use std::collections::HashSet;
use tracing::info;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check whether a terrain type index corresponds to a forest-type tile
/// (forest, forest_thick, or forest_water).
///
/// Port of the C++ `is_ot_match("forest", oter, ot_match_type::prefix)` check
/// combined with the `is_forest` predicate (overmap.cpp L1878-1882).
fn is_forest_index(idx: u32, registry: &TerrainRegistry) -> bool {
    idx == registry.forest_index
        || idx == registry.forest_thick_index
        || idx == registry.forest_water_index
}

// ---------------------------------------------------------------------------
// place_forest_trails system
// ---------------------------------------------------------------------------

/// Place forest trails by flood-filling contiguous forest regions and
/// connecting interior points via MST-based pathfinding.
///
/// Port of `overmap::place_forest_trails()` (overmap.cpp L1875-1998).
pub fn place_forest_trails(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 11);
    let forest_trail = settings; // alias matching C++ `forest_trail`

    // -----------------------------------------------------------------------
    // Build a dense [[u32; 180]; 180] terrain grid from chunk entities at z=0.
    // This avoids dual-borrow issues: we read all terrain once, then flood-fill
    // from the grid, then write back to chunks.
    // -----------------------------------------------------------------------
    let mut ter_grid = [[0u32; OMAP_DIM as usize]; OMAP_DIM as usize];
    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < OMAP_DIM as usize && gy < OMAP_DIM as usize {
                    let idx = ly * CHUNK_DIM + lx;
                    ter_grid[gx][gy] = chunk.terrain[idx].type_index();
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // C++: std::unordered_set<point_om_omt> visited;
    //      const auto is_forest = [&]( const point_om_omt & p ) {
    //          if( !inbounds( p, 1 ) ) { return false; }
    //          const oter_id current_terrain = ter( tripoint_om_omt( p, 0 ) );
    //          return current_terrain == oter_forest
    //              || current_terrain == oter_forest_thick
    //              || current_terrain == oter_forest_water;
    //      };
    // -----------------------------------------------------------------------
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut trail_grid = [[false; OMAP_DIM as usize]; OMAP_DIM as usize];

    let is_forest = |p: (i32, i32)| -> bool {
        if !inbounds_omt_margin(p, 1) {
            return false;
        }
        let idx = ter_grid[p.0 as usize][p.1 as usize];
        idx == registry.forest_index
            || idx == registry.forest_thick_index
            || idx == registry.forest_water_index
    };

    let bounds = (0i32, 0i32, OMAP_DIM, OMAP_DIM);

    // -----------------------------------------------------------------------
    // C++: for( int i = 0; i < OMAPX; i++ ) {
    //        for( int j = 0; j < OMAPY; j++ ) {
    // -----------------------------------------------------------------------
    for i in 0..OMAP_DIM {
        for j in 0..OMAP_DIM {
            // C++:     tripoint_om_omt seed_point( i, j, 0 );
            //          oter_id oter = ter( seed_point );
            //          if( !is_ot_match( "forest", oter, ot_match_type::prefix ) ) { continue; }
            //          if( visited.find( seed_point.xy() ) != visited.end() ) { continue; }
            let seed_point: (i32, i32) = (i, j);
            let oter = ter_grid[i as usize][j as usize];
            if !is_forest_index(oter, &registry) {
                continue;
            }
            if visited.contains(&seed_point) {
                continue;
            }

            // C++:     std::vector<point_om_omt> forest_points =
            //             ff::point_flood_fill_4_connected<std::vector>(
            //                 seed_point.xy(), visited, is_forest );
            let forest_points = point_flood_fill_4(seed_point, bounds, &is_forest);

            // C++'s point_flood_fill_4_connected updates `visited` internally
            // during the flood fill.  Rust's `point_flood_fill_4` does not, so
            // we mark all returned points as visited afterwards.
            for &p in &forest_points {
                visited.insert(p);
            }

            // C++:     if( forest_points.empty() ||
            //             forest_points.size() <
            //                 static_cast<size_t>( forest_trail.minimum_forest_size ) ) {
            //             continue;
            //         }
            if forest_points.is_empty()
                || forest_points.len() < forest_trail.forest_trail_min_size
            {
                continue;
            }

            // C++:     if( !one_in( forest_trail.chance ) ) { continue; }
            if !rng.one_in(forest_trail.forest_trail_chance) {
                continue;
            }

            // C++:     auto north_south_most = std::minmax_element(
            //             forest_points.begin(), forest_points.end(),
            //             []( const point_om_omt & lhs, const point_om_omt & rhs ) {
            //                 return lhs.y() < rhs.y();
            //             } );
            //          auto west_east_most = std::minmax_element(
            //             forest_points.begin(), forest_points.end(),
            //             []( const point_om_omt & lhs, const point_om_omt & rhs ) {
            //                 return lhs.x() < rhs.x();
            //             } );
            let northmost = *forest_points
                .iter()
                .min_by_key(|p| p.1)
                .unwrap_or(&seed_point);
            let southmost = *forest_points
                .iter()
                .max_by_key(|p| p.1)
                .unwrap_or(&seed_point);
            let westmost = *forest_points
                .iter()
                .min_by_key(|p| p.0)
                .unwrap_or(&seed_point);
            let eastmost = *forest_points
                .iter()
                .max_by_key(|p| p.0)
                .unwrap_or(&seed_point);

            // C++:     point_om_omt center(
            //             westmost.x() + ( eastmost.x() - westmost.x() ) / 2,
            //             northmost.y() + ( southmost.y() - northmost.y() ) / 2 );
            let center = (
                westmost.0 + (eastmost.0 - westmost.0) / 2,
                northmost.1 + (southmost.1 - northmost.1) / 2,
            );
            let center_point = center;

            // C++:     point_om_omt actual_center_point =
            //             *std::min_element(
            //                 forest_points.begin(), forest_points.end(),
            //                 [&center_point]( const point_om_omt & lhs,
            //                                  const point_om_omt & rhs ) {
            //                     return square_dist( lhs, center_point )
            //                          < square_dist( rhs, center_point );
            //                 } );
            let actual_center_point = *forest_points
                .iter()
                .min_by(|&&lhs, &&rhs| {
                    square_dist(lhs, center_point).cmp(&square_dist(rhs, center_point))
                })
                .unwrap_or(&seed_point);

            // C++:     int max_random_points = forest_trail.random_point_min
            //             + forest_points.size() / forest_trail.random_point_size_scalar;
            //          max_random_points = std::min(
            //             max_random_points, forest_trail.random_point_max );
            let max_random_points = (forest_trail.forest_trail_random_point_min
                + forest_points.len() as i32 / forest_trail.forest_trail_random_point_size_scalar)
                .min(forest_trail.forest_trail_random_point_max);

            // C++:     std::vector<point_om_omt> chosen_points = { actual_center_point };
            let mut chosen_points: Vec<(i32, i32)> = vec![actual_center_point];

            // C++:     int random_point_count = 0;
            //          std::shuffle( forest_points.begin(), forest_points.end(),
            //                        rng_get_engine() );
            //          for( const auto &random_point : forest_points ) {
            //              if( random_point_count >= max_random_points ) { break; }
            //              random_point_count++;
            //              chosen_points.emplace_back( random_point );
            //          }
            let mut random_point_count = 0i32;
            let mut shuffled = forest_points.clone();
            let n = shuffled.len();
            // Fisher-Yates shuffle using the RNG
            for ii in (1..n).rev() {
                let jj = rng.random_usize(ii + 1);
                shuffled.swap(ii, jj);
            }
            for &random_point in &shuffled {
                if random_point_count >= max_random_points {
                    break;
                }
                random_point_count += 1;
                chosen_points.push(random_point);
            }

            // C++:     if( one_in( forest_trail.border_point_chance ) ) {
            //               chosen_points.emplace_back( northmost );
            //           }
            if rng.one_in(forest_trail.forest_trail_border_point_chance) {
                chosen_points.push(northmost);
            }
            if rng.one_in(forest_trail.forest_trail_border_point_chance) {
                chosen_points.push(southmost);
            }
            if rng.one_in(forest_trail.forest_trail_border_point_chance) {
                chosen_points.push(westmost);
            }
            if rng.one_in(forest_trail.forest_trail_border_point_chance) {
                chosen_points.push(eastmost);
            }

            // C++:     const overmap_connection_id &overmap_connection_forest_trail =
            //                settings->overmap_connection.trail_connection;
            //          connect_closest_points( chosen_points, 0,
            //                                  *overmap_connection_forest_trail );
            connect_closest_points(
                &chosen_points,
                0,
                ConnectionType::ForestTrail,
                &mut rng,
                |from, to, _z, _ct| {
                    // The C++ connection's build() places trail tiles directly.
                    // Here we mark a trail_grid so we can write the terrain
                    // back into chunks in a second pass.
                    let path = line_between(from, to);
                    for &(px, py) in &path {
                        if px >= 0 && px < OMAP_DIM && py >= 0 && py < OMAP_DIM {
                            trail_grid[px as usize][py as usize] = true;
                        }
                    }
                },
            );
        }
    }

    // -------------------------------------------------------------------
    // Write trail grid back to chunks.
    // Only overwrite forest-type tiles (forest, forest_thick, forest_water).
    // Orient trail tiles NS/EW/NESW based on neighbour connectivity.
    // -------------------------------------------------------------------
    let forest_idx = registry.forest_index;
    let forest_thick_idx = registry.forest_thick_index;
    let forest_water_idx = registry.forest_water_index;

    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= OMAP_DIM as usize || gy >= OMAP_DIM as usize || !trail_grid[gx][gy] {
                    continue;
                }

                let idx = ly * CHUNK_DIM + lx;
                let ct = chunk.terrain[idx].type_index();

                // Only overwrite forest-type tiles.
                if ct != forest_idx && ct != forest_thick_idx && ct != forest_water_idx {
                    continue;
                }

                // Determine trail tile orientation based on neighbours.
                let north = gy > 0 && trail_grid[gx][gy - 1];
                let south = gy + 1 < OMAP_DIM as usize && trail_grid[gx][gy + 1];
                let east = gx + 1 < OMAP_DIM as usize && trail_grid[gx + 1][gy];
                let west = gx > 0 && trail_grid[gx - 1][gy];

                let has_ns = north || south;
                let has_ew = east || west;

                let (handle, rotation) = if has_ns && has_ew {
                    // Crossroads tile
                    if let Some(h) = registry.handle_by_id("forest_trail_nesw") {
                        (h, 0u8)
                    } else if let Some(h) = registry.handle_by_id("forest_trail") {
                        (h, 0u8)
                    } else {
                        continue;
                    }
                } else if has_ew {
                    // East–west running trail
                    if let Some(h) = registry.handle_by_id("forest_trail") {
                        (h, 1u8)
                    } else {
                        continue;
                    }
                } else {
                    // North–south running trail (or isolated single tile)
                    if let Some(h) = registry.handle_by_id("forest_trail") {
                        (h, 0u8)
                    } else {
                        continue;
                    }
                };
                new_terrain[idx] = registry.rotate(handle, rotation);
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
        "Forest trails placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
