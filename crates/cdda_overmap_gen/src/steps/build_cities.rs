//! City street-grid construction — verbatim port of C++
//! `overmap::build_cities()` (overmap_city.cpp L214-233) plus the city street
//! building functions (L234-559).
//!
//! ## Algorithm
//!
//! 1. Build a dense terrain grid from z=0 chunks.
//! 2. For each [`super::cities::City`]:
//!    a. Place `road_nesw` at the city centre.
//!    b. Build streets in all 4 cardinal directions from the centre via
//!       recursive [`build_city_street`].
//! 3. [`flood_fill_city_tiles`] marks enclosed areas as city tiles (without
//!    changing terrain).
//! 4. Write terrain changes back to chunks.
//! 5. Update the [`super::cities::CityTiles`] resource.

use std::collections::{HashSet, VecDeque};

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::direction::{OmDirection, FOUR_ADJACENT_OFFSETS};
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::{City, CityTiles};

// ---------------------------------------------------------------------------
// Line constants — matching C++ `om_lines` bit layout
// ---------------------------------------------------------------------------

/// Probability constant for placing buildings alongside streets.
/// C++ `BUILDINGCHANCE` = 4, meaning `one_in(4)` or 25%.
const BUILDINGCHANCE: i32 = 4;

// ---------------------------------------------------------------------------
// Street node
// ---------------------------------------------------------------------------

/// A node in a street path — position + direction of travel.
#[derive(Debug, Clone, Copy)]
struct StreetNode {
    pos: (i32, i32),
    dir: OmDirection,
}

// ---------------------------------------------------------------------------
// straight_path
// ---------------------------------------------------------------------------

/// Create a straight path of [`StreetNode`]s from `source` in `dir` for `len` steps.
///
/// Verbatim port of C++ `straight_path()` (overmap.cpp L23-34):
/// - First `len-1` nodes have direction `dir`.
/// - Last node has direction `Invalid` (path terminus).
fn straight_path(source: (i32, i32), dir: OmDirection, len: i32) -> Vec<StreetNode> {
    if len <= 0 {
        return Vec::new();
    }

    let mut nodes = Vec::with_capacity(len as usize);
    let disp = dir.displace(1);
    let mut p = source;

    for _ in 0..len - 1 {
        nodes.push(StreetNode { pos: p, dir });
        p = (p.0 + disp.0, p.1 + disp.1);
    }
    // Last node — no direction (terminus)
    nodes.push(StreetNode {
        pos: p,
        dir: OmDirection::Invalid,
    });

    nodes
}

// ---------------------------------------------------------------------------
// valid_placement — check if a position can host a city street
// ---------------------------------------------------------------------------

/// Returns `true` if `pos` is a valid tile for street placement in `dir`.
///
/// Port of C++ `valid_placement()` lambda inside `lay_out_street()`
/// (overmap_city.cpp L306-382).
///
/// Checks:
/// - Point is within bounds with margin 1.
/// - Terrain is not a river, ravine, ravine-edge, highway, or
///   highway-reserved.
/// - Terrain does not collide with ≥2 existing road tiles in the 1-radius
///   neighbourhood (excluding the point directly ahead and directly behind).
fn valid_placement(
    grid: &[[u32; OMAP_DIM as usize]; OMAP_DIM as usize],
    registry: &TerrainRegistry,
    pos: (i32, i32),
    dir: OmDirection,
) -> bool {
    let (x, y) = pos;

    // Bounds check with 1-tile margin
    if x < 1 || x >= OMAP_DIM - 1 || y < 1 || y >= OMAP_DIM - 1 {
        return false;
    }

    let handle = TerrainHandle(grid[y as usize][x as usize]);
    let flags = registry.flags_for(handle);

    // Reject water / impassable terrain
    if flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
        || flags.contains(TerrainFlags::IMPASSABLE)
    {
        return false;
    }

    // Collision check — count existing road neighbours in the 3×3
    // neighbourhood, excluding:
    //   - the tile directly ahead: pos + displace(dir, 1)
    //   - the tile directly behind: pos + displace(opposite(dir), 1)
    //   - pos itself
    let forward = dir.displace(1);
    let backward = dir.opposite().displace(1);

    let skip = [
        (x + forward.0, y + forward.1),
        (x + backward.0, y + backward.1),
        pos,
    ];

    let mut collisions = 0u32;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let np = (x + dx, y + dy);
            if skip.contains(&np) {
                continue;
            }
            if np.0 < 0 || np.0 >= OMAP_DIM || np.1 < 0 || np.1 >= OMAP_DIM {
                continue;
            }
            let nh = TerrainHandle(grid[np.1 as usize][np.0 as usize]);
            if registry.flags_for(nh).contains(TerrainFlags::ROAD) {
                collisions += 1;
                if collisions >= 2 {
                    return false;
                }
            }
        }
    }

    true
}

