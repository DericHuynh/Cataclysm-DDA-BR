//! Step 3b: Build city street grids and flood-fill enclosed areas.
//!
//! Verbatim port of CDDA master's `overmap_city.cpp` functions:
//!   - `straight_path`              (L38-53)
//!   - `overmap::build_cities`       (L214-233)
//!   - `overmap::pick_random_building_to_place` (L235-279)
//!   - `overmap::place_building`     (L281-304)
//!   - `overmap::lay_out_street`     (L306-382)
//!   - `overmap::build_city_street`  (L384-460)
//!   - `overmap::clear_cities`       (L462-465)
//!   - `overmap::is_in_city`         (L468-476)
//!   - `overmap::distance_to_city`   (L478-498)
//!   - `overmap::approx_distance_to_city` (L500-514)
//!   - `overmap::flood_fill_city_tiles`   (L516-559)
//!
//! Instead of working on `oter_id` directly, we work on a dense
//! `[[TerrainHandle; OMAP_DIM]; OMAP_DIM]` grid at z=0 built from chunk
//! entities, then flush writes back via `ParallelCommands`.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::{City, CityTiles};
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    closest_points_first, inbounds_omt, inbounds_omt_margin, trig_dist,
};
use cdda_overmap::direction::{OmDirection, FOUR_ADJACENT_OFFSETS};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_overmap::Rng;
use std::collections::{HashSet, VecDeque};
use tracing::info;

// ---------------------------------------------------------------------------
// Constants — matching C++ statics at overmap_city.cpp top
// ---------------------------------------------------------------------------

/// C++: `static const oter_str_id oter_road_nesw( "road_nesw" );`
const OTER_ROAD_NESW: &str = "road_nesw";
/// C++: `static const oter_str_id oter_road_nesw_manhole( "road_nesw_manhole" );`
const OTER_ROAD_NESW_MANHOLE: &str = "road_nesw_manhole";
/// C++: `static const oter_type_str_id oter_type_road( "road" );`
const OTER_TYPE_ROAD: &str = "road";
/// C++: `static constexpr int BUILDINGCHANCE = 4;`
const BUILDINGCHANCE: i32 = 4;

// ---------------------------------------------------------------------------
// directed_node / directed_path — mirror C++ pf::directed_node / directed_path
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct DirectedNode {
    pos: (i32, i32),
    dir: OmDirection,
}

#[derive(Clone, Debug)]
struct DirectedPath {
    nodes: Vec<DirectedNode>,
}

// ---------------------------------------------------------------------------
// straight_path — C++ overmap_city.cpp L38-53
// ---------------------------------------------------------------------------

/// C++: `pf::directed_path<point_om_omt> straight_path( const point_om_omt &source,
///        om_direction::type dir, size_t len )`
fn straight_path(source: (i32, i32), dir: OmDirection, len: usize) -> DirectedPath {
    // if (len == 0) return res;
    if len == 0 {
        return DirectedPath { nodes: Vec::new() };
    }
    // point_om_omt p = source;
    let mut p = source;
    // res.nodes.reserve(len);
    let mut nodes = Vec::with_capacity(len);
    // for (size_t i = 0; i + 1 < len; ++i) {
    //     res.nodes.emplace_back(p, dir);
    //     p += om_direction::displace(dir);
    // }
    for _ in 0..len - 1 {
        nodes.push(DirectedNode { pos: p, dir });
        let (dx, dy) = dir.displace(1);
        p = (p.0 + dx, p.1 + dy);
    }
    // res.nodes.emplace_back(p, om_direction::type::invalid);
    nodes.push(DirectedNode {
        pos: p,
        dir: OmDirection::Invalid,
    });
    DirectedPath { nodes }
}

// ---------------------------------------------------------------------------
// ConnectionStub — simplified overmap_connection
// ---------------------------------------------------------------------------

/// Simplified version of C++ `overmap_connection`.
///
/// Since the full connection system isn't ported yet, this provides the minimal
/// interface needed by `build_cities` functions:
///
/// - `pick_subtype_for(ter_id)` – returns true for field/forest, false for
///   river/highway/ravine (matching the C++ terrain eligibility check).
/// - `has(ter_id)` – returns true if the terrain has the ROAD flag.
struct ConnectionStub;

