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
use crate::steps::neighbor_connections::ConnectionExits;
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
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    x: i32,
    y: i32,
    z: i8,
) -> TerrainHandle {
    for (_entity, chunk_pos, chunk) in chunks {
        if chunk_pos.z.0 != z {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = x - ox;
        let ly = y - oy;
        if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
            let idx = ly as usize * CHUNK_DIM + lx as usize;
            return chunk.terrain[idx];
        }
    }
    TerrainHandle::NULL
}

/// Score a candidate road node for A\* pathfinding.
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

    if flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
        || flags.contains(TerrainFlags::HIGHWAY)
        || flags.contains(TerrainFlags::IMPASSABLE)
    {
        return NodeScore::REJECTED;
    }

    let ct = handle.type_index();
    let base_cost = if flags.contains(TerrainFlags::ROAD) || flags.contains(TerrainFlags::BRIDGE) {
        0
    } else if ct == registry.field_index {
        2
    } else if ct == registry.forest_index
        || ct == registry.forest_thick_index
        || ct == registry.forest_water_index
    {
        5
    } else {
        3
    };

    NodeScore::new(base_cost, 0)
}

// ---------------------------------------------------------------------------
// Build helpers
// ---------------------------------------------------------------------------

fn build_connection_from_path(path: &[DirectedNode], road_grid: &mut [[bool; 180]]) {
    for node in path {
        let (x, y) = node.pos;
        if inbounds_omt((x, y)) {
            road_grid[x as usize][y as usize] = true;
        }
    }
}

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
/// Reads `ConnectionExits` (populated by `populate_connections_out_from_neighbors`)
/// for cross-overmap road continuity. Falls back to random exit generation
/// if the resource is absent.
pub fn place_roads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
    exits: Option<Res<ConnectionExits>>,
) {
    let op_city_size = settings.city_size;
    if op_city_size <= 0 || !settings.place_roads {
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 3);

    // ---- 1. Get city centers ----
    let mut city_centers: Vec<(i32, i32)> = Vec::new();
    for city in &cities {
        city_centers.push((city.omt_x, city.omt_y));
    }
    if city_centers.is_empty() {
        return;
    }

    // ---- 2. Generate border exit points ----
    let mut roads_out: Vec<(i32, i32)> = Vec::new();

    // Prefer deterministic neighbor exits (cross-overmap continuity).
    if let Some(ref exits_res) = exits {
        for &p in &exits_res.all() {
            let handle = get_terrain_at(&chunks, p.0, p.1, 0);
            if !is_water(handle, &registry) {
                roads_out.push(p);
            }
        }
    }

    // Fallback: random exit generation on non-water border tiles.
    if roads_out.is_empty() {
        let mut dir_groups = [(0, 1), (2, 3)];
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
                for _ in 0..50 {
                    let coord = rng.range_i32(10, OMAP_DIM - 11);
                    let p = match edge {
                        0 => (coord, 0),
                        1 => (OMAP_DIM - 1, coord),
                        2 => (coord, OMAP_DIM - 1),
                        3 => (0, coord),
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
    }

    // ---- 3. Assemble road_points ----
    let mut road_points: Vec<(i32, i32)> = Vec::new();
    road_points.extend_from_slice(&roads_out);
    road_points.extend_from_slice(&city_centers);

    if road_points.len() < 2 {
        return;
    }

    // ---- 4. Build dense terrain grid ----
    let mut terrain_grid = [[0u32; 180]; 180];
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
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    terrain_grid[gx][gy] = chunk.terrain[idx].0;
                }
            }
        }
    }

    // ---- 5. Build road network via MST + A\* ----
    let mut road_grid = [[false; 180]; 180];
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
                let line = line_between(from, to);
                for &(x, y) in &line {
                    if inbounds_omt((x, y)) {
                        road_grid[x as usize][y as usize] = true;
                    }
                }
            }
        },
    );

    // ---- 6. Write road grid back to chunks ----
    let road_ns = registry
        .handle_by_id("road_ns")
        .unwrap_or(TerrainHandle::NULL);
    let road_ew = registry
        .handle_by_id("road_ew")
        .unwrap_or(TerrainHandle::NULL);
    let road_nesw = registry
        .handle_by_id("road_nesw")
        .unwrap_or(TerrainHandle::NULL);

    // Bail out if road terrain handles are missing.
    if road_ns == TerrainHandle::NULL || road_ew == TerrainHandle::NULL {
        info!("Road terrain handles missing, skipping inter-city roads");
        return;
    }

    let field_index = registry.field_index;
    let forest_index = registry.forest_index;
    let forest_thick_index = registry.forest_thick_index;
    let forest_water_index = registry.forest_water_index;
    let road_ns_idx = road_ns.type_index();
    let road_ew_idx = road_ew.type_index();
    let road_nesw_idx = road_nesw.type_index();
    let reg = &*registry;

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
                if gx >= 180 || gy >= 180 || !road_grid[gx][gy] {
                    continue;
                }

                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let current = chunk.terrain[idx];
                let ct = current.type_index();

                if is_water(current, reg) {
                    continue;
                }

                let is_field = ct == field_index;
                let is_forest =
                    ct == forest_index || ct == forest_thick_index || ct == forest_water_index;
                let is_road = ct == road_ns_idx || ct == road_ew_idx || ct == road_nesw_idx;

                if !is_field && !is_forest && !is_road {
                    continue;
                }

                let north = gy > 0 && road_grid[gx][gy - 1];
                let south = gy + 1 < 180 && road_grid[gx][gy + 1];
                let east = gx + 1 < 180 && road_grid[gx + 1][gy];
                let west = gx > 0 && road_grid[gx - 1][gy];

                let has_ns = north || south;
                let has_ew = east || west;

                let handle = if has_ns && has_ew {
                    road_nesw
                } else if has_ew {
                    road_ew
                } else {
                    road_ns
                };
                new_terrain[idx] = handle;
                modified = true;
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
        "Roads placed: {} road points, {} exit points for overmap ({}, {})",
        road_points.len(),
        roads_out.len(),
        config.om_x,
        config.om_y
    );
}
