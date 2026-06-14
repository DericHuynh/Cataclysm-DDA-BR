//! Lake placement — verbatim port of C++ `overmap::place_lakes()`
//! (overmap_water.cpp L259-370).
//!
//! ## Algorithm
//!
//! 1. Build terrain grid from z=0 chunks, collecting river tiles.
//! 2. Find lake candidates: noise > threshold AND currently field.
//! 3. Flood-fill 4-connected clusters; skip clusters below `lake_size_min`.
//! 4. Merge existing river tiles into each lake set (for seamless shores).
//! 5. Compute shores: any tile with an 8-neighbor NOT in the merged lake+rivers
//!    set is a shore.
//! 6. Connect each lake to the nearest river by placing `river_center` tiles
//!    along a straight line (Bresenham).
//! 7. Write `lake_surface` / `lake_shore` back to z=0 chunks.

use bevy_ecs::prelude::*;
use cdda_sim::noise::lake_noise_at;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::connections::{closest_points_first, inbounds_omt, line_between};
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::{HashSet, VecDeque};
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
// Place lakes — system entry point
// ---------------------------------------------------------------------------

/// Place lake terrain on the overmap.
///
/// Verbatim port of C++ `overmap::place_lakes()` (overmap_water.cpp L259-370).
pub fn place_lakes(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.overmap_lake {
        info!("place_lakes: skipped — overmap_lake is false");
        return;
    }

    let settings_lake = &settings.lake;
    let noise_threshold = settings_lake.noise_threshold_lake as f32;
    let lake_size_min = settings_lake.lake_size_min;
    let invert_lakes = settings_lake.invert_lakes;

    let global_base_x = config.om_x * OMAP_DIM;
    let global_base_y = config.om_y * OMAP_DIM;

    // --- Build grid -----------------------------------------------------------
    let (mut grid, is_river, z0_chunks) = build_omt_grid_with_rivers(&chunks, &registry);

    let field_raw = core_terrains.field.0;
    let lake_surface_raw = core_terrains.lake_surface.0;
    let lake_shore_raw = core_terrains.lake_shore.0;
    let river_center_raw = core_terrains.river_center.0;

    // --- Noise-based lake predicate -------------------------------------------
    // C++ L271-275 — extended bounds of [-5, OMAP_DIM+5) to handle edge lakes
    let is_lake_noise = |x: i32, y: i32| -> bool {
        let global_x = global_base_x + x;
        let global_y = global_base_y + y;
        let noise = lake_noise_at(global_x, global_y, config.noise_seed);
        let above_threshold = noise > noise_threshold;
        if invert_lakes {
            !above_threshold
        } else {
            above_threshold
        }
    };

    // Flood-fill predicate: in the extended bounds and noisy enough.
    // Extended bounds = [-5, OMAP_DIM+5) — C++ L268-270
    let flood_predicate = |x: i32, y: i32| -> bool {
        x > -5 && x < OMAP_DIM + 5 && y > -5 && y < OMAP_DIM + 5 && is_lake_noise(x, y)
    };

    // Global visited set across all lake cluster searches — C++ L278
    let mut visited: HashSet<(i32, i32)> = HashSet::new();

    // Track whether any river tiles exist — C++ `rivers.empty()` check at L318
    let has_rivers = is_river.iter().any(|row| row.iter().any(|&b| b));

    // Seed RNG for random lake point selection (uses seed+1 for lakes).
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 1);

    let mut lake_count: usize = 0;
    let mut placed_river_connections: usize = 0;

    // --- Flood-fill lakes -----------------------------------------------------
    for i in 0..OMAP_DIM {
        for j in 0..OMAP_DIM {
            let seed = (i, j);
            if visited.contains(&seed) {
                continue;
            }

            // Seed must be in-bounds, field terrain, and above noise threshold.
            // Inlined (not a closure) to avoid borrowing `grid` across the
            // mutation that follows below — C++ L284-288.
            if seed.0 < 0 || seed.0 >= OMAP_DIM || seed.1 < 0 || seed.1 >= OMAP_DIM {
                continue;
            }
            if grid[seed.1 as usize][seed.0 as usize] != field_raw || !is_lake_noise(seed.0, seed.1)
            {
                continue;
            }

            // 4-connected flood fill within extended bounds — C++ L290-291
            let lake_points = point_flood_fill_4_ext(seed, &mut visited, &flood_predicate);

            // Skip lakes below minimum size — C++ L294-298
            if lake_points.len() < lake_size_min {
                continue;
            }

            // Build lake set from flood fill points — C++ L303
            let mut lake_set: HashSet<(i32, i32)> = lake_points.iter().copied().collect();

            // --- Connect lake to nearest river — C++ L318-340 ------------------
            if has_rivers {
                // Pick a random point from the lake set
                let lake_pts: Vec<(i32, i32)> = lake_set.iter().copied().collect();
                let random_lake_point =
                    lake_pts[rng.range_i32(0, lake_pts.len() as i32 - 1) as usize];

                // Search for nearest river using closest_points_first
                let mut river_connection: Option<(i32, i32)> = None;
                for find_river in closest_points_first(random_lake_point, OMAP_DIM / 2) {
                    if inbounds_omt(find_river)
                        && is_river[find_river.1 as usize][find_river.0 as usize]
                    {
                        river_connection = Some(find_river);
                        break;
                    }
                }

                // Place river_center tiles along the straight line — C++ L334-339
                if let Some(river_pt) = river_connection {
                    for pt in line_between(random_lake_point, river_pt) {
                        if inbounds_omt(pt) {
                            grid[pt.1 as usize][pt.0 as usize] = river_center_raw;
                        }
                    }
                    placed_river_connections += 1;
                }
            }

            // --- Merge existing river tiles into lake set — C++ L342-349 --------
            for y in 0..OMAP_DIM as usize {
                for x in 0..OMAP_DIM as usize {
                    if is_river[y][x] {
                        lake_set.insert((x as i32, y as i32));
                    }
                }
            }

            // --- Compute shores and place terrain — C++ L355-370 ---------------
            for &(lx, ly) in &lake_points {
                if !inbounds_omt((lx, ly)) {
                    continue;
                }

                // 8-neighbor shore check — C++ L359-365
                let mut shore = false;
                'outer: for ni in -1i32..=1 {
                    for nj in -1i32..=1 {
                        let n = (lx + ni, ly + nj);
                        if !lake_set.contains(&n) {
                            shore = true;
                            break 'outer;
                        }
                    }
                }

                grid[ly as usize][lx as usize] = if shore {
                    lake_shore_raw
                } else {
                    lake_surface_raw
                };
            }

            lake_count += 1;
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        lakes = lake_count,
        river_connections = placed_river_connections,
        "place_lakes: terrain computed"
    );

    // --- Write back to chunks --------------------------------------------------
    write_back_grid(&grid, &z0_chunks, &mut commands);
}

// ---------------------------------------------------------------------------
// point_flood_fill_4_ext — flood fill with shared visited set
// ---------------------------------------------------------------------------

fn point_flood_fill_4_ext(
    start: (i32, i32),
    visited: &mut HashSet<(i32, i32)>,
    predicate: &impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32)> {
    const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    let mut result = Vec::new();
    let mut queue = VecDeque::new();

    if visited.contains(&start) || !predicate(start.0, start.1) {
        return result;
    }

    visited.insert(start);
    queue.push_back(start);

    while let Some(p) = queue.pop_front() {
        result.push(p);
        for (dx, dy) in DIRS {
            let np = (p.0 + dx, p.1 + dy);
            if !visited.contains(&np) && predicate(np.0, np.1) {
                visited.insert(np);
                queue.push_back(np);
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Write-back
// ---------------------------------------------------------------------------

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
