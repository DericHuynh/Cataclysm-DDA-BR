//! Step 2b: Place lakes using noise + flood-fill clustering.
//!
//! Port of CDDA master's `overmap::place_lakes()` (overmap_water.cpp L259-370).

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_noise;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use std::collections::VecDeque;
use tracing::info;

/// Place LAKE_SURFACE and LAKE_SHORE using noise threshold + flood-fill.
///
/// Algorithm (matching C++ overmap_water.cpp L259-370):
/// 1. Identify lake candidate tiles (noise > threshold, only on FIELD).
/// 2. Flood-fill 4-connected clusters, skip clusters below min_size.
/// 3. Merge existing river tiles into each lake set (so shore detection
///    at river confluences produces seamless lake surface).
/// 4. Compute shores: any tile with an 8-neighbor NOT in the merged
///    lake+rivers set is a shore.
/// 5. Connect each lake to the nearest river (if any exist) by placing
///    river_center tiles along a straight line.
pub fn place_lakes(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let field_index = registry.field_index;
    let threshold = settings.lake_noise_threshold;
    let min_size = settings.lake_size_min;
    let invert = settings.invert_lakes;
    let seed = config.noise_seed;

    // Phase 1: build a boolean mask of lake candidates.
    let mut lake_mask = [[false; 180]; 180];
    let mut seeds: Vec<(usize, usize)> = Vec::new();

    // Read current terrain + identify existing river tiles.
    let mut current = [[0u32; 180]; 180];
    let mut is_river = [[false; 180]; 180];
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
                    let handle = chunk.get(lx, ly);
                    current[gx][gy] = handle.type_index();
                    if registry.flags_for(handle).contains(TerrainFlags::RIVER) {
                        is_river[gx][gy] = true;
                    }
                }
            }
        }
    }

    // Find candidate lake tiles: noise > threshold AND currently FIELD.
    // invert_lakes XOR flips sense: when true, lakes go where noise is LOW.
    for x in 0..180 {
        for y in 0..180 {
            if current[x][y] != field_index {
                continue;
            }
            let n = cdda_noise::lake_noise_at(x as i32, y as i32, seed);
            let is_candidate = if invert {
                n <= threshold
            } else {
                n > threshold
            };
            if is_candidate {
                lake_mask[x][y] = true;
                seeds.push((x, y));
            }
        }
    }

    if seeds.is_empty() {
        return;
    }

    // Phase 2: flood-fill 4-connected to form clusters.
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

    if clusters.is_empty() {
        return;
    }

    // Phase 3: resolve handles and build merged lake+rivers set.
    let lake_surface = registry
        .handle_by_id("lake_surface")
        .unwrap_or(TerrainHandle::new(0, 0));
    let lake_shore = registry
        .handle_by_id("lake_shore")
        .unwrap_or(TerrainHandle::new(0, 0));
    let river_center = registry
        .handle_by_id("river_center")
        .unwrap_or(TerrainHandle::NULL);

    // Build per-cluster merged sets (lake points + any overlapping river).
    let mut lake_tiles: [[Option<TerrainHandle>; 180]; 180] = [[None; 180]; 180];
    let mut lake_river_connections: Vec<((usize, usize), (usize, usize))> = Vec::new();

    for cluster in &clusters {
        // --- Merge river tiles into the lake set (DRIFT #3 fix) ---
        let mut merged_set = [[false; 180]; 180];
        for &(x, y) in cluster {
            merged_set[x][y] = true;
        }
        for x in 0..180 {
            for y in 0..180 {
                if is_river[x][y] {
                    merged_set[x][y] = true;
                }
            }
        }

        // --- River connection (DRIFT #2 fix) ---
        // Pick a random lake-interior point and find nearest river.
        if river_center != TerrainHandle::NULL && !cluster.is_empty() {
            // Use a midpoint of the cluster as the connection point.
            let mut sum_x = 0usize;
            let mut sum_y = 0usize;
            for &(x, y) in cluster {
                sum_x += x;
                sum_y += y;
            }
            let lake_pt = (sum_x / cluster.len(), sum_y / cluster.len());

            // Find nearest river tile by Chebyshev distance.
            let mut nearest_river: Option<(usize, usize)> = None;
            let mut nearest_dist = i32::MAX;
            for x in 0..180 {
                for y in 0..180 {
                    if is_river[x][y] {
                        let dist = ((x as i32 - lake_pt.0 as i32).abs())
                            .max((y as i32 - lake_pt.1 as i32).abs());
                        if dist < nearest_dist {
                            nearest_dist = dist;
                            nearest_river = Some((x, y));
                        }
                    }
                }
            }
            if let Some(river_pt) = nearest_river {
                lake_river_connections.push((lake_pt, river_pt));
            }
        }

        // --- Compute shores against the merged set ---
        for &(x, y) in cluster {
            let shore = neighbors_8(x, y)
                .iter()
                .any(|&(nx, ny)| !merged_set[nx][ny]);
            lake_tiles[x][y] = Some(if shore { lake_shore } else { lake_surface });
        }
    }

    // Phase 4: draw river connections as river_center tiles.
    for &((lx, ly), (rx, ry)) in &lake_river_connections {
        let line = line_between((lx as i32, ly as i32), (rx as i32, ry as i32));
        for &(px, py) in &line {
            if px >= 0 && px < OMAP_DIM && py >= 0 && py < OMAP_DIM {
                let ux = px as usize;
                let uy = py as usize;
                // Only place river_center on non-lake, non-ocean tiles.
                if lake_tiles[ux][uy].is_none() && !is_river[ux][uy] {
                    // Don't overwrite ocean.
                    let handle = TerrainHandle::new(current[ux][uy], 0);
                    if !registry.flags_for(handle).contains(TerrainFlags::OCEAN) {
                        lake_tiles[ux][uy] = Some(river_center);
                        is_river[ux][uy] = true;
                    }
                }
            }
        }
    }

    // Phase 5: write back via par_iter.
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= 180 || gy >= 180 {
                    continue;
                }
                if let Some(new_handle) = lake_tiles[gx][gy] {
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
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: new_terrain,
                });
            });
        }
    });

    info!(
        "Lakes placed: {} clusters, {} river connections for overmap ({}, {})",
        clusters.len(),
        lake_river_connections.len(),
        config.om_x,
        config.om_y
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Bresenham line between two integer points.
fn line_between(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    let (mut x, mut y) = from;
    let (x2, y2) = to;
    let dx = (x2 - x).abs();
    let dy = -(y2 - y).abs();
    let sx = if x < x2 { 1 } else { -1 };
    let sy = if y < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        pts.push((x, y));
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    pts
}
