//! Railroad placement via minimum-spanning-tree pathfinding.
//!
//! Verbatim port of C++ `overmap::place_railroads()` (overmap.cpp L2227-2297).
//!
//! ## Algorithm
//!
//! 1. Build terrain grid from z=0 chunks.
//! 2. Collect border exit points from [`ConnectionExits`] (or generate fallback).
//! 3. Build `railroad_points`: exits + random points around each city.
//! 4. Call [`connect_closest_points`] with [`ConnectionType::Railroad`].
//! 5. Write railroad terrain back to chunks.

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
// Edge coordinate constants — matching C++ L2238-2241
// ---------------------------------------------------------------------------

/// X-coordinates for the 4 edges: [North, East, South, West].
/// When a particular edge is "null" (no neighbor), the coordinate is set to
/// these constants so the fallback generator knows which edge to sample.
const EDGE_COORDS_X: [i32; 4] = [OMAP_DIM - 1, -1, 0, -1];
const EDGE_COORDS_Y: [i32; 4] = [-1, OMAP_DIM - 1, -1, 0];

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

fn ter_at(grid: &OmtGrid, x: i32, y: i32) -> TerrainHandle {
    if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
        TerrainHandle(grid[y as usize][x as usize])
    } else {
        TerrainHandle::NULL
    }
}

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

// ---------------------------------------------------------------------------
// Railroad scoring function for greedy_path
// ---------------------------------------------------------------------------

fn railroad_scoring_fn(
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

    // Existing railroads and roads are cheap
    if flags.contains(TerrainFlags::RAILROAD) || flags.contains(TerrainFlags::ROAD) {
        return NodeScore::new(1, 0);
    }

    // Default moderate cost
    NodeScore::new(5, 0)
}

// ---------------------------------------------------------------------------
// generate_fallback_railroad_exits
// ---------------------------------------------------------------------------

/// Generate railroad exit points from null-neighbor edges.
///
/// For each direction where the corresponding coordinate is -1 (indicating
/// no neighbor), sample a point along that edge.
fn generate_fallback_railroad_exits(
    grid: &OmtGrid,
    registry: &TerrainRegistry,
    rng: &mut XorShiftRng,
) -> Vec<(i32, i32)> {
    let margin = 10;
    let mut exits = Vec::new();

    for dir_idx in 0..4 {
        let ex = EDGE_COORDS_X[dir_idx];
        let ey = EDGE_COORDS_Y[dir_idx];

        // Only generate for "null" edges
        if ex != -1 && ey != -1 {
            continue;
        }

        let mut candidates: Vec<(i32, i32)> = Vec::new();

        match dir_idx {
            0 => {
                // North edge (y=0)
                for x in margin..OMAP_DIM - margin {
                    let h = ter_at(grid, x, 0);
                    if !registry.flags_for(h).contains(TerrainFlags::RIVER) {
                        candidates.push((x, 0));
                    }
                }
            }
            1 => {
                // East edge (x=OMAP_DIM-1)
                for y in margin..OMAP_DIM - margin {
                    let h = ter_at(grid, OMAP_DIM - 1, y);
                    if !registry.flags_for(h).contains(TerrainFlags::RIVER) {
                        candidates.push((OMAP_DIM - 1, y));
                    }
                }
            }
            2 => {
                // South edge (y=OMAP_DIM-1)
                for x in margin..OMAP_DIM - margin {
                    let h = ter_at(grid, x, OMAP_DIM - 1);
                    if !registry.flags_for(h).contains(TerrainFlags::RIVER) {
                        candidates.push((x, OMAP_DIM - 1));
                    }
                }
            }
            3 => {
                // West edge (x=0)
                for y in margin..OMAP_DIM - margin {
                    let h = ter_at(grid, 0, y);
                    if !registry.flags_for(h).contains(TerrainFlags::RIVER) {
                        candidates.push((0, y));
                    }
                }
            }
            _ => {}
        }

        if !candidates.is_empty() {
            let idx = rng.random_usize(candidates.len());
            exits.push(candidates[idx]);
        }
    }

    exits
}

// ---------------------------------------------------------------------------
// points_in_radius
// ---------------------------------------------------------------------------

/// Return all OMT points within Chebyshev distance `radius` of `center`.
fn points_in_radius(center: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
    let (cx, cy) = center;
    let mut pts = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let x = cx + dx;
            let y = cy + dy;
            if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
                pts.push((x, y));
            }
        }
    }
    pts
}

// ---------------------------------------------------------------------------
// build_railroad_connection
// ---------------------------------------------------------------------------

