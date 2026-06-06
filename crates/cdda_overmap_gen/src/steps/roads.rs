//! Inter-city road placement via minimum-spanning-tree pathfinding.
//!
//! Verbatim port of C++ `overmap::place_roads()` (overmap.cpp L2163-2208).
//!
//! ## Algorithm
//!
//! 1. Build a dense terrain grid from z=0 chunks.
//! 2. Collect border exit points from [`ConnectionExits`] (or generate fallback).
//! 3. Build `road_points`: exits + city centers.
//! 4. Call [`connect_closest_points`] with [`ConnectionType::InterCityRoad`].
//! 5. For each MST edge, run [`greedy_path`] + write road terrain.
//! 6. Write terrain changes back to chunks via `par_iter`.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, inbounds_omt_margin, line_between, ConnectionType,
};
use cdda_overmap::direction::{Rng, FOUR_ADJACENT_OFFSETS};
use cdda_overmap::pathfinding::{greedy_path, DirectedNode, NodeScore};
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use crate::steps::neighbor_connections::ConnectionExits;

// ---------------------------------------------------------------------------
// Line-segment constants
// ---------------------------------------------------------------------------

const LINE_N: u16 = 1;
const LINE_E: u16 = 2;
const LINE_S: u16 = 4;
const LINE_W: u16 = 8;

fn set_segment(line: u16, dir_idx: usize) -> u16 {
    line | (1u16 << dir_idx)
}

fn has_segment(line: u16, dir_idx: usize) -> bool {
    (line & (1u16 << dir_idx)) != 0
}

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

type OmtGrid = [[u32; OMAP_DIM as usize]; OMAP_DIM as usize];

fn build_omt_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
) -> (OmtGrid, Vec<(Entity, ChunkPosition)>) {
    let mut grid = [[0u32; OMAP_DIM as usize]; OMAP_DIM as usize];
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
// Terrain accessor
// ---------------------------------------------------------------------------

fn ter_at(grid: &OmtGrid, x: i32, y: i32) -> TerrainHandle {
    if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
        TerrainHandle(grid[y as usize][x as usize])
    } else {
        TerrainHandle::NULL
    }
}

// ---------------------------------------------------------------------------
// build_connection — write road terrain for a connection path
// ---------------------------------------------------------------------------

fn build_connection(
    grid: &mut OmtGrid,
    registry: &TerrainRegistry,
    core_terrains: &CoreTerrains,
    from: (i32, i32),
    to: (i32, i32),
    _z: i32,
    _conn_type: ConnectionType,
) {
    let road_ns = core_terrains.road_ns.0;
    let road_ew = core_terrains.road_ew.0;
    let road_nesw = core_terrains.road_nesw.0;

    let line = line_between(from, to);

    for &(x, y) in &line {
        if !inbounds_omt((x, y)) {
            continue;
        }
        let xu = x as usize;
        let yu = y as usize;

        // Determine road type from cardinal-neighbor connectivity
        let mut segments: u16 = 0;

        for (dir_idx, &(dx, dy)) in FOUR_ADJACENT_OFFSETS.iter().enumerate() {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || nx >= OMAP_DIM || ny < 0 || ny >= OMAP_DIM {
                continue;
            }
            let nh = TerrainHandle(grid[ny as usize][nx as usize]);
            let nflags = registry.flags_for(nh);
            if nflags.contains(TerrainFlags::ROAD) || nflags.contains(TerrainFlags::HIGHWAY) {
                segments = set_segment(segments, dir_idx);
            }
        }

        // Also check the line directionality
        let idx_in_line = line.iter().position(|&p| p == (x, y)).unwrap_or(0);
        if idx_in_line > 0 {
            let prev = line[idx_in_line - 1];
            if prev.0 < x {
                segments = set_segment(segments, 3); // West (prev is west of current)
            } else if prev.0 > x {
                segments = set_segment(segments, 1); // East
            } else if prev.1 < y {
                segments = set_segment(segments, 0); // North
            } else if prev.1 > y {
                segments = set_segment(segments, 2); // South
            }
        }
        if idx_in_line + 1 < line.len() {
            let next = line[idx_in_line + 1];
            if next.0 < x {
                segments = set_segment(segments, 3); // West
            } else if next.0 > x {
                segments = set_segment(segments, 1); // East
            } else if next.1 < y {
                segments = set_segment(segments, 0); // North
            } else if next.1 > y {
                segments = set_segment(segments, 2); // South
            }
        }

        let road_type = match segments {
            s if s == LINE_N | LINE_S || s == LINE_N || s == LINE_S => road_ns,
            s if s == LINE_E | LINE_W || s == LINE_E || s == LINE_W => road_ew,
            _ => road_nesw,
        };

        grid[yu][xu] = road_type;
    }
}

