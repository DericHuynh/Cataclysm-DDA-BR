//! Step 3b: Build city street grids and flood-fill enclosed areas.
//!
//! Port of CDDA master's `overmap::build_cities()` (overmap_city.cpp L214-233),
//! `build_city_street` (L384-460), and `flood_fill_city_tiles` (L516-559).

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::{City, CityTiles};
use crate::steps::city_buildings::CityBuildingCatalog;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::VecDeque;
use tracing::info;

/// Four cardinal directions as (dx, dy): N, E, S, W.
const CARDINAL_DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Build the city street grid and flood-fill enclosed tiles with buildings.
///
/// For each city:
/// 1. Pick a random starting direction.
/// 2. Walk the 4 cardinal directions (turning right each time).
/// 3. For each direction, build a street outward from the center with branches.
/// 4. After all streets are built, flood-fill areas fully enclosed by roads
///    and mark them as city tiles.
pub fn build_cities(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    cities: Query<&City>,
    mut city_tiles: ResMut<CityTiles>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
    catalog: Option<Res<CityBuildingCatalog>>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);
    let road_ns = registry
        .handle_by_id("road_ns")
        .unwrap_or(TerrainHandle::NULL);
    let road_ew = registry
        .handle_by_id("road_ew")
        .unwrap_or(TerrainHandle::NULL);
    let road_nesw = registry
        .handle_by_id("road_nesw")
        .unwrap_or(TerrainHandle::NULL);
    let road_nesw_manhole = registry
        .handle_by_id("road_nesw_manhole")
        .unwrap_or(road_nesw);
    let field_index = registry.field_index;

    // Build a mutable OMAP_DIM × OMAP_DIM grid (true = road/city tile).
    let mut grid = [[false; OMAP_DIM as usize]; OMAP_DIM as usize];

    // Seed grid from existing city tile centers.
    for &(x, y) in &city_tiles.tiles {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            grid[x as usize][y as usize] = true;
        }
    }

    for city in &cities {
        let cx = city.omt_x;
        let cy = city.omt_y;
        let size = city.size as i32;

        // Pick random start direction.
        let start_dir = rng.range_i32(0, 3) as usize;
        let mut dir_idx = start_dir;

        // CDDA: do { build_city_street(...); } while ((cur_dir = turn_right(cur_dir)) != start_dir);
        loop {
            let (dx, dy) = CARDINAL_DIRS[dir_idx];
            build_city_street(cx, cy, size, dx, dy, &mut grid, &mut rng);
            dir_idx = (dir_idx + 1) % 4; // turn_right
            if dir_idx == start_dir {
                break;
            }
        }
    }

    // Flood-fill enclosed areas.
    flood_fill_city_tiles(&mut grid);

    // Place buildings along city streets (CDDA place_building)
    if let Some(catalog) = catalog {
        if !catalog.buildings.is_empty() {
            place_buildings_along_streets(&mut grid, &catalog, &mut rng);
        }
    }

    // Write grid back to chunks.
    for (chunk_pos, mut chunk) in &mut chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= OMAP_DIM as usize || gy >= OMAP_DIM as usize {
                    continue;
                }
                if !grid[gx][gy] {
                    continue;
                }

                let current_type = chunk.get(lx, ly).type_index();
                // Don't overwrite existing roads, rivers, lakes, oceans.
                if current_type == road_ns.type_index()
                    || current_type == road_ew.type_index()
                    || current_type == road_nesw.type_index()
                {
                    continue;
                }
                // Only overwrite default terrain.
                if current_type != field_index {
                    continue;
                }

                // Determine road orientation from neighbours.
                let north = gy > 0 && grid[gx][gy - 1];
                let south = gy + 1 < OMAP_DIM as usize && grid[gx][gy + 1];
                let east = gx + 1 < OMAP_DIM as usize && grid[gx + 1][gy];
                let west = gx > 0 && grid[gx - 1][gy];
                let has_ns = north || south;
                let has_ew = east || west;

                let handle = if has_ns && has_ew {
                    // 50% chance of manhole on 4-way intersections (CDDA L443-445)
                    if rng.one_in(2) { road_nesw_manhole } else { road_nesw }
                } else if has_ew {
                    road_ew
                } else {
                    road_ns
                };
                chunk.set(lx, ly, handle);
            }
        }
    }

    // Update CityTiles.
    city_tiles.tiles.clear();
    for x in 0..OMAP_DIM as usize {
        for y in 0..OMAP_DIM as usize {
            if grid[x][y] {
                city_tiles.tiles.insert((x as i32, y as i32));
            }
        }
    }
    info!("Cities built: {} tiles", city_tiles.tiles.len());
}

