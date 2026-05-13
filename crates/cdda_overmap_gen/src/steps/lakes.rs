//! Step 2b: Place lakes using noise + flood-fill clustering.
//!
//! Port of CDDA master's `overmap::place_lakes()`.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_noise;
use crate::pipeline::OvermapGenConfig;
use std::collections::VecDeque;
use tracing::info;

/// Place LAKE_SURFACE and LAKE_SHORE using noise threshold + flood-fill.
pub fn place_lakes(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
) {
    let field_index = registry.field_index;

    let threshold = 0.25; // lake noise threshold
    let min_size = 20usize;
    let seed = config.noise_seed;

    // Phase 1: build a boolean mask of lake candidates across the 180×180 overmap.
    // Since chunks are 32×32 and we need to flood-fill across chunk boundaries,
    // we build a temporary dense array.
    let mut lake_mask = [[false; 180]; 180];
    let mut seeds: Vec<(usize, usize)> = Vec::new();

    // Read current terrain from all z=0 chunks into the dense array.
    let mut current = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 { continue; }
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

    // Find candidate lake tiles: noise > threshold AND currently FIELD.
    for x in 0..180 {
        for y in 0..180 {
            if current[x][y] != field_index {
                continue;
            }
            let n = cdda_noise::lake_noise_at(x as i32, y as i32, seed);
            if n > threshold {
                lake_mask[x][y] = true;
                seeds.push((x, y));
            }
        }
    }

    if seeds.is_empty() {
        return;
    }

    // Phase 2: flood-fill to form clusters.
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
            // 4-connected neighbors
            for (nx, ny) in neighbors_4(cx, cy) {
                if !visited[nx][ny] && lake_mask[nx][ny] {
                    visited[nx][ny] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        if cluster.len() >= min_size {
            clusters.push(cluster);
        }
    }

    // Phase 3: write lake surface + shore tiles back to chunks.
    let lake_surface = registry.handle_by_id("lake_surface")
        .unwrap_or(TerrainHandle::new(0, 0));
    let lake_shore = registry.handle_by_id("lake_shore")
        .unwrap_or(TerrainHandle::new(0, 0));

    // Build a set of all lake tiles for fast shore detection.
    let mut is_lake = [[false; 180]; 180];
    for cluster in &clusters {
        for &(x, y) in cluster {
            is_lake[x][y] = true;
        }
    }

    for cluster in &clusters {
        for &(x, y) in cluster {
            // Check if any 8-neighbor is NOT a lake tile → shore.
            let shore = neighbors_8(x, y).iter().any(|&(nx, ny)| !is_lake[nx][ny]);
            let handle = if shore { lake_shore } else { lake_surface };

            // Write back to the appropriate chunk.
            for (chunk_pos, mut chunk) in &mut chunks {
                if chunk_pos.z.0 != 0 { continue; }
                let (ox, oy) = chunk_pos.omt_origin();
                let lx = x as i32 - ox;
                let ly = y as i32 - oy;
                if lx >= 0 && lx < 32 && ly >= 0 && ly < 32 {
                    chunk.set(lx as u8, ly as u8, handle);
                    break;
                }
            }
        }
    }

    info!(
        "Lakes placed: {} clusters for overmap ({}, {})",
        clusters.len(), config.om_x, config.om_y
    );
}

fn neighbors_4(x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut v = Vec::with_capacity(4);
    if x > 0 { v.push((x - 1, y)); }
    if x + 1 < 180 { v.push((x + 1, y)); }
    if y > 0 { v.push((x, y - 1)); }
    if y + 1 < 180 { v.push((x, y + 1)); }
    v
}

fn neighbors_8(x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut v = neighbors_4(x, y);
    if x > 0 && y > 0 { v.push((x - 1, y - 1)); }
    if x + 1 < 180 && y > 0 { v.push((x + 1, y - 1)); }
    if x > 0 && y + 1 < 180 { v.push((x - 1, y + 1)); }
    if x + 1 < 180 && y + 1 < 180 { v.push((x + 1, y + 1)); }
    v
}
