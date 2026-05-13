//! Step 4: Connect city centers with roads via MST + A\* pathfinding.
//!
//! Port of CDDA master's `overmap::place_roads()` (overmap.cpp L2168-2225) and
//! `overmap::build_connection()` (overmap.cpp L2563-2648).
//!
//! Uses `connect_closest_points` from `cdda_overmap::connections` to build
//! a minimum-spanning-tree network, then routes each connection with A\*
//! pathfinding (`greedy_path`) that avoids rivers, lakes, oceans, and other
//! impassable terrain.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, line_between, ConnectionType,
};
use cdda_overmap::pathfinding::{greedy_path, DirectedNode, NodeScore};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

// ---------------------------------------------------------------------------
// Scoring helpers
// ---------------------------------------------------------------------------

/// Return a terrain handle at OMT coordinates (global 0..180).
fn get_terrain_at(
    chunks: &Query<(&ChunkPosition, &mut OvermapChunk)>,
    x: i32,
    y: i32,
    z: i8,
) -> TerrainHandle {
    for (chunk_pos, chunk) in chunks {
        if chunk_pos.z.0 != z {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = x - ox;
        let ly = y - oy;
        if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
            return chunk.get(lx as u8, ly as u8);
        }
    }
    TerrainHandle::NULL
}

/// Score a candidate road node for A\* pathfinding.
///
/// Returns lower costs for existing roads and fields, higher costs for
/// forests and swamps, and rejects water, ravines, and impassable terrain.
fn score_road_node(
    node: DirectedNode,
    _prev: Option<DirectedNode>,
    terrain_grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
) -> NodeScore {
    let (x, y) = node.pos;
    if x < 0 || x >= 180 || y < 0 || y >= 180 {
        return NodeScore::REJECTED;
    }

    let handle = TerrainHandle(terrain_grid[x as usize][y as usize]);
    let flags = registry.flags_for(handle);

    // Reject water, ravines, highways, impassable.
    if flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
        || flags.contains(TerrainFlags::HIGHWAY)
        || flags.contains(TerrainFlags::IMPASSABLE)
    {
        return NodeScore::REJECTED;
    }

    // Cost by terrain type (lower = preferred).
    let ct = handle.type_index();
    let base_cost = if flags.contains(TerrainFlags::ROAD) || flags.contains(TerrainFlags::BRIDGE) {
        0 // existing road / bridge is free
    } else if ct == registry.field_index {
        2 // open field
    } else if ct == registry.forest_index
        || ct == registry.forest_thick_index
        || ct == registry.forest_water_index
    {
        5 // forest
    } else {
        3 // default — light vegetation / other passable
    };

    NodeScore::new(base_cost, 0)
}

// ---------------------------------------------------------------------------
// Build helpers
// ---------------------------------------------------------------------------

/// Mark every tile in `path` as a road in `road_grid`.
fn build_connection_from_path(path: &[DirectedNode], road_grid: &mut [[bool; 180]]) {
    for node in path {
        let (x, y) = node.pos;
        if inbounds_omt((x, y)) {
            road_grid[x as usize][y as usize] = true;
        }
    }
}