// ---------------------------------------------------------------------------
// build_city_street — port of overmap_city.cpp L384-460
// ---------------------------------------------------------------------------

/// Build a street from `(cx, cy)` outward in direction `(dx, dy)`.
///
/// Recursively branches left/right at intervals to form a grid pattern.
fn build_city_street(
    cx: i32,
    cy: i32,
    size: i32,
    dx: i32,
    dy: i32,
    grid: &mut [[bool; OMAP_DIM as usize]],
    rng: &mut XorShiftRng,
) {
    // Pre-scan: how far can we go in this direction before hitting an edge?
    let mut actual_len: i32 = 0;
    let mut px = cx;
    let mut py = cy;

    while actual_len < size * 2 {
        px += dx;
        py += dy;
        if px < 1 || px >= OMAP_DIM - 1 || py < 1 || py >= OMAP_DIM - 1 {
            break;
        }
        actual_len += 1;
    }
    if actual_len == 0 {
        return;
    }

    // Walk outward from center placing roads and branching.
    px = cx;
    py = cy;

    for i in 0..=actual_len {
        if px >= 0 && px < OMAP_DIM && py >= 0 && py < OMAP_DIM {
            // Mark as road tile.
            grid[px as usize][py as usize] = true;

            // Widen roads for large cities: place parallel tiles.
            if size > 5 && dx != 0 {
                // Road runs east-west: widen north-south.
                let wy1 = py - 1;
                let wy2 = py + 1;
                if wy1 >= 0 && wy1 < OMAP_DIM {
                    grid[px as usize][wy1 as usize] = true;
                }
                if wy2 >= 0 && wy2 < OMAP_DIM {
                    grid[px as usize][wy2 as usize] = true;
                }
            } else if size > 5 && dy != 0 {
                // Road runs north-south: widen east-west.
                let wx1 = px - 1;
                let wx2 = px + 1;
                if wx1 >= 0 && wx1 < OMAP_DIM {
                    grid[wx1 as usize][py as usize] = true;
                }
                if wx2 >= 0 && wx2 < OMAP_DIM {
                    grid[wx2 as usize][py as usize] = true;
                }
            }

            // Branch left/right at intervals (not too close to center, not too close to edge).
            if i >= 3 && i <= actual_len - 3 && rng.one_in(3) {
                let left_size = (size as f64 * 0.5) as i32;
                let right_size = (size as f64 * 0.5) as i32;

                if left_size >= 2 {
                    let (lx, ly) = if dx != 0 { (-dy, -dx) } else { (dy, -dx) };
                    build_city_street(px, py, left_size, lx, ly, grid, rng);
                }
                if right_size >= 2 {
                    let (rx, ry) = if dx != 0 { (dy, dx) } else { (-dy, dx) };
                    build_city_street(px, py, right_size, rx, ry, grid, rng);
                }
            }
        }
        px += dx;
        py += dy;
    }

    // At the end of the street, maybe make a turn (neighbourhood effect).
    let end_x = cx + dx * actual_len;
    let end_y = cy + dy * actual_len;
    if actual_len >= 4 && rng.one_in(3) {
        let (tx, ty) = if rng.one_in(2) {
            if dx != 0 {
                (-dy, -dx)
            } else {
                (dy, -dx)
            }
        } else {
            if dx != 0 {
                (dy, dx)
            } else {
                (-dy, dx)
            }
        };
        build_city_street(end_x, end_y, (size as f64 * 0.5) as i32, tx, ty, grid, rng);
    }
}

// ---------------------------------------------------------------------------
// flood_fill_city_tiles — port of overmap_city.cpp L516-559
// ---------------------------------------------------------------------------