// ---------------------------------------------------------------------------
// scoring function for greedy_path
// ---------------------------------------------------------------------------

fn road_scoring_fn(
    grid: &OmtGrid,
    registry: &TerrainRegistry,
    node: DirectedNode,
    _prev: Option<DirectedNode>,
) -> NodeScore {
    let (x, y) = node.pos;
    if !inbounds_omt_margin((x, y), 1) {
        return NodeScore::REJECTED;
    }

    let handle = ter_at(grid, x, y);
    let flags = registry.flags_for(handle);

    // Reject impassable / water tiles
    if flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
        || flags.contains(TerrainFlags::IMPASSABLE)
    {
        return NodeScore::REJECTED;
    }

    // Roads and highways are cheap
    if flags.contains(TerrainFlags::ROAD) || flags.contains(TerrainFlags::HIGHWAY) {
        return NodeScore::new(1, 0);
    }

    // Default: moderate cost
    NodeScore::new(5, 0)
}

// ---------------------------------------------------------------------------
// generate_fallback_exits
// ---------------------------------------------------------------------------

/// Generate fallback road exit points when fewer than 3 exits are available
/// from [`ConnectionExits`].
///
/// For each of the 4 cardinal directions, pick a border point with margin 10,
/// shuffle, and select the first non-river point (checking the tile itself
/// plus left/right neighbours for rivers).
fn generate_fallback_exits(
    grid: &OmtGrid,
    registry: &TerrainRegistry,
    rng: &mut XorShiftRng,
) -> Vec<(i32, i32)> {
    let margin = 10;
    let mut exits = Vec::new();

    let edges: [(i32, i32, i32, i32, i32, i32); 4] = [
        // (x_start, x_end, y_start, y_end, x_step, y_step)
        (margin, OMAP_DIM - margin, 0, 0, 1, 0), // North
        (OMAP_DIM - 1, OMAP_DIM - 1, margin, OMAP_DIM - margin, 0, 1), // East
        (margin, OMAP_DIM - margin, OMAP_DIM - 1, OMAP_DIM - 1, 1, 0), // South
        (0, 0, margin, OMAP_DIM - margin, 0, 1), // West
    ];

    for &(x_start, x_end, y_start, y_end, x_step, y_step) in &edges {
        // Collect all valid border points
        let mut candidates: Vec<(i32, i32)> = Vec::new();
        let mut x = x_start;
        let mut y = y_start;
        loop {
            if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
                candidates.push((x, y));
            }
            if (x_step > 0 && x >= x_end) || (x_step == 0 && y >= y_end) {
                break;
            }
            if x_step == 0 && y_step > 0 && y >= y_end {
                break;
            }
            x += x_step;
            y += y_step;
        }

        // Shuffle and select first non-river point
        let mut found = false;
        while !candidates.is_empty() && !found {
            let idx = rng.random_usize(candidates.len());
            let pt = candidates.swap_remove(idx);

            let handle = ter_at(grid, pt.0, pt.1);
            let flags = registry.flags_for(handle);

            if !flags.contains(TerrainFlags::RIVER) {
                // Check left and right neighbours perpendicular to the edge
                let (left_dx, left_dy) = if y_step != 0 { (-1, 0) } else { (0, -1) };
                let (right_dx, right_dy) = if y_step != 0 { (1, 0) } else { (0, 1) };

                let left_pt = (pt.0 + left_dx, pt.1 + left_dy);
                let right_pt = (pt.0 + right_dx, pt.1 + right_dy);

                let left_flags = registry.flags_for(ter_at(grid, left_pt.0, left_pt.1));
                let right_flags = registry.flags_for(ter_at(grid, right_pt.0, right_pt.1));

                if !left_flags.contains(TerrainFlags::RIVER)
                    && !right_flags.contains(TerrainFlags::RIVER)
                {
                    exits.push(pt);
                    found = true;
                }
            }
        }
    }

    exits
}