/// Return `true` if `handle` is water (river, lake, ocean).
fn is_water(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

// ---------------------------------------------------------------------------
// place_roads system
// ---------------------------------------------------------------------------

/// Place inter-city roads using MST + A\* pathfinding.
///
/// # Algorithm (port of `overmap::place_roads`)
///
/// 1. Generate 2–3 border exit points (avoiding rivers) for cross-overmap
///    road continuity.
/// 2. Collect `road_points`: exit points + city centers (or a fallback center).
/// 3. Build a dense terrain grid from chunk data for the scoring function.
/// 4. Call `connect_closest_points` to build an MST-based road network.
///    The build function uses `greedy_path` for A\* routing around obstacles.
/// 5. Write road tiles back to chunks with correct NS/EW/intersection rotation.
pub fn place_roads(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.place_roads {
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 3);

    // ---- 1. Generate border exit points ----
    let mut roads_out: Vec<(i32, i32)> = Vec::new();

    // Try N/E, S/W pairs for at least 3 exits, avoiding rivers.
    // Shuffle direction pairs.
    let mut dir_groups = [(0, 1), (2, 3)]; // (North, East), (South, West)
    for i in (1..dir_groups.len()).rev() {
        let j = rng.range_i32(0, i as i32) as usize;
        dir_groups.swap(i, j);
    }

    for &(d1, d2) in &dir_groups {
        if roads_out.len() >= 3 {
            break;
        }
        for &edge in &[d1, d2] {
            if roads_out.len() >= 3 {
                break;
            }
            // Try several positions along this edge.
            for _ in 0..50 {
                let coord = rng.range_i32(10, OMAP_DIM - 11);
                let p = match edge {
                    0 => (coord, 0),            // North
                    1 => (OMAP_DIM - 1, coord), // East
                    2 => (coord, OMAP_DIM - 1), // South
                    3 => (0, coord),            // West
                    _ => continue,
                };
                let handle = get_terrain_at(&chunks, p.0, p.1, 0);
                if !is_water(handle, &registry) {
                    roads_out.push(p);
                    break;
                }
            }
        }
    }

    // ---- 2. Assemble road_points ----
    let city_centers: Vec<(i32, i32)> = cities.iter().map(|c| (c.omt_x, c.omt_y)).collect();

    let mut road_points: Vec<(i32, i32)> = Vec::new();
    road_points.extend_from_slice(&roads_out);

    if city_centers.is_empty() {
        // Fallback: random central point.
        let fx = rng.range_i32(OMAP_DIM / 4, 3 * OMAP_DIM / 4);
        let fy = rng.range_i32(OMAP_DIM / 4, 3 * OMAP_DIM / 4);
        road_points.push((fx, fy));
    } else {
        road_points.extend_from_slice(&city_centers);
    }

    if road_points.len() < 2 {
        return;
    }

    // ---- 3. Build dense terrain grid for the scoring function ----
    let mut terrain_grid = [[0u32; 180]; 180];
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
                    terrain_grid[gx][gy] = chunk.get(lx, ly).0;
                }
            }
        }
    }

    // ---- 4. Build road network via MST + A\* ----
    let mut road_grid = [[false; 180]; 180];

    // Mark city center tiles as road.
    for &(x, y) in &road_points {
        if inbounds_omt((x, y)) {
            road_grid[x as usize][y as usize] = true;
        }
    }

    connect_closest_points(
        &road_points,
        0,
        ConnectionType::InterCityRoad,
        &mut rng,
        |from, to, z, _ct| {
            if z != 0 {
                return;
            }

            // Build the A\* scoring function for this connection.
            let scoring_fn = {
                let terrain_grid = &terrain_grid;
                let registry = &*registry;
                move |node: DirectedNode, prev: Option<DirectedNode>| {
                    score_road_node(node, prev, terrain_grid, registry)
                }
            };

            let path = greedy_path(from, to, (OMAP_DIM, OMAP_DIM), &scoring_fn);

            if !path.is_empty() {
                build_connection_from_path(&path, &mut road_grid);
            } else {
                // Fallback: straight Bresenham line when A\* fails
                // (e.g. all paths blocked).
                let line = line_between(from, to);
                for &(x, y) in &line {
                    if inbounds_omt((x, y)) {
                        road_grid[x as usize][y as usize] = true;
                    }
                }
            }
        },
    );

    // ---- 5. Write road grid back to chunks ----
    let road_ns = registry
        .handle_by_id("road_ns")
        .unwrap_or(TerrainHandle::NULL);
    let road_ew = registry
        .handle_by_id("road_ew")
        .unwrap_or(TerrainHandle::NULL);
    let road_nesw = registry
        .handle_by_id("road_nesw")
        .unwrap_or(TerrainHandle::NULL);

    let field_index = registry.field_index;

    for (chunk_pos, mut chunk) in &mut chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= 180 || gy >= 180 || !road_grid[gx][gy] {
                    continue;
                }

                let current = chunk.get(lx, ly);
                let ct = current.type_index();

                // Never overwrite water.
                if is_water(current, &registry) {
                    continue;
                }

                // Only overwrite field, forest types, and existing roads.
                let is_field = ct == field_index;
                let is_forest = ct == registry.forest_index
                    || ct == registry.forest_thick_index
                    || ct == registry.forest_water_index;
                let is_road = ct == road_ns.type_index()
                    || ct == road_ew.type_index()
                    || ct == road_nesw.type_index();

                if !is_field && !is_forest && !is_road {
                    continue;
                }

                // Determine road orientation from neighbours in the road grid.
                let north = gy > 0 && road_grid[gx][gy - 1];
                let south = gy + 1 < 180 && road_grid[gx][gy + 1];
                let east = gx + 1 < 180 && road_grid[gx + 1][gy];
                let west = gx > 0 && road_grid[gx - 1][gy];

                let has_ns = north || south;
                let has_ew = east || west;

                let handle = if has_ns && has_ew {
                    road_nesw // intersection / curve
                } else if has_ew {
                    road_ew // horizontal road
                } else {
                    road_ns // vertical road (default for endpoints)
                };
                chunk.set(lx, ly, handle);
            }
        }
    }

    info!(
        "Roads placed: {} road points, {} exit points for overmap ({}, {})",
        road_points.len(),
        roads_out.len(),
        config.om_x,
        config.om_y
    );
}
