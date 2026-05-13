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
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
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

    // Phase 3: write ocean surface + shore tiles at z=0.
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

    // Place surface + shore at z=0.
    for cluster in &clusters {
        for &(x, y) in cluster {
            // Check if any 8-neighbor is NOT an ocean tile → shore.
            let shore = neighbors_8(x, y)
                .iter()
                .any(|&(nx, ny)| !is_ocean[nx][ny]);
            let handle = if shore {
                ocean_shore
            } else {
                ocean_surface
            };

            write_tile_to_chunks(&mut chunks, x as i32, y as i32, 0, handle);
        }
    }

    // Phase 4: place z-levels below surface.
    // ocean_water_cube from z=-1 down to ocean_depth+1, ocean_bed at ocean_depth.
    let ocean_depth = settings.ocean_depth;
    for cluster in &clusters {
        for &(x, y) in cluster {
            let is_shore = neighbors_8(x, y)
                .iter()
                .any(|&(nx, ny)| !is_ocean[nx][ny]);

            if is_shore {
                // Shore: just water cubes down to ocean_depth, no special bed.
                for z in (ocean_depth + 1)..0 {
                    write_tile_to_chunks(&mut chunks, x as i32, y as i32, z, ocean_water_cube);
                }
            } else {
                // Deep ocean: water cubes down to ocean_depth+1, bed at ocean_depth.
                for z in (ocean_depth + 1)..0 {
                    write_tile_to_chunks(&mut chunks, x as i32, y as i32, z, ocean_water_cube);
                }
                write_tile_to_chunks(&mut chunks, x as i32, y as i32, ocean_depth, ocean_bed);
            }
        }
    }

    info!(
        "Oceans placed: {} clusters for overmap ({}, {})",
        clusters.len(),
        config.om_x,
        config.om_y
    );
}

/// Write a terrain handle to the chunk that contains the given OMT coordinate.
fn write_tile_to_chunks(
    chunks: &mut Query<(&ChunkPosition, &mut OvermapChunk)>,
    omt_x: i32,
    omt_y: i32,
    z: i32,
    handle: TerrainHandle,
) {
    for (chunk_pos, mut chunk) in chunks {
        if chunk_pos.z.0 != z as i8 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = omt_x - ox;
        let ly = omt_y - oy;
        if lx >= 0 && lx < 32 && ly >= 0 && ly < 32 {
            chunk.set(lx as u8, ly as u8, handle);
            break;
        }
    }
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