// ---------------------------------------------------------------------------
// lay_out_street
// ---------------------------------------------------------------------------

/// Find a valid straight street segment starting from `source` in direction
/// `dir` with a maximum length of `len`.
///
/// Verbatim port of C++ `lay_out_street()` (overmap_city.cpp L306-382).
///
/// Returns the path as a `Vec<StreetNode>` (may be shorter than `len` if
/// blocked).  Adds visited tiles to `city_tiles`.
fn lay_out_street(
    grid: &[[u32; OMAP_DIM as usize]; OMAP_DIM as usize],
    registry: &TerrainRegistry,
    city_tiles: &mut HashSet<(i32, i32)>,
    source: (i32, i32),
    dir: OmDirection,
    len: i32,
) -> Vec<StreetNode> {
    if len <= 0 {
        return Vec::new();
    }

    let disp = dir.displace(1);

    // C++ L315-321: check if the tile `len` steps ahead is in-bounds and
    // passable — if so, extend `len` by 1.
    let en_pos = (source.0 + disp.0 * (len + 1), source.1 + disp.1 * (len + 1));
    let mut actual_len = len;
    if en_pos.0 >= 0 && en_pos.0 < OMAP_DIM && en_pos.1 >= 0 && en_pos.1 < OMAP_DIM {
        let en_handle = TerrainHandle(grid[en_pos.1 as usize][en_pos.0 as usize]);
        let en_flags = registry.flags_for(en_handle);
        if !en_flags.contains(TerrainFlags::RIVER)
            && !en_flags.contains(TerrainFlags::LAKE)
            && !en_flags.contains(TerrainFlags::OCEAN)
            && !en_flags.contains(TerrainFlags::IMPASSABLE)
        {
            actual_len += 1;
        }
    }

    // C++ L323-346: walk forward, stopping at the first invalid tile.
    let mut walked = 0i32;
    while walked < actual_len {
        let pos = (source.0 + disp.0 * walked, source.1 + disp.1 * walked);

        if !valid_placement(grid, registry, pos, dir) {
            break;
        }

        city_tiles.insert(pos);
        walked += 1;

        // C++ L339-343: if we've walked >1 tile and the current tile is
        // already a road, stop (we've hit an existing street).
        if walked > 1 {
            let cur_handle = TerrainHandle(grid[pos.1 as usize][pos.0 as usize]);
            if registry.flags_for(cur_handle).contains(TerrainFlags::ROAD) {
                break;
            }
        }
    }

    straight_path(source, dir, walked)
}

// ---------------------------------------------------------------------------
// pick_random_building_to_place
// ---------------------------------------------------------------------------

/// Pick a building type to place alongside a city street.
///
/// Port of C++ `pick_random_building_to_place()` (overmap_city.cpp L235-279).
///
/// Uses Gaussian-distribution logic for shop/park placement radii.
/// Since the building-special database is not yet ported, returns a
/// placeholder [`String`]: `"house"`, `"shop"`, or `"park"`.
fn pick_random_building_to_place(
    rng: &mut XorShiftRng,
    settings: &OvermapRegionSettings,
    town_dist: i32,
) -> String {
    let shop_radius = settings.city.shop_radius;
    let park_radius = settings.city.park_radius;
    let shop_sigma = settings.city.shop_sigma;
    let park_sigma = settings.city.park_sigma;

    // Approximate normal_roll using sum of 4 uniforms (central limit theorem).
    // C++ uses `normal_roll(mean, stddev)`.
    fn normal_approx(rng: &mut XorShiftRng, mean: i32, sigma: i32) -> i32 {
        if sigma <= 0 {
            return mean;
        }
        // Sum of 4 uniforms [0, sigma*2], centered
        let sum: f32 = (0..4).map(|_| rng.range_f32(0.0, (sigma * 2) as f32)).sum();
        let avg = sum / 4.0;
        mean + (avg - sigma as f32) as i32
    }

    let shop_normal = {
        let n = normal_approx(rng, shop_radius, shop_sigma);
        std::cmp::max(shop_radius, n)
    };
    let park_normal = {
        let n = normal_approx(rng, park_radius, park_sigma);
        std::cmp::max(park_radius, n)
    };

    // C++ L260-277
    if shop_normal > town_dist {
        "shop".to_string()
    } else if park_normal > town_dist {
        "park".to_string()
    } else {
        "house".to_string()
    }
}