/// 4-connected flood fill: find enclosed regions and mark as city tiles.
///
/// Any region of non-road tiles that does not touch the overmap border
/// is considered enclosed and gets filled with city tiles.
fn flood_fill_city_tiles(grid: &mut [[bool; OMAP_DIM as usize]]) {
    let mut visited = [[false; OMAP_DIM as usize]; OMAP_DIM as usize];

    for sx in 1..OMAP_DIM as usize - 1 {
        for sy in 1..OMAP_DIM as usize - 1 {
            if grid[sx][sy] || visited[sx][sy] {
                continue;
            }

            // Flood fill this region.
            let mut region: Vec<(usize, usize)> = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back((sx, sy));
            visited[sx][sy] = true;
            let mut touches_border = false;

            while let Some((x, y)) = queue.pop_front() {
                region.push((x, y));
                if x == 0 || x == OMAP_DIM as usize - 1 || y == 0 || y == OMAP_DIM as usize - 1 {
                    touches_border = true;
                }
                for (nx, ny) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if nx < OMAP_DIM as usize
                        && ny < OMAP_DIM as usize
                        && !grid[nx][ny]
                        && !visited[nx][ny]
                    {
                        visited[nx][ny] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }

            // If region is enclosed (not touching border), mark as city.
            if !touches_border {
                for &(x, y) in &region {
                    grid[x][y] = true;
                }
            }
        }
    }
}


// ---------------------------------------------------------------------------
// place_buildings_along_streets — CDDA place_building port
// ---------------------------------------------------------------------------

/// For each road tile, try to place a building on both sides.
/// BUILDINGCHANCE = 4 in CDDA (skip 1/4, place 3/4).
const BUILDINGCHANCE: i32 = 4;

fn place_buildings_along_streets(
    grid: &mut [[bool; OMAP_DIM as usize]],
    catalog: &CityBuildingCatalog,
    rng: &mut XorShiftRng,
) {
    // Build a snapshot of road positions to avoid borrow issues.
    let road_positions: Vec<(i32, i32)> = (0..OMAP_DIM as usize)
        .flat_map(|x| (0..OMAP_DIM as usize).map(move |y| (x, y)))
        .filter(|&(x, y)| grid[x][y])
        .map(|(x, y)| (x as i32, y as i32))
        .collect();

    let n_buildings = catalog.buildings.len();
    if n_buildings == 0 { return; }

    for &(rx, ry) in &road_positions {
        // Check cardinal neighbors for possible building positions
        for &(dx, dy) in &[(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let bx = rx + dx;
            let by = ry + dy;
            if bx < 1 || bx >= OMAP_DIM - 1 || by < 1 || by >= OMAP_DIM - 1 { continue; }
            let bux = bx as usize;
            let buy = by as usize;
            // Only place buildings on empty tiles adjacent to roads
            if grid[bux][buy] { continue; }

            // CDDA: if (!one_in(BUILDINGCHANCE)) place_building(...)
            if rng.one_in(BUILDINGCHANCE) { continue; }

            // Pick a random building and place its OMTs
            let idx = rng.range_i32(0, n_buildings as i32 - 1) as usize;
            let building = &catalog.buildings[idx];

            // Try to place the building's OMTs
            let can_place = match &building.overmaps {
                Some(overmaps) => {
                    overmaps.iter().all(|omt| {
                        let px = bx + omt.point.first().copied().unwrap_or(0);
                        let py = by + omt.point.get(1).copied().unwrap_or(0);
                        if px < 1 || px >= OMAP_DIM - 1 || py < 1 || py >= OMAP_DIM - 1 { return false; }
                        !grid[px as usize][py as usize]
                    })
                }
                None => false,
            };

            if can_place {
                if let Some(overmaps) = &building.overmaps {
                    for omt in overmaps {
                        let px = bx + omt.point.first().copied().unwrap_or(0);
                        let py = by + omt.point.get(1).copied().unwrap_or(0);
                        if px >= 0 && px < OMAP_DIM && py >= 0 && py < OMAP_DIM {
                            grid[px as usize][py as usize] = true;
                        }
                    }
                }
                // Skip adjacent tile to avoid double-placement
                break;
            }
        }
    }
}
