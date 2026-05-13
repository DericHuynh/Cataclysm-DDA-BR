//! Step 2c: Place oceans using noise + gradient + flood-fill.
//!
//! Port of CDDA master's `overmap::place_oceans()` (overmap_water.cpp L402-545)
//! and `calculate_ocean_gradient()` (overmap_water.cpp L372-400).
//!
//! Oceans are placed at the edges of the overmap where the ocean gradient
//! combined with noise exceeds a threshold. The gradient is computed from
//! `OvermapRegionSettings::ocean_start` values.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_noise;
use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use std::collections::VecDeque;
use tracing::info;

/// Port of overmap::calculate_ocean_gradient() (C++ L372-400).
///
/// Computes an ocean density gradient from each cardinal edge of the
/// overmap where `ocean_start` is set. Higher values = more likely to
/// be ocean.
///
/// `ocean_start` is indexed by om_direction order: [North, East, South, West].
pub fn calculate_ocean_gradient(
    p: (i32, i32),
    om_x: i32,
    om_y: i32,
    settings: &OvermapRegionSettings,
) -> f32 {
    let northern = settings.ocean_start[0].unwrap_or(i32::MAX);
    let eastern = settings.ocean_start[1].unwrap_or(i32::MAX);
    let southern = settings.ocean_start[2].unwrap_or(i32::MAX);
    let western = settings.ocean_start[3].unwrap_or(i32::MAX);

    let mut adj_n = 0.0f32;
    let mut adj_e = 0.0f32;
    let mut adj_w = 0.0f32;
    let mut adj_s = 0.0f32;

    if om_y <= -northern {
        adj_n = 0.0005
            * (180.0 - p.1 as f32 + ((om_y + northern) * 180).unsigned_abs() as f32);
    }
    if om_x >= eastern {
        adj_e = 0.0005 * (p.0 as f32 + ((om_x - eastern) * 180) as f32);
    }
    if om_x <= -western {
        adj_w = 0.0005
            * (180.0 - p.0 as f32 + ((om_x + western) * 180).unsigned_abs() as f32);
    }
    if om_y >= southern {
        adj_s = 0.0005 * (p.1 as f32 + ((om_y - southern) * 180) as f32);
    }

    adj_n.max(adj_e).max(adj_w).max(adj_s)
}