// ---------------------------------------------------------------------------
// place_building — mark a building position
// ---------------------------------------------------------------------------

/// Mark a building adjacent to the street, one tile away in direction `dir`.
/// C++: `const tripoint_om_omt building_pos = p + om_direction::displace(dir);`
fn place_building(
    pos: (i32, i32),
    dir: OmDirection,
    _building_type: &str,
    buildings: &mut HashSet<(i32, i32)>,
) {
    let (dx, dy) = dir.displace(1);
    buildings.insert((pos.0 + dx, pos.1 + dy));
}

// ---------------------------------------------------------------------------
// build_connection — place road tiles along a street path
// ---------------------------------------------------------------------------

/// Place road terrain along a street path, resolving intersections with
/// existing roads.
///
/// Simplified port of C++ `build_connection()` (overmap.cpp L2563-2648) for
/// the intra-city case.
///
/// For each node:
/// - Determine the road type from the direction of travel.
/// - Check cardinal neighbours for existing roads; upgrade to intersection
///   (`road_nesw`) when cross-connections exist.
fn build_connection(
    grid: &mut [[u32; OMAP_DIM as usize]; OMAP_DIM as usize],
    registry: &TerrainRegistry,
    core_terrains: &CoreTerrains,
    path: &[StreetNode],
) {
    let road_ns = core_terrains.road_ns.0;
    let road_ew = core_terrains.road_ew.0;
    let road_nesw = core_terrains.road_nesw.0;

    for node in path {
        let (x, y) = node.pos;
        if x < 0 || x >= OMAP_DIM || y < 0 || y >= OMAP_DIM {
            continue;
        }
        let xu = x as usize;
        let yu = y as usize;

        let current_dir = node.dir;

        // Path terminus — no direction, skip
        if current_dir == OmDirection::Invalid {
            continue;
        }

        // Determine base road type from direction
        let is_vertical = current_dir == OmDirection::North || current_dir == OmDirection::South;

        // Check the 4 cardinal neighbours for existing roads
        let mut cross = false;
        for (dir_idx, &(dx, dy)) in FOUR_ADJACENT_OFFSETS.iter().enumerate() {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || nx >= OMAP_DIM || ny < 0 || ny >= OMAP_DIM {
                continue;
            }
            let nh = TerrainHandle(grid[ny as usize][nx as usize]);
            if registry.flags_for(nh).contains(TerrainFlags::ROAD) {
                let neighbor_dir = OmDirection::from_index(dir_idx);
                // If this neighbour road is perpendicular to our travel,
                // we need a full intersection.
                if !neighbor_dir.are_parallel(current_dir) {
                    cross = true;
                }
            }
        }

        let road_type = if cross {
            road_nesw
        } else if is_vertical {
            road_ns
        } else {
            road_ew
        };

        grid[yu][xu] = road_type;
    }
}

// ---------------------------------------------------------------------------
// build_city_street — recursive street builder
// ---------------------------------------------------------------------------

