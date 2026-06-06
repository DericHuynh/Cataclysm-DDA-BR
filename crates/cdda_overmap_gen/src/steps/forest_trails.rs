//! Forest trail placement via flood-fill region detection and MST pathfinding.
//!
//! Verbatim port of C++ `overmap::place_forest_trails()` (overmap.cpp L1875-1998).
//!
//! ## Algorithm
//!
//! 1. Build terrain grid from z=0 chunks.
//! 2. For each OMT tile: if forest and not visited, flood-fill 4-connected.
//! 3. Skip regions smaller than `forest_trail_min_size`.
//! 4. `one_in(forest_trail_chance)` random skip.
//! 5. Find extrema (N/S/E/W) and approximate center.
//! 6. Pick random interior points.
//! 7. Optionally include border extrema points.
//! 8. Call [`connect_closest_points`] with [`ConnectionType::ForestTrail`].
//! 9. Write forest trail tiles back to chunks.

use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, line_between, ConnectionType,
};
use cdda_overmap::direction::{Rng, FOUR_ADJACENT_OFFSETS};
use cdda_overmap::registry::{CoreTerrains, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// Grid type
// ---------------------------------------------------------------------------

type OmtGrid = [[u32; OMAP_DIM as usize]; OMAP_DIM as usize];

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

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
// is_forest predicate
// ---------------------------------------------------------------------------

/// Returns `true` if the terrain handle represents a forest tile
/// (forest, forest_thick, or forest_water).
fn is_forest(handle: TerrainHandle, core_terrains: &CoreTerrains) -> bool {
    let type_idx = handle.type_index();
    type_idx == core_terrains.forest.type_index()
        || type_idx == core_terrains.forest_thick.type_index()
        || type_idx == core_terrains.forest_water.type_index()
}

// ---------------------------------------------------------------------------
// Flood-fill forest regions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ForestRegion {
    points: Vec<(i32, i32)>,
    north: i32,
    south: i32,
    east: i32,
    west: i32,
}

