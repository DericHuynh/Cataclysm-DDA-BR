//! Step 2e: Place ravines using greedy paths + width expansion.
//!
//! Port of CDDA master's `overmap::place_ravines()` (overmap.cpp L2428-2501).
//!
//! Ravines are long, narrow fissures that cut through the terrain.
//! Each ravine is a greedy path from a random origin to a random offset,
//! widened to `ravine_width`, then propagated down through z-levels.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, OMAP_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use std::collections::HashSet;
use tracing::info;

/// Generate a greedy (Manhattan-biased) path from `from` to `to`.
///
/// At each step, randomly chooses between the X and Y axes when both
/// need to move, producing an L-shaped or stair-step path.
fn greedy_path(from: (i32, i32), to: (i32, i32), rng: &mut XorShiftRng) -> Vec<(i32, i32)> {
    let mut path = vec![from];
    let mut current = from;
    while current != to {
        let dx = (to.0 - current.0).signum();
        let dy = (to.1 - current.1).signum();
        // Randomly choose axis when both are non-zero.
        if dx != 0 && dy != 0 {
            if rng.range_i32(0, 1) == 0 {
                current.0 += dx;
            } else {
                current.1 += dy;
            }
        } else {
            current.0 += dx;
            current.1 += dy;
        }
        path.push(current);
    }
    path
}

/// Place ravines and ravine edges on the overmap.
///
/// Algorithm:
/// 1. For each ravine (num_ravines times):
///    - Pick random origin within bounds and random offset (ravine_range).
///    - Generate a greedy path from origin to origin+offset.
///    - For each path point, add all points within ravine_width to a set.
/// 2. For each rift point, check 8 neighbors for edges.
/// 3. Place ravine/ravine_edge at z=0, then propagate down through z-levels
///    to ravine_depth, placing ravine_floor/ravine_floor_edge at the bottom.
pub fn place_ravines(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let num_ravines = settings.ravine_num;
    if num_ravines == 0 {
        return;
    }

    let ravine_range = settings.ravine_range;
    let ravine_width = settings.ravine_width;
    let ravine_depth = settings.ravine_depth;
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 4);

    // Resolve terrain handles.
    let ravine_handle = registry
        .handle_by_id("ravine")
        .unwrap_or(TerrainHandle::new(0, 0));
    let ravine_edge_handle = registry
        .handle_by_id("ravine_edge")
        .unwrap_or(TerrainHandle::new(0, 0));
    let ravine_floor_handle = registry
        .handle_by_id("ravine_floor")
        .unwrap_or(TerrainHandle::new(0, 0));
    let ravine_floor_edge_handle = registry
        .handle_by_id("ravine_floor_edge")
        .unwrap_or(TerrainHandle::new(0, 0));

    // Phase 1: generate rift points.
    let mut rift_points: HashSet<(i32, i32)> = HashSet::new();

    let margin = ravine_width + 2;
    for _ in 0..num_ravines {
        let origin_x = rng.range_i32(margin, OMAP_DIM - margin - 1);
        let origin_y = rng.range_i32(margin, OMAP_DIM - margin - 1);

        let offset_x = rng.range_i32(-ravine_range, ravine_range);
        let offset_y = rng.range_i32(-ravine_range, ravine_range);

        let dest_x = (origin_x + offset_x).clamp(margin, OMAP_DIM - margin - 1);
        let dest_y = (origin_y + offset_y).clamp(margin, OMAP_DIM - margin - 1);

        let path = greedy_path((origin_x, origin_y), (dest_x, dest_y), &mut rng);

        // Widen the path: add all points within ravine_width of each path point.
        for &(px, py) in &path {
            for dx in -ravine_width..=ravine_width {
                for dy in -ravine_width..=ravine_width {
                    let nx = px + dx;
                    let ny = py + dy;
                    if nx >= 0 && nx < OMAP_DIM && ny >= 0 && ny < OMAP_DIM {
                        rift_points.insert((nx, ny));
                    }
                }
            }
        }
    }

    if rift_points.is_empty() {
        return;
    }

    // Phase 2: classify rift vs edge points, then place terrain at z=0.
    let mut is_ravine = [[false; 180]; 180];
    for &(x, y) in &rift_points {
        is_ravine[x as usize][y as usize] = true;
    }

    for &(x, y) in &rift_points {
        // Check 8 neighbors: if any is NOT in the rift set, this is an edge.
        let is_edge = neighbors_8(x, y)
            .iter()
            .any(|&(nx, ny)| !is_ravine[nx as usize][ny as usize]);

        let handle = if is_edge {
            ravine_edge_handle
        } else {
            ravine_handle
        };

        write_tile_to_chunks(&mut chunks, x, y, 0, handle);
    }

    // Phase 3: propagate down through z-levels.
    // z=-1 to ravine_depth+1: ravine / ravine_edge (same as surface).
    // z=ravine_depth: ravine_floor / ravine_floor_edge.
    for &(x, y) in &rift_points {
        let is_edge = neighbors_8(x, y)
            .iter()
            .any(|&(nx, ny)| !is_ravine[nx as usize][ny as usize]);

        for z in (ravine_depth + 1)..0 {
            let handle = if is_edge {
                ravine_edge_handle
            } else {
                ravine_handle
            };
            write_tile_to_chunks(&mut chunks, x, y, z, handle);
        }

        // Bottom floor.
        let floor_handle = if is_edge {
            ravine_floor_edge_handle
        } else {
            ravine_floor_handle
        };
        write_tile_to_chunks(&mut chunks, x, y, ravine_depth, floor_handle);
    }

    info!(
        "Ravines placed: {} rift points for overmap ({}, {})",
        rift_points.len(),
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

fn neighbors_8(x: i32, y: i32) -> Vec<(i32, i32)> {
    vec![
        (x - 1, y - 1),
        (x, y - 1),
        (x + 1, y - 1),
        (x - 1, y),
        (x + 1, y),
        (x - 1, y + 1),
        (x, y + 1),
        (x + 1, y + 1),
    ]
}