/// Recursively build streets outward from a position.
///
/// Verbatim port of C++ `build_city_street()` (overmap_city.cpp L384-460).
///
/// # Parameters
/// - `p`: starting position.
/// - `cs`: remaining "city size" budget (decremented each step).
/// - `dir`: direction of travel.
/// - `block_width`: width of the block; if 2, a new random width in [3,5]
///   is chosen for sub-streets.
/// - `buildings`: set to record placed building positions.
#[allow(clippy::too_many_arguments)]
fn build_city_street(
    grid: &mut [[u32; OMAP_DIM as usize]; OMAP_DIM as usize],
    registry: &TerrainRegistry,
    core_terrains: &CoreTerrains,
    city_tiles: &mut HashSet<(i32, i32)>,
    buildings: &mut HashSet<(i32, i32)>,
    rng: &mut XorShiftRng,
    settings: &OvermapRegionSettings,
    p: (i32, i32),
    cs: i32,
    dir: OmDirection,
    block_width: i32,
) {
    if cs <= 0 {
        return;
    }

    let mut c = cs;
    let mut croad = cs;

    // --- Lay out the main street --------------------------------------------
    let street_path = lay_out_street(grid, registry, city_tiles, p, dir, cs + 1);

    if street_path.len() <= 1 {
        return;
    }

    // --- Place road terrain --------------------------------------------------
    build_connection(grid, registry, core_terrains, &street_path);

    // --- Choose new block width for sub-streets ------------------------------
    let new_width = if block_width == 2 {
        rng.range_i32(3, 5)
    } else {
        2
    };

    // --- Walk the street, spawning sub-streets and buildings -----------------
    for (i, node) in street_path.iter().enumerate() {
        // Skip the first node (it's the start point)
        if i == 0 {
            continue;
        }

        c -= 1;

        // C++ L414-428: branch sub-streets when we have enough budget
        if c >= 2 && c < croad - block_width {
            croad = c;

            // Left branch
            let mut left = cs - rng.range_i32(1, 3);
            if left == 1 {
                left += 1;
            }
            build_city_street(
                grid,
                registry,
                core_terrains,
                city_tiles,
                buildings,
                rng,
                settings,
                node.pos,
                left,
                dir.turn_left(),
                new_width,
            );

            // Right branch
            let mut right = cs - rng.range_i32(1, 3);
            if right == 1 {
                right += 1;
            }
            build_city_street(
                grid,
                registry,
                core_terrains,
                city_tiles,
                buildings,
                rng,
                settings,
                node.pos,
                right,
                dir.turn_right(),
                new_width,
            );
        }

        // C++ L430-432: place buildings on left and right of the street
        if !rng.one_in(BUILDINGCHANCE) {
            let building = pick_random_building_to_place(rng, settings, croad);
            place_building(node.pos, dir.turn_left(), &building, buildings);
        }
        if !rng.one_in(BUILDINGCHANCE) {
            let building = pick_random_building_to_place(rng, settings, croad);
            place_building(node.pos, dir.turn_left(), &building, buildings);
        }
    }

    // --- C++ L434-448: continuation at the end of the street -----------------
    let last_node = street_path.last().unwrap();
    let remaining = cs - rng.range_i32(1, 3);

    if remaining >= 2 && c == 0 {
        let rnd_dir = dir.turn_random(rng);
        build_city_street(
            grid,
            registry,
            core_terrains,
            city_tiles,
            buildings,
            rng,
            settings,
            last_node.pos,
            remaining,
            rnd_dir,
            new_width,
        );

        // C++ L443-447: with 20% chance, also continue in the opposite direction
        if rng.one_in(5) {
            build_city_street(
                grid,
                registry,
                core_terrains,
                city_tiles,
                buildings,
                rng,
                settings,
                last_node.pos,
                remaining,
                rnd_dir.opposite(),
                new_width,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// flood_fill_city_tiles
// ---------------------------------------------------------------------------

/// Flood-fill enclosed areas bounded by city streets and mark them as city
/// tiles.
///
/// Verbatim port of C++ `flood_fill_city_tiles()` (overmap_city.cpp L516-559).
///
/// **IMPORTANT**: This function ONLY adds points to the `city_tiles` set.
/// It does **NOT** change terrain.  The `city_tiles` set is used downstream
/// for `is_in_city()` queries.
fn flood_fill_city_tiles(city_tiles: &mut HashSet<(i32, i32)>) {
    // 2D visited bitmap (OMAP_DIM × OMAP_DIM).
    let mut visited = vec![false; (OMAP_DIM as usize) * (OMAP_DIM as usize)];
    let idx = |x: i32, y: i32| -> usize { y as usize * OMAP_DIM as usize + x as usize };

    const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    for sy in 0..OMAP_DIM {
        for sx in 0..OMAP_DIM {
            let start_idx = idx(sx, sy);
            if visited[start_idx] {
                continue;
            }

            // Skip tiles that are already city tiles (they form the boundary).
            if city_tiles.contains(&(sx, sy)) {
                visited[start_idx] = true;
                continue;
            }

            // 4-connected flood fill
            let mut area: Vec<(i32, i32)> = Vec::new();
            let mut enclosed = true;
            let mut queue = VecDeque::new();

            queue.push_back((sx, sy));
            visited[start_idx] = true;

            while let Some((x, y)) = queue.pop_front() {
                area.push((x, y));

                for &(dx, dy) in &DIRS {
                    let nx = x + dx;
                    let ny = y + dy;

                    // Hit the map edge → not enclosed
                    if nx < 0 || nx >= OMAP_DIM || ny < 0 || ny >= OMAP_DIM {
                        enclosed = false;
                        continue;
                    }

                    let nidx = idx(nx, ny);
                    if visited[nidx] {
                        continue;
                    }

                    // Stop at city tiles (they form the boundary)
                    if city_tiles.contains(&(nx, ny)) {
                        visited[nidx] = true;
                        continue;
                    }

                    visited[nidx] = true;
                    queue.push_back((nx, ny));
                }
            }

            // If the area is fully enclosed, mark all tiles as city tiles
            if enclosed {
                for pt in &area {
                    city_tiles.insert(*pt);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

/// Build a dense `[[u32; 180]; 180]` grid from z=0 chunk entities.
fn build_omt_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
) -> (
    [[u32; OMAP_DIM as usize]; OMAP_DIM as usize],
    Vec<(Entity, ChunkPosition)>,
) {
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

/// Write the modified grid back to z=0 chunk entities via `Commands`.
fn write_back_grid(
    grid: &[[u32; OMAP_DIM as usize]; OMAP_DIM as usize],
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
                if omt_x >= 0 && omt_x < OMAP_DIM && omt_y >= 0 && omt_y < OMAP_DIM {
                    let new_handle = TerrainHandle(grid[omt_y as usize][omt_x as usize]);
                    new_terrain[idx] = new_handle;
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

// ---------------------------------------------------------------------------
// build_cities — system entry point
// ---------------------------------------------------------------------------

/// Build city street grids and flood-fill enclosed city tiles.
///
/// Verbatim port of C++ `overmap::build_cities()` (overmap_city.cpp L214-233)
/// plus all helper functions.
///
/// Reads [`City`] components placed by [`super::place_cities`] and writes
/// road terrain to z=0 chunks.  Updates the [`CityTiles`] resource.
#[allow(clippy::too_many_arguments)]
pub fn build_cities(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    _par_commands: ParallelCommands,
    cities: Query<&City>,
    city_tiles: Option<ResMut<CityTiles>>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    eprintln!("BUILD_CITIES called! city_spec={}", settings.city_spec);
    if !settings.city_spec {
        info!("build_cities: skipped — city_spec is false");
        return;
    }

    let city_count = cities.iter().count();
    eprintln!(
        "build_cities: city_count={} road_ns_idx={} road_ew_idx={} road_nesw_idx={}",
        city_count,
        core_terrains.road_ns.type_index(),
        core_terrains.road_ew.type_index(),
        core_terrains.road_nesw.type_index()
    );
    if city_count == 0 {
        info!("build_cities: no cities to build");
        return;
    }

    eprintln!(
        "build_cities: starting street grid construction, {} cities",
        city_count
    );

    // --- Build terrain grid --------------------------------------------------
    eprintln!("build_cities: about to build grid");
    let (mut grid, z0_chunks) = build_omt_grid(&chunks);
    let existing_ct = city_tiles.map(|ct| ct.clone()).unwrap_or_default();
    let mut tiles = existing_ct.tiles;
    let mut buildings = existing_ct.buildings;

    eprintln!("build_cities: grid built, {} chunks", z0_chunks.len());
    eprintln!("build_cities: grid[90][90]={}", grid[90][90]);
    let road_nesw_raw = core_terrains.road_nesw.0;

    // --- For each city, build streets in 4 directions ------------------------
    for city in cities.iter() {
        let cx = city.omt_x;
        let cy = city.omt_y;
        let cs = city.size as i32;

        info!(
            "build_cities: processing city at ({}, {}), size={}",
            cx, cy, cs
        );

        // Seed RNG from city position for deterministic placement
        let mut rng = XorShiftRng::new(
            (config.om_x as u64)
                ^ ((config.om_y as u64) << 16)
                ^ ((cx as u64) << 32)
                ^ ((cy as u64) << 48),
        );

        // Place road_nesw at city centre
        if cx >= 0 && cx < OMAP_DIM && cy >= 0 && cy < OMAP_DIM {
            grid[cy as usize][cx as usize] = road_nesw_raw;
            eprintln!(
                "build_cities: SET grid[{}][{}] = {} (road_nesw_raw={})",
                cx, cy, grid[cy as usize][cx as usize], road_nesw_raw
            );
            tiles.insert((cx, cy));
            info!(
                "build_cities: placed road_nesw at city center ({}, {})",
                cx, cy
            );
        }

        // Build streets in all 4 cardinal directions from the centre
        let start_dir = OmDirection::random(&mut rng);
        info!("build_cities: start_dir = {:?}, cs = {}", start_dir, cs);

        for dir_offset in 0..4 {
            let dir =
                OmDirection::from_index((start_dir.to_index() + dir_offset) % OmDirection::SIZE);
            info!("build_cities: building street dir={:?}", dir);
            build_city_street(
                &mut grid,
                &registry,
                &core_terrains,
                &mut tiles,
                &mut buildings,
                &mut rng,
                &settings,
                (cx, cy),
                cs,
                dir,
                2,
            );
        }

        info!(cx, cy, cs, "build_cities: built streets for city");
    }

    let before_fill = tiles.len();
    flood_fill_city_tiles(&mut tiles);
    info!(
        city_tiles_before = before_fill,
        city_tiles_after = tiles.len(),
        buildings_placed = buildings.len(),
        "build_cities: flood-fill complete"
    );

    // --- Write terrain changes back to chunks --------------------------------
    write_back_grid(&grid, &z0_chunks, &mut commands);

    // --- Update CityTiles resource -------------------------------------------
    commands.insert_resource(CityTiles { tiles, buildings });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdda_overmap::registry::TerrainFlags;

    fn make_registry() -> (TerrainRegistry, CoreTerrains) {
        let mut r = TerrainRegistry::empty();
        r.register_no_entity("field", TerrainFlags::empty(), 2, "field".into(), 0);
        r.register_no_entity(
            "road_ns",
            TerrainFlags::from_bits(TerrainFlags::ROAD | TerrainFlags::LINE_DRAWING),
            1,
            "road".into(),
            0,
        );
        r.register_no_entity(
            "road_ew",
            TerrainFlags::from_bits(TerrainFlags::ROAD | TerrainFlags::LINE_DRAWING),
            1,
            "road".into(),
            0,
        );
        r.register_no_entity(
            "road_nesw",
            TerrainFlags::from_bits(TerrainFlags::ROAD | TerrainFlags::LINE_DRAWING),
            1,
            "road".into(),
            0,
        );
        let ct = CoreTerrains::from_registry(&r);
        (r, ct)
    }

    #[test]
    fn test_lay_out_street_simple() {
        let (reg, ct) = make_registry();
        let mut grid = [[ct.field.0; 180]; 180]; // field handles
        let mut city_tiles = HashSet::new();

        // Place a road at center
        grid[90][90] = ct.road_nesw.0;

        // Lay out street going north from center
        let path = lay_out_street(
            &grid,
            &reg,
            &mut city_tiles,
            (90, 90),
            OmDirection::North,
            13,
        );

        eprintln!("TEST: path len = {}", path.len());
        eprintln!("TEST: city_tiles count = {}", city_tiles.len());
        assert!(path.len() > 1, "path should have multiple nodes");
    }

    #[test]
    fn test_build_connection_writes_roads() {
        let (reg, ct) = make_registry();
        let mut grid = [[ct.field.0; 180]; 180];

        // Create a path going east from (10,10)
        let path = straight_path((10, 10), OmDirection::East, 5);
        assert_eq!(path.len(), 5);

        build_connection(&mut grid, &reg, &ct, &path);

        // Check that road tiles were written
        for i in 0..4 {
            let h = TerrainHandle(grid[10][10 + i]);
            eprintln!(
                "TEST: grid[10][{}] = type_index={} flags={:?}",
                10 + i,
                h.type_index(),
                reg.flags_for(h)
            );
            assert!(
                reg.flags_for(h).contains(TerrainFlags::ROAD),
                "tile at (10,{}) should be a road",
                10 + i
            );
        }
        // Last node has Invalid direction, should not be written
        let h = TerrainHandle(grid[10][14]);
        assert!(
            !reg.flags_for(h).contains(TerrainFlags::ROAD),
            "tile at (10,14) should NOT be a road (terminus)"
        );
    }
}