fn build_railroad_connection(
    grid: &mut OmtGrid,
    registry: &TerrainRegistry,
    core_terrains: &CoreTerrains,
    from: (i32, i32),
    to: (i32, i32),
    _z: i32,
    _conn_type: ConnectionType,
) {
    let railroad_ns = registry
        .handle_by_id("railroad_ns")
        .map(|h| h.0)
        .unwrap_or(core_terrains.road_ns.0);
    let railroad_ew = registry
        .handle_by_id("railroad_ew")
        .map(|h| h.0)
        .unwrap_or(core_terrains.road_ew.0);
    let railroad_nesw = registry
        .handle_by_id("railroad_nesw")
        .map(|h| h.0)
        .unwrap_or(core_terrains.road_nesw.0);

    let line = line_between(from, to);

    for &(x, y) in &line {
        if !inbounds_omt((x, y)) {
            continue;
        }
        let xu = x as usize;
        let yu = y as usize;

        // Determine railroad type from line direction
        let idx_in_line = line.iter().position(|&p| p == (x, y)).unwrap_or(0);
        let mut segments: u16 = 0;

        if idx_in_line > 0 {
            let prev = line[idx_in_line - 1];
            if prev.1 < y {
                segments = set_segment(segments, 0); // North
            } else if prev.0 > x {
                segments = set_segment(segments, 1); // East
            } else if prev.1 > y {
                segments = set_segment(segments, 2); // South
            } else if prev.0 < x {
                segments = set_segment(segments, 3); // West
            }
        }
        if idx_in_line + 1 < line.len() {
            let next = line[idx_in_line + 1];
            if next.1 < y {
                segments = set_segment(segments, 0);
            } else if next.0 > x {
                segments = set_segment(segments, 1);
            } else if next.1 > y {
                segments = set_segment(segments, 2);
            } else if next.0 < x {
                segments = set_segment(segments, 3);
            }
        }

        // Check for existing railroads in cardinal neighbors
        for (dir_idx, &(dx, dy)) in FOUR_ADJACENT_OFFSETS.iter().enumerate() {
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && nx < OMAP_DIM && ny >= 0 && ny < OMAP_DIM {
                let nh = TerrainHandle(grid[ny as usize][nx as usize]);
                if registry.flags_for(nh).contains(TerrainFlags::RAILROAD) {
                    segments = set_segment(segments, dir_idx);
                }
            }
        }

        let rail_type = match segments {
            s if s == LINE_N || s == LINE_S || s == LINE_N | LINE_S => railroad_ns,
            s if s == LINE_E || s == LINE_W || s == LINE_E | LINE_W => railroad_ew,
            _ => railroad_nesw,
        };

        grid[yu][xu] = rail_type;
    }
}

// ---------------------------------------------------------------------------
// place_railroads — system entry point
// ---------------------------------------------------------------------------

/// Place railroads connecting border exits and city-adjacent points.
///
/// Port of C++ `overmap::place_railroads()` (overmap.cpp L2227-2297).
pub fn place_railroads(
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
    if settings.city.city_size <= 0 || !settings.place_railroads {
        info!("place_railroads: skipped — city_size<=0 or place_railroads=false");
        return;
    }

    info!("place_railroads: starting railroad network construction");

    // --- Build terrain grid --------------------------------------------------
    let (mut grid, _z0_chunks) = build_omt_grid(&chunks);
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 13);

    // --- Collect railroad exit points ----------------------------------------
    let mut railroads_out: Vec<(i32, i32)> = if let Some(ref exits_res) = exits {
        let mut pts = Vec::new();
        pts.extend(&exits_res.north);
        pts.extend(&exits_res.east);
        pts.extend(&exits_res.south);
        pts.extend(&exits_res.west);
        pts
    } else {
        Vec::new()
    };

    // Fallback: generate from null-neighbor edges
    if railroads_out.len() < 3 {
        railroads_out.extend(generate_fallback_railroad_exits(&grid, &registry, &mut rng));
    }

    // --- Build railroad points: exits + random city-adjacent points ----------
    let mut railroad_points: Vec<(i32, i32)> = Vec::new();
    railroad_points.extend(&railroads_out);

    for city in cities.iter() {
        let city_pos = (city.omt_x, city.omt_y);
        let radius = city.size as i32 * 4;
        let mut candidates = points_in_radius(city_pos, radius);

        if !candidates.is_empty() {
            let idx = rng.random_usize(candidates.len());
            railroad_points.push(candidates.swap_remove(idx));
        }
    }

    info!(
        exits = railroads_out.len(),
        total_points = railroad_points.len(),
        "place_railroads: points collected"
    );

    // --- Connect points via MST ----------------------------------------------
    connect_closest_points(&railroad_points, 0, ConnectionType::Railroad, &mut rng, {
        let grid_ref = &mut grid;
        let registry_ref = &registry;
        let core_terrains_ref = &core_terrains;
        move |from, to, z, ct| {
            let max = (OMAP_DIM, OMAP_DIM);
            let scoring = |node: DirectedNode, prev: Option<DirectedNode>| {
                railroad_scoring_fn(grid_ref, registry_ref, node, prev)
            };
            let path = greedy_path(from, to, max, &scoring);
            if !path.is_empty() {
                let mut line_pts: Vec<(i32, i32)> = path.iter().map(|n| n.pos).collect();
                line_pts.reverse();
                for window in line_pts.windows(2) {
                    let sub_line = line_between(window[0], window[1]);
                    for &pt in &sub_line {
                        if inbounds_omt(pt) {
                            let rr_nesw = registry_ref
                                .handle_by_id("railroad_nesw")
                                .map(|h| h.0)
                                .unwrap_or(core_terrains_ref.road_nesw.0);
                            grid_ref[pt.1 as usize][pt.0 as usize] = rr_nesw;
                        }
                    }
                }
            } else {
                build_railroad_connection(
                    grid_ref,
                    registry_ref,
                    core_terrains_ref,
                    from,
                    to,
                    z,
                    ct,
                );
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

    info!("place_railroads: railroad network complete");
}