/// Flood-fill all forest regions on the grid. Returns a list of regions
/// (each at least `min_size` tiles).
fn find_forest_regions(
    grid: &OmtGrid,
    core_terrains: &CoreTerrains,
    min_size: usize,
) -> Vec<ForestRegion> {
    let mut visited = vec![false; (OMAP_DIM as usize) * (OMAP_DIM as usize)];
    let idx = |x: i32, y: i32| -> usize { y as usize * OMAP_DIM as usize + x as usize };
    let mut regions = Vec::new();

    const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    for sy in 0..OMAP_DIM {
        for sx in 0..OMAP_DIM {
            let si = idx(sx, sy);
            if visited[si] {
                continue;
            }

            let handle = TerrainHandle(grid[sy as usize][sx as usize]);
            if !is_forest(handle, core_terrains) {
                visited[si] = true;
                continue;
            }

            // Flood-fill this forest region
            let mut points = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back((sx, sy));
            visited[si] = true;

            let mut north = sy;
            let mut south = sy;
            let mut east = sx;
            let mut west = sx;

            while let Some((x, y)) = queue.pop_front() {
                points.push((x, y));

                if y < north {
                    north = y;
                }
                if y > south {
                    south = y;
                }
                if x > east {
                    east = x;
                }
                if x < west {
                    west = x;
                }

                for &(dx, dy) in &DIRS {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || nx >= OMAP_DIM || ny < 0 || ny >= OMAP_DIM {
                        continue;
                    }
                    let ni = idx(nx, ny);
                    if visited[ni] {
                        continue;
                    }
                    let nh = TerrainHandle(grid[ny as usize][nx as usize]);
                    if is_forest(nh, core_terrains) {
                        visited[ni] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }

            if points.len() >= min_size {
                regions.push(ForestRegion {
                    points,
                    north,
                    south,
                    east,
                    west,
                });
            }
        }
    }

    regions
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
// build_trail_connection
// ---------------------------------------------------------------------------

fn build_trail_connection(
    grid: &mut OmtGrid,
    registry: &TerrainRegistry,
    core_terrains: &CoreTerrains,
    from: (i32, i32),
    to: (i32, i32),
    _z: i32,
    _conn_type: ConnectionType,
) {
    let trail_ns = registry
        .handle_by_id("forest_trail_ns")
        .map(|h| h.0)
        .unwrap_or(core_terrains.road_ns.0);
    let trail_ew = registry
        .handle_by_id("forest_trail_ew")
        .map(|h| h.0)
        .unwrap_or(core_terrains.road_ew.0);
    let trail_nesw = registry
        .handle_by_id("forest_trail_nesw")
        .map(|h| h.0)
        .unwrap_or(core_terrains.road_nesw.0);

    let line = line_between(from, to);

    for &(x, y) in &line {
        if !inbounds_omt((x, y)) {
            continue;
        }
        let xu = x as usize;
        let yu = y as usize;

        let idx_in_line = line.iter().position(|&p| p == (x, y)).unwrap_or(0);
        let mut segments: u16 = 0;

        // Determine directionality from line neighbors
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

        // Also check for existing trails in cardinal neighbors
        for (dir_idx, &(dx, dy)) in FOUR_ADJACENT_OFFSETS.iter().enumerate() {
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && nx < OMAP_DIM && ny >= 0 && ny < OMAP_DIM {
                let nh = TerrainHandle(grid[ny as usize][nx as usize]);
                if registry
                    .string_id_for(nh)
                    .map_or(false, |id| id.starts_with("forest_trail"))
                {
                    segments = set_segment(segments, dir_idx);
                }
            }
        }

        let trail_type = match segments {
            s if s == LINE_N || s == LINE_S || s == LINE_N | LINE_S => trail_ns,
            s if s == LINE_E || s == LINE_W || s == LINE_E | LINE_W => trail_ew,
            _ => trail_nesw,
        };

        grid[yu][xu] = trail_type;
    }
}

// ---------------------------------------------------------------------------
// place_forest_trails — system entry point
// ---------------------------------------------------------------------------

/// Place forest trails within qualifying forest regions.
///
/// Port of C++ `overmap::place_forest_trails()` (overmap.cpp L1875-1998).
pub fn place_forest_trails(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.forest_trail {
        info!("place_forest_trails: skipped — forest_trail=false");
        return;
    }

    let trail_settings = &settings.forest_trail_settings;
    info!("place_forest_trails: starting trail placement");

    // --- Build terrain grid --------------------------------------------------
    let (mut grid, _z0_chunks) = build_omt_grid(&chunks);

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 19);

    // --- Find forest regions -------------------------------------------------
    let regions = find_forest_regions(&grid, &core_terrains, trail_settings.minimum_forest_size);

    info!(
        forest_regions = regions.len(),
        min_size = trail_settings.minimum_forest_size,
        "place_forest_trails: regions detected"
    );

    let mut trails_placed = 0usize;

    for region in &regions {
        // Random skip
        if !rng.one_in(trail_settings.chance) {
            continue;
        }

        // --- Find center of extrema ------------------------------------------
        let center_x = (region.east + region.west) / 2;
        let center_y = (region.north + region.south) / 2;

        // --- Find actual forest point closest to center ----------------------
        let mut closest_to_center: Option<(i32, i32)> = None;
        let mut best_dist = i32::MAX;
        for &pt in &region.points {
            let dx = pt.0 - center_x;
            let dy = pt.1 - center_y;
            let dist = dx * dx + dy * dy;
            if dist < best_dist {
                best_dist = dist;
                closest_to_center = Some(pt);
            }
        }

        let Some(center_pt) = closest_to_center else {
            continue;
        };

        // --- Pick random interior points -------------------------------------
        let max_random = trail_settings.random_point_max.min(
            trail_settings.random_point_min
                + region.points.len() as i32 / trail_settings.random_point_size_scalar,
        );
        let num_random = trail_settings.random_point_min.max(max_random.clamp(
            trail_settings.random_point_min,
            trail_settings.random_point_max,
        ));

        // Shuffle region points and pick first num_random
        let mut shuffled = region.points.clone();
        for i in (1..shuffled.len()).rev() {
            let j = rng.random_usize(i + 1);
            shuffled.swap(i, j);
        }

        let mut trail_points: Vec<(i32, i32)> = Vec::new();
        trail_points.push(center_pt);

        for &pt in shuffled.iter().take(num_random as usize) {
            trail_points.push(pt);
        }

        // --- Optionally include border extrema points ------------------------
        if !rng.one_in(trail_settings.border_point_chance) {
            // Add N/S/E/W extrema (closest actual forest points)
            let extrema = [
                (center_x, region.north), // North
                (region.east, center_y),  // East
                (center_x, region.south), // South
                (region.west, center_y),  // West
            ];

            for &ext in &extrema {
                let mut closest: Option<(i32, i32)> = None;
                let mut best_d = i32::MAX;
                for &pt in &region.points {
                    let dx = pt.0 - ext.0;
                    let dy = pt.1 - ext.1;
                    let d = dx * dx + dy * dy;
                    if d < best_d {
                        best_d = d;
                        closest = Some(pt);
                    }
                }
                if let Some(cp) = closest {
                    trail_points.push(cp);
                }
            }
        }

        if trail_points.len() < 2 {
            continue;
        }

        // --- Connect points via MST ------------------------------------------
        connect_closest_points(
            &trail_points,
            0,
            ConnectionType::ForestTrail,
            &mut rng,
            |from, to, z, ct| {
                build_trail_connection(&mut grid, &registry, &core_terrains, from, to, z, ct);
            },
        );

        trails_placed += 1;
    }

    info!(trails_placed, "place_forest_trails: trail networks built");

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

    info!("place_forest_trails: complete");
}
