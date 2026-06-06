//! Ocean placement — verbatim port of C++ `overmap::place_oceans()`
//! (overmap_water.cpp L402-545) and `calculate_ocean_gradient()` (L372-400).

use bevy_ecs::prelude::*;
use cdda_noise::ocean_noise_at;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::connections::{inbounds_omt, line_between, square_dist};
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use std::collections::{HashSet, VecDeque};
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::{OvermapRegionSettings, RegionSettingsOcean};

pub fn calculate_ocean_gradient(
    p: (i32, i32),
    om_x: i32,
    om_y: i32,
    settings: &RegionSettingsOcean,
) -> f32 {
    let northern_ocean = settings.ocean_start_north.unwrap_or(i32::MAX);
    let eastern_ocean = settings.ocean_start_east.unwrap_or(i32::MAX);
    let western_ocean = settings.ocean_start_west.unwrap_or(i32::MAX);
    let southern_ocean = settings.ocean_start_south.unwrap_or(i32::MAX);

    let mut ocean_adjust_n = 0.0f32;
    let mut ocean_adjust_e = 0.0f32;
    let mut ocean_adjust_w = 0.0f32;
    let mut ocean_adjust_s = 0.0f32;

    if om_y <= -northern_ocean {
        ocean_adjust_n =
            0.0005 * (OMAP_DIM - p.1 + ((om_y + northern_ocean) * OMAP_DIM).abs()) as f32;
    }
    if om_x >= eastern_ocean {
        ocean_adjust_e = 0.0005 * (p.0 + (om_x - eastern_ocean) * OMAP_DIM) as f32;
    }
    if om_x <= -western_ocean {
        ocean_adjust_w =
            0.0005 * (OMAP_DIM - p.0 + ((om_x + western_ocean) * OMAP_DIM).abs()) as f32;
    }
    if om_y >= southern_ocean {
        ocean_adjust_s = 0.0005 * (p.1 + (om_y - southern_ocean) * OMAP_DIM) as f32;
    }

    ocean_adjust_n
        .max(ocean_adjust_e)
        .max(ocean_adjust_w)
        .max(ocean_adjust_s)
}

fn connect_to_closest_river(
    connection_point: (i32, i32),
    grid: &mut [[u32; 180]; 180],
    is_river: &[[bool; 180]; 180],
    river_center_raw: u32,
) -> bool {
    let mut closest_distance: i32 = -1;
    let mut closest_point: Option<(i32, i32)> = None;

    for x in 0..OMAP_DIM {
        for y in 0..OMAP_DIM {
            if !is_river[y as usize][x as usize] {
                continue;
            }
            let dist = square_dist(connection_point, (x, y));
            if dist < closest_distance || closest_distance < 0 {
                closest_point = Some((x, y));
                closest_distance = dist;
            }
        }
    }

    if let Some(river_pt) = closest_point {
        if closest_distance > 0 {
            for pt in line_between(connection_point, river_pt) {
                if inbounds_omt(pt) {
                    grid[pt.1 as usize][pt.0 as usize] = river_center_raw;
                }
            }
            return true;
        }
    }
    false
}

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

pub fn place_oceans(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    let settings_ocean = &settings.ocean;
    if !settings.overmap_ocean {
        info!("place_oceans: skipped — overmap_ocean is false");
        return;
    }

    let _ocean_depth = settings_ocean.ocean_depth;
    let ocean_size_min = settings_ocean.ocean_size_min;
    let noise_threshold = settings_ocean.noise_threshold_ocean;

    let global_base_x = config.om_x * OMAP_DIM;
    let global_base_y = config.om_y * OMAP_DIM;

    let (mut grid, is_river, z0_chunks) = build_omt_grid_with_rivers(&chunks, &registry);

    let field_raw = core_terrains.field.0;
    let ocean_surface_raw = core_terrains.ocean.0;
    let ocean_shore_raw = core_terrains.lake_shore.0;
    let river_center_raw = core_terrains.river_center.0;

    let is_ocean = |x: i32, y: i32| -> bool {
        if x <= -6 || x >= OMAP_DIM + 5 || y <= -6 || y >= OMAP_DIM + 5 {
            return false;
        }
        let gradient = calculate_ocean_gradient((x, y), config.om_x, config.om_y, settings_ocean);
        if gradient == 0.0 {
            return false;
        }
        let global_x = global_base_x + x;
        let global_y = global_base_y + y;
        let noise = ocean_noise_at(global_x, global_y, config.noise_seed);
        noise + gradient > noise_threshold
    };

    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut ocean_count: usize = 0;
    let mut placed_river_connections: usize = 0;

    for i in 0..OMAP_DIM {
        for j in 0..OMAP_DIM {
            let seed = (i, j);
            if visited.contains(&seed) {
                continue;
            }

            // Seed must be in-bounds, field, and meet the ocean predicate.
            // Inlined to avoid borrowing `grid` across the mutation below.
            if seed.0 < 0 || seed.0 >= OMAP_DIM || seed.1 < 0 || seed.1 >= OMAP_DIM {
                continue;
            }
            if grid[seed.1 as usize][seed.0 as usize] != field_raw || !is_ocean(seed.0, seed.1) {
                continue;
            }

            let ocean_points = point_flood_fill_4_ext(seed, &mut visited, &is_ocean);

            if ocean_points.len() < ocean_size_min {
                continue;
            }

            let mut ocean_set: HashSet<(i32, i32)> = ocean_points.iter().copied().collect();

            for y in 0..OMAP_DIM as usize {
                for x in 0..OMAP_DIM as usize {
                    if is_river[y][x] {
                        ocean_set.insert((x as i32, y as i32));
                    }
                }
            }

            for &(lx, ly) in &ocean_points {
                if !inbounds_omt((lx, ly)) {
                    continue;
                }

                let mut shore = false;
                'outer: for ni in -1i32..=1 {
                    for nj in -1i32..=1 {
                        let n = (lx + ni, ly + nj);
                        if !ocean_set.contains(&n) {
                            shore = true;
                            break 'outer;
                        }
                    }
                }

                grid[ly as usize][lx as usize] = if shore {
                    ocean_shore_raw
                } else {
                    ocean_surface_raw
                };
            }

            let northmost = ocean_points
                .iter()
                .filter(|&&(ox, oy)| inbounds_omt((ox, oy)))
                .min_by_key(|&&(_ox, oy)| oy)
                .copied();
            let southmost = ocean_points
                .iter()
                .filter(|&&(ox, oy)| inbounds_omt((ox, oy)))
                .max_by_key(|&&(_ox, oy)| oy)
                .copied();

            if let Some(nm) = northmost {
                if connect_to_closest_river(nm, &mut grid, &is_river, river_center_raw) {
                    placed_river_connections += 1;
                }
            }
            if let Some(sm) = southmost {
                if connect_to_closest_river(sm, &mut grid, &is_river, river_center_raw) {
                    placed_river_connections += 1;
                }
            }

            ocean_count += 1;
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        oceans = ocean_count,
        river_connections = placed_river_connections,
        "place_oceans: terrain computed"
    );

    write_back_grid(&grid, &z0_chunks, &mut commands);
}

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
