//! Step 2e: Place ravines using greedy paths + width expansion.
//!
//! Port of CDDA master's `overmap::place_ravines()` (overmap.cpp L2428-2501).
//!
//! Ravines are long, narrow fissures that cut through the terrain.
//! Each ravine is a greedy path from a random origin to a random offset,
//! widened to `ravine_width`, then propagated down through z-levels.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
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
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
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

    // Phase 2: classify rift vs edge points, precompute handles.
    let mut is_ravine = [[false; 180]; 180];
    for &(x, y) in &rift_points {
        is_ravine[x as usize][y as usize] = true;
    }

    let mut surface_handles: [[Option<TerrainHandle>; 180]; 180] = [[None; 180]; 180];
    let mut is_edge: [[bool; 180]; 180] = [[false; 180]; 180];
    for &(x, y) in &rift_points {
        let edge = neighbors_8(x, y)
            .iter()
            .any(|&(nx, ny)| !is_ravine[nx as usize][ny as usize]);
        is_edge[x as usize][y as usize] = edge;
        surface_handles[x as usize][y as usize] = Some(if edge {
            ravine_edge_handle
        } else {
            ravine_handle
        });
    }

    // Phase 3: write back using par_iter across all z-levels.
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z = chunk_pos.z.0 as i32;
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
                if !is_ravine[gx][gy] {
                    continue;
                }

                let handle = if z == 0 {
                    surface_handles[gx][gy]
                } else if z > ravine_depth && z < 0 {
                    // z=-1 down to ravine_depth+1: same as surface.
                    Some(if is_edge[gx][gy] {
                        ravine_edge_handle
                    } else {
                        ravine_handle
                    })
                } else if z == ravine_depth {
                    // Bottom floor.
                    Some(if is_edge[gx][gy] {
                        ravine_floor_edge_handle
                    } else {
                        ravine_floor_handle
                    })
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
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: new_terrain,
                });
            });
        }
    });

    info!(
        "Ravines placed: {} rift points for overmap ({}, {})",
        rift_points.len(),
        config.om_x,
        config.om_y
    );
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
