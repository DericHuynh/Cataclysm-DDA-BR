//! Step 2e: Place ravines using greedy paths + width expansion.
//!
//! Port of CDDA master's `overmap::place_ravines()` (overmap.cpp L2428-2501).

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::{BinaryHeap, HashSet};
use tracing::info;

// ---------------------------------------------------------------------------
// Greedy path with random costs — matches C++ pf::greedy_path
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
struct PathNode {
    pos: (i32, i32),
    cost: i32,
    heuristic: i32,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse: BinaryHeap is max-heap, we want min (cost+heuristic).
        (other.cost + other.heuristic).cmp(&(self.cost + self.heuristic))
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Generate a greedy path matching C++ `pf::greedy_path` with random costs.
///
/// Uses a priority queue with random costs (1 or 2, matching C++ `rng(1,2)`)
/// and Chebyshev-distance heuristic. This produces winding, natural-looking
/// ravines rather than the stair-step Manhattan paths.
fn greedy_path(
    from: (i32, i32),
    to: (i32, i32),
    bounds: (i32, i32),
    rng: &mut XorShiftRng,
) -> Vec<(i32, i32)> {
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut came_from: std::collections::HashMap<(i32, i32), (i32, i32)> =
        std::collections::HashMap::new();
    let mut heap = BinaryHeap::new();

    let h = |p: (i32, i32)| -> i32 { (p.0 - to.0).abs().max((p.1 - to.1).abs()) };

    heap.push(PathNode {
        pos: from,
        cost: 0,
        heuristic: h(from),
    });
    visited.insert(from);

    let dirs: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    while let Some(node) = heap.pop() {
        if node.pos == to {
            // Reconstruct path.
            let mut path = vec![to];
            let mut cur = to;
            while let Some(&prev) = came_from.get(&cur) {
                path.push(prev);
                cur = prev;
            }
            path.reverse();
            return path;
        }

        for &(dx, dy) in &dirs {
            let nx = node.pos.0 + dx;
            let ny = node.pos.1 + dy;
            if nx < 0 || nx >= bounds.0 || ny < 0 || ny >= bounds.1 {
                continue;
            }
            let np = (nx, ny);
            if visited.contains(&np) {
                continue;
            }
            visited.insert(np);
            came_from.insert(np, node.pos);
            // Random cost 1 or 2, matching C++ `rng(1, 2)`.
            let step_cost = rng.range_i32(1, 2);
            heap.push(PathNode {
                pos: np,
                cost: node.cost + step_cost,
                heuristic: h(np),
            });
        }
    }

    // Fallback: straight line.
    line_between(from, to)
}

// ---------------------------------------------------------------------------
// place_ravines
// ---------------------------------------------------------------------------

/// Place ravines and ravine edges on the overmap.
///
/// Algorithm (matching C++ overmap.cpp L2428-2501):
/// 1. For each ravine, pick random origin + offset, generate a random-cost
///    greedy path, and widen it to `ravine_width`.
/// 2. Classify each rift point as edge (any 8-neighbor NOT a rift) or interior.
/// 3. Place terrain at z=0, then propagate down to `ravine_depth`.
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

    // C++: [1 - ravine_width, ravine_width) — exclusive upper bound.
    let w_start = 1 - ravine_width;
    let w_end = ravine_width;
    let margin = ravine_width * 3;

    for _ in 0..num_ravines {
        let origin_x = rng.range_i32(margin, OMAP_DIM - margin - 1);
        let origin_y = rng.range_i32(margin, OMAP_DIM - margin - 1);

        let offset_x = rng.range_i32(-ravine_range, ravine_range);
        let offset_y = rng.range_i32(-ravine_range, ravine_range);

        let dest_x = (origin_x + offset_x).clamp(margin, OMAP_DIM - margin - 1);
        let dest_y = (origin_y + offset_y).clamp(margin, OMAP_DIM - margin - 1);

        let path = greedy_path(
            (origin_x, origin_y),
            (dest_x, dest_y),
            (OMAP_DIM, OMAP_DIM),
            &mut rng,
        );

        // Widen: C++ uses [1-w, w) range.
        for &(px, py) in &path {
            for dx in w_start..w_end {
                for dy in w_start..w_end {
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

    // Phase 2: classify rift vs edge.
    let mut is_ravine = [[false; 180]; 180];
    for &(x, y) in &rift_points {
        is_ravine[x as usize][y as usize] = true;
    }

    let mut surface_handles: [[Option<TerrainHandle>; 180]; 180] = [[None; 180]; 180];
    let mut is_edge: [[bool; 180]; 180] = [[false; 180]; 180];
    for &(x, y) in &rift_points {
        // C++: check 8 neighbors, break on first non-rift.
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

    // Phase 3: write back across all z-levels.
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
                    Some(if is_edge[gx][gy] {
                        ravine_edge_handle
                    } else {
                        ravine_handle
                    })
                } else if z == ravine_depth {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
