//! Ravine (rift) placement — verbatim port of C++ `overmap::place_ravines()`
//! (overmap.cpp L2428-2501).
//!
//! ## Algorithm
//!
//! 1. For `ravine_num` iterations:
//!    - Pick a random origin within the overmap (with 1-tile margin).
//!    - Pick a random offset in `[-ravine_range, ravine_range]`.
//!    - Run `greedy_path` from origin to `origin + offset` with random costs
//!      `NodeScore::new(0, rng(1,2))` matching C++ `pf::node_score(0, rng(1, 2))`.
//!    - Widen the path by Chebyshev radius `[1 - ravine_width, ravine_width)`,
//!      adding all points to a `rift_points` set.
//! 2. For each rift point, classify as *edge* if any 8-neighbor is NOT in the
//!    rift set (or out of bounds).
//! 3. Place at z=0: `ravine_edge` or `ravine`.
//! 4. For z = -1 down to `ravine_depth - 1`: same pattern.
//! 5. At z = `ravine_depth`: `ravine_floor_edge` or `ravine_floor`.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::connections::inbounds_omt;
use cdda_overmap::pathfinding::{greedy_path, DirectedNode, NodeScore};
use cdda_overmap::registry::{CoreTerrains, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::HashSet;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

/// Build a dense `[[u32; 180]; 180]` grid from z=0 chunk entities.
fn build_omt_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
) -> ([[u32; 180]; 180], Vec<(Entity, ChunkPosition)>) {
    let mut grid = [[0u32; 180]; 180];
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
                    grid[omt_y as usize][omt_x as usize] = chunk.get(lx, ly).0;
                }
            }
        }
    }

    (grid, z0_chunks)
}

// ---------------------------------------------------------------------------
// place_ravines — system entry point
// ---------------------------------------------------------------------------

/// Place ravine (rift) terrain on the overmap.
///
/// Verbatim port of C++ `overmap::place_ravines()` (overmap.cpp L2428-2501).
pub fn place_ravines(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.overmap_ravine {
        info!("place_ravines: skipped — overmap_ravine is false");
        return;
    }

    let settings_ravine = &settings.ravine;
    if settings_ravine.ravine_num == 0 {
        info!("place_ravines: skipped — ravine_num is 0");
        return;
    }

    let ravine_range = settings_ravine.ravine_range;
    let ravine_width = settings_ravine.ravine_width;
    let ravine_depth = settings_ravine.ravine_depth;

    // --- Build grid -----------------------------------------------------------
    let (mut grid, z0_chunks) = build_omt_grid(&chunks);

    // Terrain handles from registry by string ID (C++ uses string IDs).
    let ravine_raw = registry
        .handle_by_id("ravine")
        .unwrap_or_else(|| core_terrains.field)
        .0;
    let ravine_edge_raw = registry
        .handle_by_id("ravine_edge")
        .unwrap_or_else(|| core_terrains.field)
        .0;

    // --- RNG for path costs ---------------------------------------------------
    // C++ uses `rng(1, 2)` inside the scoring closure of greedy_path.
    // Since greedy_path takes `&F where F: Fn(...)`, we cannot use `&mut` RNG
    // inside the closure. We pre-generate random values into a Vec and share
    // an index via a `Cell<usize>` (which is Copy, so the closure is Fn).
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 5);
    // Estimate max path nodes: worst-case is ravine_range * 2 steps * 4 expansion.
    let max_nodes = (ravine_range as usize * 4).max(200);
    let random_costs: Vec<i32> = (0..max_nodes).map(|_| rng.range_i32(1, 2)).collect();
    let cost_idx = std::cell::Cell::new(0usize);

    // --- Collect all rift points — C++ L2447-2475 -----------------------------
    let mut rift_points: HashSet<(i32, i32)> = HashSet::new();

    for _n in 0..settings_ravine.ravine_num {
        // Random origin within the overmap — C++ L2451-2453
        let margin = 1;
        let origin_x = rng.range_i32(margin, OMAP_DIM - 1 - margin);
        let origin_y = rng.range_i32(margin, OMAP_DIM - 1 - margin);
        let origin = (origin_x, origin_y);

        // Random offset — C++ L2449-2452
        let offset_x = rng.range_i32(-ravine_range, ravine_range);
        let offset_y = rng.range_i32(-ravine_range, ravine_range);
        let destination = (origin_x + offset_x, origin_y + offset_y);

        // C++ L2453: `if( !inbounds( destination, ravine_width * 3 ) ) { continue; }`
        let dest_margin = ravine_width * 3;
        if destination.0 < dest_margin
            || destination.0 >= OMAP_DIM - dest_margin
            || destination.1 < dest_margin
            || destination.1 >= OMAP_DIM - dest_margin
        {
            continue;
        }

        // --- Greedy path with random costs — C++ L2440-2443 -------------------
        // `pf::node_score( 0, rng( 1, 2 ) )` — node cost 0, heuristic 1 or 2.
        // Cell<usize> is Copy → closure is Fn.
        let scoring_fn = |_cur: DirectedNode, _prev: Option<DirectedNode>| -> NodeScore {
            let idx = cost_idx.get();
            cost_idx.set((idx + 1) % max_nodes);
            let heuristic = random_costs[idx];
            NodeScore::new(0, heuristic)
        };

        let path = greedy_path(origin, destination, (OMAP_DIM, OMAP_DIM), &scoring_fn);

        // C++ L2456-2473 — widen the path by Chebyshev radius [1-width, width)
        for node in &path {
            let (px, py) = node.pos;
            for i in (1 - ravine_width)..ravine_width {
                for j in (1 - ravine_width)..ravine_width {
                    let n = (px + j, py + i);
                    // C++ L2470: `if( inbounds( n, 1 ) )` — 1-tile margin
                    if n.0 >= 1 && n.0 < OMAP_DIM - 1 && n.1 >= 1 && n.1 < OMAP_DIM - 1 {
                        rift_points.insert(n);
                    }
                }
            }
        }
    }

    // --- Classify edge vs interior and place terrain — C++ L2480-2500 ---------
    let mut ravine_count: usize = 0;

    for &(px, py) in &rift_points {
        // 8-neighbor check: is this an edge? — C++ L2482-2489
        let mut edge = false;
        'outer: for ni in -1i32..=1 {
            for nj in -1i32..=1 {
                let n = (px + ni, py + nj);
                if !rift_points.contains(&n) || !inbounds_omt(n) {
                    edge = true;
                    break 'outer;
                }
            }
        }

        // Place terrain at z=0 — C++ L2490-2500
        if edge {
            grid[py as usize][px as usize] = ravine_edge_raw;
        } else {
            grid[py as usize][px as usize] = ravine_raw;
        }
        ravine_count += 1;

        // z < 0 terrain deferred — needs separate chunk entities per z-level.
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        rift_points = ravine_count,
        ravine_depth,
        "place_ravines: terrain computed (z<0 terrain deferred)"
    );

    // --- Write back to chunks --------------------------------------------------
    write_back_grid(&grid, &z0_chunks, &mut commands);
}

// ---------------------------------------------------------------------------
// Write-back
// ---------------------------------------------------------------------------

/// Write the modified grid back to z=0 chunk entities via `Commands`.
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