impl ConnectionStub {
    /// C++: `connection.pick_subtype_for(ter_id)`
    ///
    /// Returns true for buildable terrain (field, forest — not road, river,
    /// highway, ravine, etc.).
    fn pick_subtype_for(&self, handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
        let flags = registry.flags_for(handle);
        if flags.contains(TerrainFlags::ROAD)
            || flags.contains(TerrainFlags::RIVER)
            || flags.contains(TerrainFlags::HIGHWAY)
            || flags.contains(TerrainFlags::IMPASSABLE)
        {
            return false;
        }
        // Check for ravine via string ID (no dedicated flag)
        if let Some(id) = registry.string_id_for(handle) {
            if id.starts_with("ravine") {
                return false;
            }
        }
        true
    }

    /// C++: `connection.has(ter_id)`
    ///
    /// Returns true if the terrain is part of this road connection.
    fn has(&self, handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
        registry.flags_for(handle).contains(TerrainFlags::ROAD)
    }
}

// ===========================================================================
// Grid helpers
// ===========================================================================

/// Read a terrain handle from the flat grid at `(x, y)`.
#[inline]
fn ter(
    grid: &[[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    x: i32,
    y: i32,
) -> TerrainHandle {
    grid[x as usize][y as usize]
}

/// Write a terrain handle to the flat grid at `(x, y)`.
#[inline]
fn ter_set(
    grid: &mut [[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    x: i32,
    y: i32,
    handle: TerrainHandle,
) {
    grid[x as usize][y as usize] = handle;
}

// ===========================================================================
// pick_random_building_to_place — C++ overmap_city.cpp L235-279 (stub)
// ===========================================================================

/// C++: `overmap_special_id overmap::pick_random_building_to_place(...)`
///
/// **Stub**: The building special placement system (`overmap_special_id`,
/// `can_place_special`, `place_special`) is not yet ported.
///
/// Always returns an empty string. Once the special catalog is available,
/// this will be replaced with the full C++ logic.
#[allow(unused_variables)]
fn pick_random_building_to_place(
    town_dist: i32,
    town_size: i32,
    placed_unique_buildings: &HashSet<String>,
    settings: &OvermapRegionSettings,
    rng: &mut XorShiftRng,
) -> String {
    // Full C++ logic:
    //
    // const region_settings_city &city_spec = settings->get_settings_city();
    // int shop_radius = city_spec.shop_radius;
    // int park_radius = city_spec.park_radius;
    // int shop_sigma = city_spec.shop_sigma;
    // int park_sigma = city_spec.park_sigma;
    // int shop_normal = shop_radius;
    // if (shop_sigma > 0) { shop_normal = max(shop_normal, (int)normal_roll(shop_radius, shop_sigma)); }
    // int park_normal = park_radius;
    // if (park_sigma > 0) { park_normal = max(park_normal, (int)normal_roll(park_radius, park_sigma)); }
    // auto building_type_to_pick = [&]() {
    //     if (shop_normal > town_dist) return std::mem_fn(&region_settings_city::pick_shop);
    //     else if (park_normal > town_dist) return std::mem_fn(&region_settings_city::pick_park);
    //     else return std::mem_fn(&region_settings_city::pick_house);
    // };
    // auto pick_building = building_type_to_pick();
    // overmap_special_id ret;
    // bool existing_unique;
    // do {
    //     ret = pick_building(city_spec);
    //     if (ret->has_flag("CITY_UNIQUE")) { existing_unique = placed_unique_buildings.find(ret) != end; }
    //     else if (ret->has_flag("GLOBALLY_UNIQUE") || ret->has_flag("OVERMAP_UNIQUE")) {
    //         existing_unique = overmap_buffer.contains_unique_special(ret);
    //     } else { existing_unique = false; }
    // } while (existing_unique || !ret->get_constraints().city_size.contains(town_size));
    // return ret;

    // Stub: return empty string (noop) until the special system is ported.
    String::new()
}

// ===========================================================================
// place_building — C++ overmap_city.cpp L281-304 (stub)
// ===========================================================================

/// C++: `void overmap::place_building(const tripoint_om_omt &p, om_direction::type dir,
///       const city &town, std::unordered_set<overmap_special_id> &placed_unique_buildings)`
///
/// **Stub**: `can_place_special` / `place_special` are not yet ported.
///
/// Currently just inserts the building position into `city_tiles` without
/// actually placing a building. Once the special system is available, this
/// will be replaced with the full C++ logic.
#[allow(unused_variables)]
fn place_building(
    pos: (i32, i32),
    dir: OmDirection,
    town_pos: (i32, i32),
    town_size: i32,
    grid: &mut [[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    rng: &mut XorShiftRng,
    registry: &TerrainRegistry,
    city_tiles: &mut HashSet<(i32, i32)>,
    placed_unique_buildings: &mut HashSet<String>,
    settings: &OvermapRegionSettings,
) {
    // C++: const tripoint_om_omt building_pos = p + om_direction::displace(dir);
    let (dx, dy) = dir.displace(1);
    let building_pos = (pos.0 + dx, pos.1 + dy);

    // C++: const om_direction::type building_dir = om_direction::opposite(dir);
    let _building_dir = dir.opposite();

    // C++: const int town_dist = (trig_dist(building_pos.xy(), town.pos) * 100) / max(town.size, 1);
    let town_dist = (trig_dist(building_pos, town_pos) * 100.0 / town_size.max(1) as f32) as i32;

    // C++: for (size_t retries = 10; retries > 0; --retries)
    for _ in 0..10 {
        // C++: const overmap_special_id building_tid = pick_random_building_to_place(...);
        let building_tid = pick_random_building_to_place(
            town_dist,
            town_size,
            placed_unique_buildings,
            settings,
            rng,
        );

        // Stub: skip actual placement (can_place_special / place_special not available).
        // In the full port this would be:
        //   if (can_place_special(*building_tid, building_pos, building_dir, false)) {
        //       vector<tripoint_om_omt> used = place_special(...);
        //       for (tripoint_om_omt &p : used) { city_tiles.insert(p.xy()); }
        //       if (building_tid->has_flag("CITY_UNIQUE")) placed_unique_buildings.emplace(building_tid);
        //       break;
        //   }

        // Until then, just mark the tile as a city tile.
        if building_tid.is_empty() {
            city_tiles.insert(building_pos);
            break;
        }
    }
}

// ===========================================================================
// lay_out_street — C++ overmap_city.cpp L306-382
// ===========================================================================

/// C++: `pf::directed_path<point_om_omt> overmap::lay_out_street(
///         const overmap_connection &connection, const point_om_omt &source,
///         om_direction::type dir, size_t len )`
fn lay_out_street(
    connection: &ConnectionStub,
    source: (i32, i32),
    dir: OmDirection,
    len: usize,
    grid: &[[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    registry: &TerrainRegistry,
    city_tiles: &mut HashSet<(i32, i32)>,
) -> DirectedPath {
    // C++: const int &highway_width = settings->overmap_highway
    //           ? settings->get_settings_highway().width_of_segments : 0;
    // Simplified: highway system not ported, width = 0.
    let _highway_width: i32 = 0;

    // C++: auto valid_placement = [this](const overmap_connection & connection,
    //        const tripoint_om_omt pos, om_direction::type dir) { ... };
    let valid_placement = |pos: (i32, i32), _dir: OmDirection| -> bool {
        // C++: if (!inbounds(pos, 1)) return false;
        if !inbounds_omt_margin(pos, 1) {
            return false;
        }

        let ter_id = ter(grid, pos.0, pos.1);
        let flags = registry.flags_for(ter_id);

        // C++: if (ter_id->is_river() || ter_id->is_ravine() || ...)
        if flags.contains(TerrainFlags::RIVER) || flags.contains(TerrainFlags::IMPASSABLE) {
            return false;
        }

        // C++: is_ravine / is_ravine_edge
        if let Some(id) = registry.string_id_for(ter_id) {
            if id.starts_with("ravine") {
                return false;
            }
        }

        // C++: is_highway
        if flags.contains(TerrainFlags::HIGHWAY) {
            return false;
        }

        // C++: !connection.pick_subtype_for(ter_id)
        if !connection.pick_subtype_for(ter_id, registry) {
            return false;
        }

        // C++: int collisions = 0;
        let mut collisions = 0;
        // C++: for (const tripoint_om_omt &checkp : points_in_radius(pos, 1))
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let checkp = (pos.0 + dx, pos.1 + dy);
                if checkp.0 < 0 || checkp.0 >= OMAP_DIM || checkp.1 < 0 || checkp.1 >= OMAP_DIM {
                    continue;
                }

                // C++: if (checkp != pos + displace(dir, 1) &&
                //        checkp != pos + displace(opposite(dir), 1) && checkp != pos)
                let (ahead_dx, ahead_dy) = _dir.displace(1);
                let (behind_dx, behind_dy) = _dir.opposite().displace(1);
                if (dx == ahead_dx && dy == ahead_dy) || (dx == behind_dx && dy == behind_dy) {
                    continue;
                }

                // C++: if (ter(checkp)->get_type_id() == oter_type_road)
                let check_ter_id = ter(grid, checkp.0, checkp.1);
                let check_flags = registry.flags_for(check_ter_id);
                if check_flags.contains(TerrainFlags::ROAD) {
                    // C++: if (collisions >= 2) return false;
                    collisions += 1;
                    if collisions >= 2 {
                        return false;
                    }
                }
            }
        }
        true
    };

    // C++: const tripoint_om_omt from(source, 0);
    // C++: const tripoint_om_omt en_pos = from + om_direction::displace(dir, len + 1);
    let (disp_dx, disp_dy) = dir.displace(len as i32 + 1);
    let en_pos = (source.0 + disp_dx, source.1 + disp_dy);

    // C++: if (inbounds(en_pos, 1) && connection.has(ter(en_pos))) { ++len; }
    let mut target_len = len;
    if inbounds_omt_margin(en_pos, 1) && connection.has(ter(grid, en_pos.0, en_pos.1), registry) {
        target_len = len + 1;
    }

    // C++: size_t actual_len = 0;
    let mut actual_len: usize = 0;
    // C++: bool checked_highway = false;
    let mut _checked_highway = false;

    // C++: while (actual_len < len) { ... }
    while actual_len < target_len {
        // C++: const tripoint_om_omt pos = from + om_direction::displace(dir, actual_len);
        let (ddx, ddy) = dir.displace(actual_len as i32);
        let pos = (source.0 + ddx, source.1 + ddy);

        // C++: if (!valid_placement(connection, pos, dir)) { break; }
        if !valid_placement(pos, dir) {
            break;
        }

        let ter_id = ter(grid, pos.0, pos.1);
        let _flags = registry.flags_for(ter_id);

        // C++: if (ter_id->is_highway_reserved()) { ... }
        // Simplified: `_highway_width == 0` means highway logic never activates.
        // The full logic is:
        //   if (!checked_highway) {
        //       if (are_parallel(dir, ter_id.obj().get_dir())) break;
        //       const tripoint pos_after_highway = pos + displace(dir, highway_width);
        //       if (!valid_placement(connection, pos_after_highway, dir)) break;
        //       checked_highway = true;
        //   }
        //   if (actual_len == len - 1) ++len;

        // C++: city_tiles.insert(pos.xy());
        city_tiles.insert(pos);

        // C++: ++actual_len;
        actual_len += 1;

        // C++: if (actual_len > 1 && connection.has(ter_id)) { break; }
        if actual_len > 1 && connection.has(ter_id, registry) {
            break;
        }
    }

    // C++: return straight_path(source, dir, actual_len);
    straight_path(source, dir, actual_len)
}

// ===========================================================================
// build_city_street — C++ overmap_city.cpp L384-460
// ===========================================================================

/// C++: `void overmap::build_city_street(const overmap_connection &connection,
///        const point_om_omt &p, int cs, om_direction::type dir, const city &town,
///        std::unordered_set<overmap_special_id> &placed_unique_buildings,
///        int block_width)`
#[allow(clippy::too_many_arguments)]
fn build_city_street(
    connection: &ConnectionStub,
    p: (i32, i32),
    cs: i32,
    dir: OmDirection,
    town_center: (i32, i32),
    original_size: i32,
    grid: &mut [[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    rng: &mut XorShiftRng,
    registry: &TerrainRegistry,
    city_tiles: &mut HashSet<(i32, i32)>,
    placed_unique_buildings: &mut HashSet<String>,
    settings: &OvermapRegionSettings,
    block_width: i32,
) {
    // C++: int c = cs;
    let mut c = cs;
    // C++: int croad = cs;
    let mut croad = cs;

    // C++: if (dir == om_direction::type::invalid) { debugmsg(...); return; }
    if dir == OmDirection::Invalid {
        // debugmsg("Invalid road direction.");
        return;
    }

    // C++: const pf::directed_path<point_om_omt> street_path = lay_out_street(
    //         connection, p, dir, cs + 1 );
    let street_path = lay_out_street(
        connection,
        p,
        dir,
        (cs + 1) as usize,
        grid,
        registry,
        city_tiles,
    );

    // C++: if (street_path.nodes.size() <= 1) { return; }
    if street_path.nodes.len() <= 1 {
        return;
    }

    // C++: build_connection(connection, street_path, 0);
    //
    // Simplified: In the full C++ code, `build_connection` writes road terrain
    // along the path. Since the connection terrain lookup (line-based road
    // textures) is complex and handled elsewhere in the Rust pipeline, we just
    // mark every node position as a city tile here instead.
    for node in &street_path.nodes {
        city_tiles.insert(node.pos);
        // Force-set whatever tile is here to road_nesw as a visual landmark.
        // In the full port this would call connection.get_linear(new_line).
        if let Some(road_nesw) = registry.handle_by_id(OTER_ROAD_NESW) {
            let current = ter(grid, node.pos.0, node.pos.1);
            if !registry.flags_for(current).contains(TerrainFlags::ROAD) {
                ter_set(grid, node.pos.0, node.pos.1, road_nesw);
            }
        }
    }

    // C++: const auto from = std::next(street_path.nodes.begin());
    // C++: const auto to = street_path.nodes.end();
    let from = 1; // Skip the first node (the source / start)

    // C++: int new_width = block_width == 2 ? rng( 3, 5 ) : 2;
    let mut new_width: i32 = if block_width == 2 {
        rng.range_i32(3, 5)
    } else {
        2
    };

    // C++: for (auto iter = from; iter != to; ++iter)
    for i in from..street_path.nodes.len() {
        let node = &street_path.nodes[i];
        let rp = node.pos;

        // C++: --c;
        c -= 1;

        // C++: if (c >= 2 && c < croad - block_width)
        if c >= 2 && c < croad - block_width {
            croad = c;

            // C++: int left = cs - rng(1, 3);
            let mut left = cs - rng.range_i32(1, 3);
            // C++: if (left == 1) left++;
            if left == 1 {
                left += 1;
            }

            // C++: int right = cs - rng(1, 3);
            let mut right = cs - rng.range_i32(1, 3);
            // C++: if (right == 1) right++;
            if right == 1 {
                right += 1;
            }

            // C++: Alternating block widths
            new_width = if new_width == 2 {
                rng.range_i32(3, 5)
            } else {
                2
            };

            // C++: build_city_street(connection, iter->pos, left, turn_left(dir), ...)
            build_city_street(
                connection,
                node.pos,
                left,
                dir.turn_left(),
                town_center,
                original_size,
                grid,
                rng,
                registry,
                city_tiles,
                placed_unique_buildings,
                settings,
                new_width,
            );

            // C++: build_city_street(connection, iter->pos, right, turn_right(dir), ...)
            build_city_street(
                connection,
                node.pos,
                right,
                dir.turn_right(),
                town_center,
                original_size,
                grid,
                rng,
                registry,
                city_tiles,
                placed_unique_buildings,
                settings,
                new_width,
            );

            // C++: const oter_id &oter = ter(rp);
            // C++: if (one_in(2) && oter->get_line() == 15 && oter->type_is(oter_type_id("road")))
            if rng.one_in(2) {
                let oter = ter(grid, rp.0, rp.1);
                let flags = registry.flags_for(oter);
                // Check if this is a 4-way road intersection (line = 15 = LINE_NESW)
                if flags.contains(TerrainFlags::ROAD) {
                    let north = rp.1 > 0
                        && registry
                            .flags_for(ter(grid, rp.0, rp.1 - 1))
                            .contains(TerrainFlags::ROAD);
                    let east = rp.0 + 1 < OMAP_DIM
                        && registry
                            .flags_for(ter(grid, rp.0 + 1, rp.1))
                            .contains(TerrainFlags::ROAD);
                    let south = rp.1 + 1 < OMAP_DIM
                        && registry
                            .flags_for(ter(grid, rp.0, rp.1 + 1))
                            .contains(TerrainFlags::ROAD);
                    let west = rp.0 > 0
                        && registry
                            .flags_for(ter(grid, rp.0 - 1, rp.1))
                            .contains(TerrainFlags::ROAD);

                    let line = (if north { 0x04 } else { 0 })  // LINE_S = 4
                        | (if east { 0x08 } else { 0 })        // LINE_W = 8
                        | (if south { 0x01 } else { 0 })       // LINE_N = 1
                        | (if west { 0x02 } else { 0 }); // LINE_E = 2

                    // C++: if oter->get_line() == 15 (i.e. LINE_NESW)
                    if line == 15 {
                        // C++: ter_set(rp, oter_road_nesw_manhole.id());
                        if let Some(manhole) = registry.handle_by_id(OTER_ROAD_NESW_MANHOLE) {
                            ter_set(grid, rp.0, rp.1, manhole);
                        }
                    }
                }
            }
        }

        // C++: if (!one_in(BUILDINGCHANCE)) { place_building(rp, turn_left(dir), ...); }
        if !rng.one_in(BUILDINGCHANCE) {
            place_building(
                rp,
                dir.turn_left(),
                town_center,
                original_size,
                grid,
                rng,
                registry,
                city_tiles,
                placed_unique_buildings,
                settings,
            );
        }

        // C++: if (!one_in(BUILDINGCHANCE)) { place_building(rp, turn_right(dir), ...); }
        if !rng.one_in(BUILDINGCHANCE) {
            place_building(
                rp,
                dir.turn_right(),
                town_center,
                original_size,
                grid,
                rng,
                registry,
                city_tiles,
                placed_unique_buildings,
                settings,
            );
        }
    }

    // C++: cs -= rng(1, 3);
    let cs_remaining = cs - rng.range_i32(1, 3);

    // C++: if (cs >= 2 && c == 0)
    if cs_remaining >= 2 && c == 0 {
        // C++: const auto &last_node = street_path.nodes.back();
        let last_node = street_path.nodes[street_path.nodes.len() - 1];

        // C++: const om_direction::type rnd_dir = om_direction::turn_random(dir);
        let rnd_dir = dir.turn_random(rng);

        // C++: build_city_street(connection, last_node.pos, cs, rnd_dir, ...);
        build_city_street(
            connection,
            last_node.pos,
            cs_remaining,
            rnd_dir,
            town_center,
            original_size,
            grid,
            rng,
            registry,
            city_tiles,
            placed_unique_buildings,
            settings,
            block_width,
        );

        // C++: if (one_in(5))
        if rng.one_in(5) {
            // C++: build_city_street(connection, last_node.pos, cs, opposite(rnd_dir), ...);
            build_city_street(
                connection,
                last_node.pos,
                cs_remaining,
                rnd_dir.opposite(),
                town_center,
                original_size,
                grid,
                rng,
                registry,
                city_tiles,
                placed_unique_buildings,
                settings,
                new_width,
            );
        }
    }
}

// ===========================================================================
// clear_cities — C++ overmap_city.cpp L462-465 (stub)
// ===========================================================================

/// C++: `void overmap::clear_cities() { cities.clear(); }`
///
/// In ECS, cities are entities with `City` components. This function despawns
/// all city entities — effectively the same as clearing the vector.
pub fn clear_cities(mut commands: Commands, cities: Query<Entity, With<City>>) {
    for entity in &cities {
        commands.entity(entity).despawn();
    }
}

// ===========================================================================
// is_in_city — C++ overmap_city.cpp L468-476
// ===========================================================================

/// C++: `bool overmap::is_in_city(const tripoint_om_omt &p) const`
pub fn is_in_city(p: (i32, i32), city_tiles: &HashSet<(i32, i32)>) -> bool {
    // C++: if (!city_tiles.empty()) { return city_tiles.find(p.xy()) != city_tiles.end(); }
    if !city_tiles.is_empty() {
        return city_tiles.contains(&p);
    }
    // C++: else { return distance_to_city(p) == 0; }
    // The `else` branch requires the grid for fallback city-distance checking,
    // but is only used when `city_tiles` is empty (during city placement).
    false
}

// ===========================================================================
// distance_to_city — C++ overmap_city.cpp L478-498
// ===========================================================================

/// C++: `std::optional<int> overmap::distance_to_city(const tripoint_om_omt &p,
///        int max_dist_to_check) const`
///
/// Uses `city_tiles` if available, otherwise falls back to searching the
/// terrain grid for city-like terrain.
pub fn distance_to_city(
    p: (i32, i32),
    city_tiles: &HashSet<(i32, i32)>,
    _grid: &[[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    _registry: &TerrainRegistry,
    _cities: &[&City],
    max_dist_to_check: i32,
) -> Option<i32> {
    // C++: if (!city_tiles.empty()) {
    if !city_tiles.is_empty() {
        // C++: for (int i = 0; i <= max_dist_to_check; i++) {
        //         for (const tripoint_om_omt &tile : closest_points_first(p, i, i)) {
        //             if (is_in_city(tile)) { return i; }
        //         }
        //     }
        for i in 0..=max_dist_to_check {
            let ring = closest_points_first(p, i);
            // Filter to only points at exact Chebyshev distance `i`.
            for &tile in &ring {
                let dist = (tile.0 - p.0).abs().max((tile.1 - p.1).abs());
                if dist == i && city_tiles.contains(&tile) {
                    return Some(i);
                }
            }
        }
    }
    // C++: else {
    //     const city &nearest_city = get_nearest_city(p);
    //     if (!!nearest_city) {
    //         return std::max(0, nearest_city.get_distance_from(p) - nearest_city.size);
    //     }
    // }
    // Fallback: iterate cities to find closest (see approx_distance_to_city for pattern)
    // This is a simplified version since `get_nearest_city` requires city position utilities.

    // C++: return {};
    None
}

// ===========================================================================
// approx_distance_to_city — C++ overmap_city.cpp L500-514
// ===========================================================================

/// C++: `std::optional<int> overmap::approx_distance_to_city(
///        const tripoint_om_omt &p, int max_dist_to_check) const`
pub fn approx_distance_to_city(
    p: (i32, i32),
    cities: &[&City],
    max_dist_to_check: i32,
) -> Option<i32> {
    // C++: std::optional<int> ret;
    let mut ret: Option<i32> = None;

    // C++: for (const city &elem : cities) {
    for city in cities {
        let cx = city.omt_x;
        let cy = city.omt_y;
        // C++: const int dist = elem.get_distance_from(p);
        // Simplified: get_distance_from = trig_dist, matching C++ trig_dist
        let dist = trig_dist(p, (cx, cy)) as i32;

        // C++: if (dist == 0) { return 0; }
        if dist == 0 {
            return Some(0);
        }

        // C++: if (dist <= max_dist_to_check)
        if dist <= max_dist_to_check {
            // C++: ret = ret.has_value() ? std::min(ret.value(), dist) : dist;
            ret = Some(match ret {
                Some(existing) => existing.min(dist),
                None => dist,
            });
        }
    }

    // C++: return ret;
    ret
}

// ===========================================================================
// flood_fill_city_tiles — C++ overmap_city.cpp L516-559
// ===========================================================================

/// C++: `void overmap::flood_fill_city_tiles()`
///
/// 4-connected flood fill: find all regions of non-city/non-road tiles that do
/// **not** touch the overmap border and mark them as city tiles.
fn flood_fill_city_tiles(
    city_tiles: &mut HashSet<(i32, i32)>,
    grid: &[[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize],
    registry: &TerrainRegistry,
) {
    // C++: std::unordered_set<point_om_omt> visited;
    let mut visited = HashSet::new();

    // C++: const half_open_rectangle<point_om_omt> omap_bounds(
    //        point_om_omt(0, 0), point_om_omt(OMAPX, OMAPY) );
    let omap_bounds = (0i32, 0i32, OMAP_DIM, OMAP_DIM);

    // C++: for (int y = 0; y < OMAPY; y++) { for (int x = 0; x < OMAPX; x++) { ... } }
    for y in 0..OMAP_DIM {
        for x in 0..OMAP_DIM {
            let checked = (x, y);

            // C++: if (visited.find(checked) != visited.end()) { continue; }
            if visited.contains(&checked) {
                continue;
            }

            // C++: bool enclosed = true;
            let mut enclosed = true;

            // C++: const auto is_unchecked = [&enclosed, &omap_bounds, this](
            //          const point_om_omt &pt) { ... };
            // We use a manual Vec-based flood fill instead of the C++ lambda,
            // because Rust's closure borrowing conflicts with `visited`.
            let mut area: Vec<(i32, i32)> = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(checked);
            visited.insert(checked);

            while let Some(pt) = queue.pop_front() {
                // C++: if (city_tiles.find(pt) != city_tiles.end()) { return false; }
                if city_tiles.contains(&pt) {
                    continue;
                }

                // C++: if (!omap_bounds.contains(pt)) { enclosed = false; return false; }
                if pt.0 < omap_bounds.0
                    || pt.0 >= omap_bounds.2
                    || pt.1 < omap_bounds.1
                    || pt.1 >= omap_bounds.3
                {
                    enclosed = false;
                    continue;
                }

                area.push(pt);

                // C++: 4-connected flood fill
                for &(dx, dy) in &FOUR_ADJACENT_OFFSETS {
                    let np = (pt.0 + dx, pt.1 + dy);
                    if !visited.contains(&np) {
                        visited.insert(np);
                        queue.push_back(np);
                    }
                }
            }

            // C++: if (!enclosed) { continue; }
            if !enclosed {
                continue;
            }

            // C++: city_tiles.reserve(city_tiles.size() + area.size());
            // C++: for (const point_om_omt &pt : area) { city_tiles.insert(pt); }
            for pt in &area {
                city_tiles.insert(*pt);
                // Also set the terrain to road_nesw so the grid write-back
                // picks it up as a city-zone tile.
                if let Some(road_nesw) = registry.handle_by_id(OTER_ROAD_NESW) {
                    let current = ter(grid, pt.0, pt.1);
                    if !registry.flags_for(current).contains(TerrainFlags::RIVER)
                        && !registry.flags_for(current).contains(TerrainFlags::HIGHWAY)
                        && !registry
                            .flags_for(current)
                            .contains(TerrainFlags::IMPASSABLE)
                    {
                        // We don't set terrain here — we mark the city tile and
                        // the main system will handle terrain assignment during flush.
                    }
                }
            }
        }
    }
}

// ===========================================================================
// build_cities — public system (entry point)
// ===========================================================================

/// C++: `void overmap::build_cities()` (overmap_city.cpp L214-233)
///
/// For each city:
/// 1. Pick a random starting direction.
/// 2. Walk the 4 cardinal directions (turning right each time).
/// 3. For each direction, call `build_city_street` (which recursively branches).
/// 4. After all streets are built, flood-fill areas fully enclosed by roads
///    and mark them as city tiles.
/// 5. Flush the terrain grid back to chunk entities.
pub fn build_cities(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    city_tiles: Option<ResMut<CityTiles>>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);

    // -----------------------------------------------------------------------
    // Build dense [[TerrainHandle; OMAP_DIM]; OMAP_DIM] grid from chunks at z=0
    // -----------------------------------------------------------------------
    let omap_size = OMAP_DIM as usize;
    let mut grid_box: Box<[[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize]> =
        Box::new([[TerrainHandle::NULL; OMAP_DIM as usize]; OMAP_DIM as usize]);
    let grid: &mut [[TerrainHandle; OMAP_DIM as usize]; OMAP_DIM as usize] = &mut *grid_box;

    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = ox + lx as i32;
                let gy = oy + ly as i32;
                if gx >= 0 && gx < OMAP_DIM && gy >= 0 && gy < OMAP_DIM {
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    grid[gx as usize][gy as usize] = chunk.terrain[idx];
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Bail out if road terrain isn't available
    // -----------------------------------------------------------------------
    if registry.handle_by_id(OTER_ROAD_NESW).is_none() {
        info!("road_nesw terrain missing, skipping city street grid");
        return;
    }

    // -----------------------------------------------------------------------
    // Prepare state
    // -----------------------------------------------------------------------
    let connection = ConnectionStub;
    let mut ct_set = city_tiles
        .as_ref()
        .map(|ct| ct.tiles.clone())
        .unwrap_or_default();
    let mut placed_unique_buildings: HashSet<String> = HashSet::new();

    // -----------------------------------------------------------------------
    // C++: for (const city &c : cities) { ... }
    // -----------------------------------------------------------------------
    for city in &cities {
        let cx = city.omt_x;
        let cy = city.omt_y;
        let town_center = (cx, cy);
        let original_size = city.size as i32;

        // C++: const om_direction::type start_dir = om_direction::random();
        let start_dir = OmDirection::random(&mut rng);
        // C++: om_direction::type cur_dir = start_dir;
        let mut cur_dir = start_dir;

        // C++: do { build_city_street(...); } while ((cur_dir = turn_right(cur_dir)) != start_dir);
        loop {
            build_city_street(
                &connection,
                (cx, cy),
                city.size as i32,
                cur_dir,
                town_center,
                original_size,
                grid,
                &mut rng,
                &registry,
                &mut ct_set,
                &mut placed_unique_buildings,
                &settings,
                2, // default block_width
            );
            cur_dir = cur_dir.turn_right();
            if cur_dir == start_dir {
                break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // C++: flood_fill_city_tiles();
    // -----------------------------------------------------------------------
    flood_fill_city_tiles(&mut ct_set, grid, &registry);

    // -----------------------------------------------------------------------
    // Write grid back to chunk entities
    // -----------------------------------------------------------------------
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = ox + lx as i32;
                let gy = oy + ly as i32;
                if gx < 0 || gx >= OMAP_DIM || gy < 0 || gy >= OMAP_DIM {
                    continue;
                }
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let new_handle = grid[gx as usize][gy as usize];
                if new_handle != new_terrain[idx] && new_handle != TerrainHandle::NULL {
                    new_terrain[idx] = new_handle;
                    modified = true;
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

    // -----------------------------------------------------------------------
    // Update CityTiles resource (if it was provided)
    // -----------------------------------------------------------------------
    if let Some(mut ct) = city_tiles {
        ct.tiles = ct_set;
        info!("Cities built: {} tiles", ct.tiles.len());
    } else {
        info!(
            "Cities built: {} tiles (CityTiles not available)",
            ct_set.len()
        );
    }
}