// ---------------------------------------------------------------------------
// place_roads — system entry point
// ---------------------------------------------------------------------------

/// Place inter-city roads connecting city centers and border exit points.
///
/// Port of C++ `overmap::place_roads()` (overmap.cpp L2163-2208).
pub fn place_roads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
    exits: Option<Res<ConnectionExits>>,
) {
    // Early return
    if settings.city.city_size <= 0 || !settings.place_roads {
        info!("place_roads: skipped — city_size<=0 or place_roads=false");
        return;
    }

    let city_count = cities.iter().count();
    if city_count == 0 {
        info!("place_roads: no cities to connect");
        return;
    }

    info!("place_roads: starting road network construction");

    // --- Build terrain grid --------------------------------------------------
    let (mut grid, _z0_chunks) = build_omt_grid(&chunks);

    // --- Collect road points -------------------------------------------------
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);

    // Border exit points
    let mut roads_out: Vec<(i32, i32)> = if let Some(ref exits_res) = exits {
        let mut pts = Vec::new();
        pts.extend(&exits_res.north);
        pts.extend(&exits_res.east);
        pts.extend(&exits_res.south);
        pts.extend(&exits_res.west);
        pts
    } else {
        Vec::new()
    };

    // Fallback exits if needed
    if roads_out.len() < 3 {
        roads_out.extend(generate_fallback_exits(&grid, &registry, &mut rng));
    }

    // City centers
    let mut road_points: Vec<(i32, i32)> = Vec::new();
    road_points.extend(&roads_out);
    for city in cities.iter() {
        road_points.push((city.omt_x, city.omt_y));
    }

    info!(
        exits = roads_out.len(),
        cities = city_count,
        total_points = road_points.len(),
        "place_roads: points collected"
    );

    // --- Connect points via MST ----------------------------------------------
    connect_closest_points(&road_points, 0, ConnectionType::InterCityRoad, &mut rng, {
        let grid_ref = &mut grid;
        let registry_ref = &registry;
        let core_terrains_ref = &core_terrains;
        move |from, to, z, ct| {
            // Use greedy_path for the actual path
            let max = (OMAP_DIM, OMAP_DIM);
            let scoring = |node: DirectedNode, prev: Option<DirectedNode>| {
                road_scoring_fn(grid_ref, registry_ref, node, prev)
            };
            let path = greedy_path(from, to, max, &scoring);
            if !path.is_empty() {
                // Convert path (dest→start) to line points for build_connection
                let mut line_pts: Vec<(i32, i32)> = path.iter().map(|n| n.pos).collect();
                line_pts.reverse(); // now start→dest
                for window in line_pts.windows(2) {
                    let sub_line = line_between(window[0], window[1]);
                    for &pt in &sub_line {
                        if inbounds_omt(pt) {
                            grid_ref[pt.1 as usize][pt.0 as usize] = core_terrains_ref.road_nesw.0;
                        }
                    }
                }
            } else {
                // Fallback: direct line
                build_connection(grid_ref, registry_ref, core_terrains_ref, from, to, z, ct);
            }
        }
    });

    // --- Write terrain changes back to chunks via par_iter --------------------
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let local_ox = (chunk_pos.chunk_x as i32) * (CHUNK_DIM as i32);
        let local_oy = (chunk_pos.chunk_y as i32) * (CHUNK_DIM as i32);

        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let wx = local_ox + lx as i32;
                let wy = local_oy + ly as i32;
                if wx >= 0 && wx < OMAP_DIM && wy >= 0 && wy < OMAP_DIM {
                    let idx = ly * CHUNK_DIM + lx;
                    let new_handle = TerrainHandle(grid[wy as usize][wx as usize]);
                    if new_terrain[idx] != new_handle && new_handle != TerrainHandle::NULL {
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

    info!("place_roads: road network complete");
}