/// Place OCEAN_SURFACE, OCEAN_SHORE, OCEAN_WATER_CUBE, and OCEAN_BED.
///
/// Uses ocean noise + gradient to identify candidate tiles, flood-fills
/// into clusters, skips clusters smaller than `ocean_size_min`, and then
/// populates the z-levels below.
pub fn place_oceans(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    // Check if oceans are enabled — at least one ocean_start must be Some.
    let oceans_enabled = settings.ocean_start.iter().any(|s| s.is_some());
    if !oceans_enabled {
        info!("Oceans disabled for overmap ({}, {}): all ocean_start are None", config.om_x, config.om_y);
        return;
    }

    let field_index = registry.field_index;
    let threshold = settings.ocean_noise_threshold;
    let min_size = settings.ocean_size_min;
    let seed = config.noise_seed;

    // Phase 1: build a boolean mask of ocean candidates.
    let mut ocean_mask = [[false; 180]; 180];
    let mut seeds: Vec<(usize, usize)> = Vec::new();

    // Read current terrain from all z=0 chunks into a dense array.
    let mut current = [[0u32; 180]; 180];
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
                    current[gx][gy] = chunk.get(lx, ly).type_index();
                }
            }
        }
    }

    // Find candidate ocean tiles: noise > threshold AND currently FIELD.
    for x in 0..180 {
        for y in 0..180 {
            if current[x][y] != field_index {
                continue;
            }
            let n = cdda_noise::ocean_noise_at(x as i32, y as i32, seed);
            let grad = calculate_ocean_gradient(
                (x as i32, y as i32),
                config.om_x,
                config.om_y,
                &settings,
            );
            if n + grad > threshold {
                ocean_mask[x][y] = true;
                seeds.push((x, y));
            }
        }
    }

    if seeds.is_empty() {
        info!("No ocean candidates for overmap ({}, {})", config.om_x, config.om_y);
        return;
    }

    // Phase 2: flood-fill 4-connected clusters.
    let mut visited = [[false; 180]; 180];
    let mut clusters: Vec<Vec<(usize, usize)>> = Vec::new();

    for &(sx, sy) in &seeds {
        if visited[sx][sy] {
            continue;
        }
        let mut cluster = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((sx, sy));
        visited[sx][sy] = true;

        while let Some((cx, cy)) = queue.pop_front() {
            cluster.push((cx, cy));
            for (nx, ny) in neighbors_4(cx, cy) {
                if !visited[nx][ny] && ocean_mask[nx][ny] {
                    visited[nx][ny] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        if cluster.len() >= min_size {
            clusters.push(cluster);
        }
    }

    if clusters.is_empty() {
        info!("No ocean clusters >= {} for overmap ({}, {})", min_size, config.om_x, config.om_y);
        return;
    }

    // Phase 3: build modified tile grids, then write back via par_iter.
    let ocean_surface = registry
        .handle_by_id("ocean_surface")
        .unwrap_or(TerrainHandle::new(0, 0));
    let ocean_shore = registry
        .handle_by_id("ocean_shore")
        .unwrap_or(TerrainHandle::new(0, 0));
    let ocean_water_cube = registry
        .handle_by_id("ocean_water_cube")
        .unwrap_or(TerrainHandle::new(0, 0));
    let ocean_bed = registry
        .handle_by_id("ocean_bed")
        .unwrap_or(TerrainHandle::new(0, 0));

    // Build a set of all ocean tiles for fast shore detection.
    let mut is_ocean = [[false; 180]; 180];
    for cluster in &clusters {
        for &(x, y) in cluster {
            is_ocean[x][y] = true;
        }
    }

    // Precompute shore status and surface handles.
    let mut surface_handles: [[Option<TerrainHandle>; 180]; 180] = [[None; 180]; 180];
    let mut is_shore: [[bool; 180]; 180] = [[false; 180]; 180];
    for cluster in &clusters {
        for &(x, y) in cluster {
            let shore = neighbors_8(x, y)
                .iter()
                .any(|&(nx, ny)| !is_ocean[nx][ny]);
            is_shore[x][y] = shore;
            surface_handles[x][y] = Some(if shore { ocean_shore } else { ocean_surface });
        }
    }

    let ocean_depth = settings.ocean_depth;

    // Phase 4: write back using par_iter across all z-levels.
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z = chunk_pos.z.0 as i32;
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= 180 || gy >= 180 { continue; }
                if !is_ocean[gx][gy] { continue; }

                let handle = if z == 0 {
                    surface_handles[gx][gy]
                } else if z > ocean_depth && z < 0 {
                    // Water cubes down to ocean_depth+1.
                    Some(ocean_water_cube)
                } else if z == ocean_depth && !is_shore[gx][gy] {
                    // Bed at ocean_depth, but only for non-shore tiles.
                    Some(ocean_bed)
                } else {
                    None
                };

                if let Some(new_handle) = handle {
                    let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
                    if new_terrain[idx] != new_handle {
                        new_terrain[idx] = new_handle;
                        modified = true;
                    }
                }
            }
        }

        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk { terrain: new_terrain });
            });
        }
    });

    info!(
        "Oceans placed: {} clusters for overmap ({}, {})",
        clusters.len(),
        config.om_x,
        config.om_y
    );
}


fn neighbors_4(x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut v = Vec::with_capacity(4);
    if x > 0 {
        v.push((x - 1, y));
    }
    if x + 1 < 180 {
        v.push((x + 1, y));
    }
    if y > 0 {
        v.push((x, y - 1));
    }
    if y + 1 < 180 {
        v.push((x, y + 1));
    }
    v
}

fn neighbors_8(x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut v = neighbors_4(x, y);
    if x > 0 && y > 0 {
        v.push((x - 1, y - 1));
    }
    if x + 1 < 180 && y > 0 {
        v.push((x + 1, y - 1));
    }
    if x > 0 && y + 1 < 180 {
        v.push((x - 1, y + 1));
    }
    if x + 1 < 180 && y + 1 < 180 {
        v.push((x + 1, y + 1));
    }
    v
}
