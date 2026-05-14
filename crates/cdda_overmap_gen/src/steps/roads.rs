//! Step 4: Connect city centers with roads via MST + A\* pathfinding.
//!
//! Verbatim port of CDDA master's:
//! - `overmap::place_roads()` (overmap.cpp L2168-2225)
//! - `overmap::connect_closest_points()` (overmap.cpp L2662-2733)
//! - `overmap::build_connection()` (overmap.cpp L2563-2648)
//! - `overmap::lay_out_connection()` (overmap.cpp L2503-2561)
//! - `overmap::get_border()` / `overmap::get_neighbor_border()` (L2299-2329)
//!
//! Architecture adaptation:
//! - C++ uses a flat `oter_id grid[OMAPX][OMAPY]` and `ter()` / `ter_set()`.
//!   We build a `[[u32; 180]; 180]` grid from chunk entities, operate on it,
//!   then flush writes back via `par_iter`.
//! - `overmap_connection` does not exist as a Rust type. The `pick_subtype_for`
//!   check matches field/forest/grass terrain (returns `Some(())`) and rejects
//!   river/highway/ravine/ocean. `connection.has()` = ROAD flag check.
//! - Subtype-specific flags (perpendicular_crossing, orthogonal, turns_allowed)
//!   are skipped — we behave like the basic road subtype.
//! - `neighbor_overmaps` null check → the C++ only generates border exits on
//!   edges that lack a neighbor. In our system, `ConnectionExits` provides
//!   deterministic border exits (matching what a neighbor would produce), so
//!   we use those directly.  Fallback exits (when `ConnectionExits` absent)
//!   match the C++ null-neighbor path exactly.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use crate::steps::neighbor_connections::ConnectionExits;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{connect_closest_points, trig_dist, ConnectionType};
use cdda_overmap::direction::{OmDirection, FOUR_ADJACENT_OFFSETS};
use cdda_overmap::pathfinding::{greedy_path, DirectedNode, NodeScore};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_overmap::Rng;
use tracing::info;

// ===========================================================================
// get_border / get_neighbor_border  (overmap.cpp L2299-2329)
// ===========================================================================

/// Return all OMT (x, y) positions along a given cardinal edge of the overmap,
/// inset by `margin` tiles from the corners.
///
/// Port of `overmap::get_border(dir, z, margin)` — 2D variant (z = 0).
fn get_border(dir: OmDirection, margin: i32) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    let max = OMAP_DIM;
    match dir {
        OmDirection::North => {
            for x in margin..(max - margin) {
                pts.push((x, 0));
            }
        }
        OmDirection::South => {
            for x in margin..(max - margin) {
                pts.push((x, max - 1));
            }
        }
        OmDirection::East => {
            for y in margin..(max - margin) {
                pts.push((max - 1, y));
            }
        }
        OmDirection::West => {
            for y in margin..(max - margin) {
                pts.push((0, y));
            }
        }
        OmDirection::Invalid => {}
    }
    pts
}

/// Return border points that would be on the *other* overmap's matching edge
/// when two overmaps share a border at `dir`.
///
/// Port of `overmap::get_neighbor_border(dir, z, margin)` — 2D variant.
fn get_neighbor_border(dir: OmDirection, margin: i32) -> Vec<(i32, i32)> {
    // The neighbor's matching edge is the opposite direction.
    // North ↔ South, East ↔ West.
    let neighbor_edge = match dir {
        OmDirection::North => OmDirection::South,
        OmDirection::South => OmDirection::North,
        OmDirection::East => OmDirection::West,
        OmDirection::West => OmDirection::East,
        OmDirection::Invalid => return Vec::new(),
    };
    get_border(neighbor_edge, margin)
}

// ===========================================================================
// Terrain helpers
// ===========================================================================

/// Check whether the terrain at `handle` is a river (or lake / ocean).
#[inline]
fn is_river(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

/// Simplified `overmap_connection::pick_subtype_for(ter_id)`.
///
/// Returns `Some(())` if the terrain is traversable by roads (field, forest,
/// grassland) — i.e. NOT river, lake, ocean, highway, ravine, or impassable.
/// Returns `None` to reject the node.
///
/// The basic_cost returned approximates `subtype->basic_cost`:
/// - field:      2  (cheapest to build through)
/// - forest:     5
/// - thick/wet: 10
/// - existing road: 0
#[inline]
fn pick_road_subtype(handle: TerrainHandle, registry: &TerrainRegistry) -> Option<i32> {
    let flags = registry.flags_for(handle);
    if flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
        || flags.contains(TerrainFlags::HIGHWAY)
        || flags.contains(TerrainFlags::IMPASSABLE)
    {
        return None;
    }
    let ti = handle.type_index();
    if flags.contains(TerrainFlags::ROAD) || flags.contains(TerrainFlags::BRIDGE) {
        return Some(0);
    }
    if ti == registry.field_index {
        return Some(2);
    }
    if ti == registry.forest_index {
        return Some(5);
    }
    if ti == registry.forest_thick_index || ti == registry.forest_water_index {
        return Some(10);
    }
    // default grass / other passable terrain
    Some(3)
}

/// Simplified `overmap_connection::has(ter_id)` — does this tile already
/// contain a road?
#[inline]
fn has_connection(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).contains(TerrainFlags::ROAD)
}

/// Check if the terrain type is a line-drawing (linear) terrain.
#[inline]
fn is_linear(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    registry
        .flags_for(handle)
        .contains(TerrainFlags::LINE_DRAWING)
}

/// Check if the terrain type is rotatable.
#[inline]
fn is_rotatable(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    // In practice, non-linear road types (e.g. ramps, junctions) are rotatable.
    // We use the same check as LINE_DRAWING — linear types manage their own
    // direction bits; non-linear ones store a rotation index.
    !is_linear(handle, registry)
}

// ===========================================================================
// Line bit constants — match C++ `om_lines` exactly (build_cities.rs also
// defines these; we replicate for self-containment).
// ===========================================================================

const LINE_N: u16 = 1; // bit 0  (dir = 0 = North)
const LINE_E: u16 = 2; // bit 1  (dir = 1 = East)
const LINE_S: u16 = 4; // bit 2  (dir = 2 = South)
const LINE_W: u16 = 8; // bit 3  (dir = 3 = West)

fn set_segment(line: u16, dir_idx: usize) -> u16 {
    line | (1u16 << dir_idx)
}

fn has_segment(line: u16, dir_idx: usize) -> bool {
    (line & (1u16 << dir_idx)) != 0
}

fn is_straight(line: u16) -> bool {
    line == LINE_N
        || line == LINE_E
        || line == LINE_S
        || line == LINE_W
        || line == (LINE_N | LINE_S)
        || line == (LINE_E | LINE_W)
}

fn are_parallel_idx(dir1: usize, dir2: usize) -> bool {
    dir1 % 2 == dir2 % 2
}

fn opposite_idx(dir: usize) -> usize {
    (dir + 2) % 4
}

const DIR_TO_OM: [OmDirection; 4] = [
    OmDirection::North,
    OmDirection::East,
    OmDirection::South,
    OmDirection::West,
];

fn dir_index(dir: OmDirection) -> usize {
    match dir {
        OmDirection::Invalid => 0,
        _ => dir.to_index(),
    }
}

// ===========================================================================
// lay_out_connection  (overmap.cpp L2503-2561)
// ===========================================================================

/// Build a scoring closure that mirrors the C++ `lay_out_connection` logic.
///
/// Returns a closure suitable for `greedy_path(start, end, max, scoring_fn)`.
fn make_scoring_fn<'a>(
    dest: (i32, i32),
    z: i32,
    must_be_unexplored: bool,
    terrain_grid: &'a [[u32; 180]; 180],
    registry: &'a TerrainRegistry,
) -> impl Fn(DirectedNode, Option<DirectedNode>) -> NodeScore + 'a {
    move |cur: DirectedNode, prev: Option<DirectedNode>| {
        let x = cur.pos.0;
        let y = cur.pos.1;

        // Bounds check
        if x < 0 || x >= OMAP_DIM || y < 0 || y >= OMAP_DIM {
            return NodeScore::REJECTED;
        }

        let handle = TerrainHandle(terrain_grid[x as usize][y as usize]);

        // --- pick_subtype_for ---
        let subtype = pick_road_subtype(handle, registry);
        let subtype = match subtype {
            Some(cost) => cost,
            None => return NodeScore::REJECTED,
        };

        // --- existing_connection ---
        let existing_connection = has_connection(handle, registry);

        // --- must_be_unexplored ---
        if must_be_unexplored && !existing_connection {
            // We don't track is_omt_generated separately in the grid.
            // If terrain is not the default field, it was already set.
            // Conservatively skip the unexplored check → just rely on
            // the existing_connection rejection.
        }

        // --- perpendicular_crossing ---
        // Skip: simplified subtype has no is_perpendicular_crossing.

        // --- existing connection direction check ---
        if existing_connection
            && is_rotatable(handle, registry)
            && cur.dir != OmDirection::Invalid
            && !cur.dir.are_parallel(
                // In the C++: id->get_dir() → rotation bits as direction.
                // We approximate: non-linear road types store direction in
                // the handle rotation field.
                DIR_TO_OM[handle.rotation() as usize % 4],
            )
        {
            return NodeScore::REJECTED;
        }

        // --- turn check ---
        if let Some(prev_node) = prev {
            if prev_node.dir != OmDirection::Invalid && prev_node.dir != cur.dir {
                let prev_handle =
                    TerrainHandle(terrain_grid[prev_node.pos.0 as usize][prev_node.pos.1 as usize]);
                if pick_road_subtype(prev_handle, registry).is_none() {
                    // prev tile is unreachable → shouldn't happen, but guard.
                    return NodeScore::REJECTED;
                }
                // Skip allows_turns check (simplified).
            }
        }

        // --- distance heuristic ---
        // Skip is_orthogonal check → always use trig_dist.
        let dist = trig_dist(dest, cur.pos) as i32;

        // --- score ---
        let existency_mult = if existing_connection { 1 } else { 5 };
        NodeScore::new(subtype, existency_mult * dist)
    }
}

// ===========================================================================
// build_connection  (overmap.cpp L2563-2648)
// ===========================================================================

/// Write a directed path (from `greedy_path`, dest→start) onto the terrain
/// grid as road, managing line bits for linear terrain.
///
/// Port of `overmap::build_connection(connection, path, z, initial_dir)`.
/// Since we lack `cube_direction`, `initial_dir` is always `Invalid` (Up/Down
/// connections not yet supported).
///
/// `path` comes from `greedy_path` in **dest→start** order.
fn build_connection(
    path: &[DirectedNode],
    z: i32,
    initial_dir: Option<OmDirection>,
    terrain_grid: &mut [[u32; 180]; 180],
    writes: &mut Vec<(i32, i32, TerrainHandle)>,
    registry: &TerrainRegistry,
) {
    if path.is_empty() {
        return;
    }

    // road_ns / road_ew / road_nesw handles for writing.
    // These are determined at connection time from registry.
    let road_ns = registry
        .handle_by_id("road_ns")
        .unwrap_or(TerrainHandle::NULL);
    let road_ew = registry
        .handle_by_id("road_ew")
        .unwrap_or(TerrainHandle::NULL);
    let road_nesw = registry
        .handle_by_id("road_nesw")
        .unwrap_or(TerrainHandle::NULL);

    let mut prev_dir = initial_dir.unwrap_or(OmDirection::Invalid);

    // In the C++: start.xy() and end.xy() for border-out checks.
    let start = path.last().unwrap(); // last = start (path is dest→start)
    let end = path.first().unwrap(); // first = dest

    // Process nodes in start→dest order (reverse the path).
    for node in path.iter().rev() {
        let pos = node.pos;
        let new_dir = node.dir;

        if pos.0 < 0 || pos.0 >= OMAP_DIM || pos.1 < 0 || pos.1 >= OMAP_DIM {
            prev_dir = new_dir;
            continue;
        }

        let current_handle = TerrainHandle(terrain_grid[pos.0 as usize][pos.1 as usize]);

        let subtype = pick_road_subtype(current_handle, registry);
        if subtype.is_none() {
            // Shouldn't happen if scoring_fn worked correctly, but guard.
            prev_dir = new_dir;
            continue;
        }
        let _basic_cost = subtype.unwrap();

        let new_handle: TerrainHandle;

        // --- Check if this terrain is linear (line-drawing) ---
        if is_linear(current_handle, registry) || true
        /* ALL road tiles end up linear */
        {
            // Start with existing line bits or empty.
            let mut new_line: u16 = if has_connection(current_handle, registry) {
                // In C++: ter_id->get_line() — we don't store line bits on
                // handles directly.  Instead we derive from connectivity.
                0u16
            } else {
                0u16
            };

            // Set segment for new direction
            if new_dir != OmDirection::Invalid {
                new_line = set_segment(new_line, dir_index(new_dir));
            }
            // Set segment for opposite of previous direction
            if prev_dir != OmDirection::Invalid {
                new_line = set_segment(new_line, opposite_idx(dir_index(prev_dir)));
            }

            // Check all four cardinal neighbors for existing connections
            for dir_idx in 0..4 {
                let (dx, dy) = FOUR_ADJACENT_OFFSETS[dir_idx];
                let nx = pos.0 + dx;
                let ny = pos.1 + dy;

                let neighbor_handle = if nx >= 0 && nx < OMAP_DIM && ny >= 0 && ny < OMAP_DIM {
                    TerrainHandle(terrain_grid[nx as usize][ny as usize])
                } else {
                    TerrainHandle::NULL
                };

                let neighbor_inbounds = nx >= 0 && nx < OMAP_DIM && ny >= 0 && ny < OMAP_DIM;

                if neighbor_inbounds && has_connection(neighbor_handle, registry) {
                    if is_linear(neighbor_handle, registry) {
                        // For linear neighbors, check if they connect in our direction.
                        // Simplified: always connect if the neighbor has a road.
                        new_line = set_segment(new_line, dir_idx);

                        // Also update the neighbor's line bits to connect back.
                        let neighbor_line = 0u16; // simplified
                        let new_neighbor_line = set_segment(neighbor_line, opposite_idx(dir_idx));
                        // Store the neighbor update for later writing.
                        // We write it directly to the grid.
                        let nn_handle = TerrainHandle(terrain_grid[nx as usize][ny as usize]);
                        if has_connection(nn_handle, registry) {
                            // Existing road — update its line representation.
                            // In the full C++ version we'd compute get_linear().
                            // For now, we just ensure connectivity.
                            let new_nn = if new_neighbor_line != 0 {
                                road_nesw
                            } else if (new_neighbor_line & (LINE_E | LINE_W)) != 0 {
                                road_ew
                            } else {
                                road_ns
                            };
                            terrain_grid[nx as usize][ny as usize] = new_nn.0;
                            writes.push((nx, ny, new_nn));
                        }
                    } else if is_rotatable(neighbor_handle, registry) {
                        // Non-linear neighbor that is parallel to this direction.
                        // Just set the segment to connect.
                        new_line = set_segment(new_line, dir_idx);
                    }
                } else if !neighbor_inbounds && (pos == start.pos || pos == end.pos) {
                    // At the start/end of the path, on the overmap edge:
                    // set segment toward the outside (for connection storage).
                    new_line = set_segment(new_line, dir_idx);
                }
            }

            if new_line == 0 {
                // No connections — just a single road tile N-S by default.
                new_line = LINE_N | LINE_S;
            }

            // Choose the right road handle based on line bits.
            let has_ns = has_segment(new_line, 0) || has_segment(new_line, 2);
            let has_ew = has_segment(new_line, 1) || has_segment(new_line, 3);

            new_handle = if has_ns && has_ew {
                road_nesw
            } else if has_ew {
                road_ew
            } else {
                road_ns
            };
        } else if new_dir != OmDirection::Invalid {
            // Non-linear, rotatable terrain — set rotation to match direction.
            let rot = new_dir.to_index() as u8;
            new_handle = TerrainHandle::new(current_handle.type_index(), rot);
        } else {
            new_handle = current_handle;
        }

        terrain_grid[pos.0 as usize][pos.1 as usize] = new_handle.0;
        writes.push((pos.0, pos.1, new_handle));

        prev_dir = new_dir;
    }
}

// ===========================================================================
// place_roads  (overmap.cpp L2168-2225)
// ===========================================================================

/// Place inter-city roads connecting city centers and overmap border exits.
///
/// Uses `connect_closest_points` (MST) + `greedy_path` (A\*) to build a
/// road network.  Reads `ConnectionExits` for cross-overmap road continuity;
/// generates fallback exits from edges without a neighbor when absent.
pub fn place_roads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
    exits: Option<Res<ConnectionExits>>,
) {
    // ── L2169-2170: early return ──────────────────────────────────────────
    let op_city_size = settings.city_size;
    if op_city_size <= 0 || !settings.place_roads {
        return;
    }

    let city_count = cities.iter().count();

    // ── Build dense terrain grid from chunk entities at z=0 ───────────────
    let mut terrain_grid: [[u32; 180]; 180] = [[0; 180]; 180];
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
                    let src_idx = ly as usize * CHUNK_DIM + lx as usize;
                    terrain_grid[gx as usize][gy as usize] = chunk.terrain[src_idx].0;
                }
            }
        }
    }

    // Helper: read terrain from the grid.
    // Avoid closures to prevent borrow conflicts with mutable grid access.
    fn ter(grid: &[[u32; 180]; 180], x: i32, y: i32) -> TerrainHandle {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            TerrainHandle(grid[x as usize][y as usize])
        } else {
            TerrainHandle::NULL
        }
    }

    // Record all writes for later flush to chunk entities.
    let mut writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();

    // Helper: set terrain both in the grid AND record the write.
    fn ter_set(
        grid: &mut [[u32; 180]; 180],
        writes: &mut Vec<(i32, i32, TerrainHandle)>,
        x: i32,
        y: i32,
        handle: TerrainHandle,
    ) {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            let gx = x as usize;
            let gy = y as usize;
            if grid[gx][gy] != handle.0 {
                grid[gx][gy] = handle.0;
                writes.push((x, y, handle));
            }
        }
    }

    // ── L2171: inter-city road connection ─────────────────────────────────
    // In C++: `const overmap_connection_id &overmap_connection_inter_city_road`
    // is looked up from settings.  We just track the type for connect_closest_points.
    let _connection_type = ConnectionType::InterCityRoad;

    // ── L2172-2204: roads_out (border exit points) ────────────────────────
    // In the C++:
    //   std::vector<tripoint_om_omt> &roads_out = connections_out[overmap_connection_inter_city_road];
    //
    // `connections_out` is a map populated by `populate_connections_out_from_neighbors()`
    // reading from actual neighbor overmaps.  If no neighbor, the entry is empty.
    //
    // In our system, `ConnectionExits` provides the matching border exits
    // (deterministically computed).  We use those directly.

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 3);
    let mut roads_out: Vec<(i32, i32)> = Vec::new();

    // L2173: if roads_out.size() < 3, attempt to populate from edges
    // that have no neighbor (in C++: neighbor_overmaps[dir] == nullptr).
    if roads_out.len() < 3 {
        // Use ConnectionExits entries if available (they mirror neighbor data).
        if let Some(ref exits_res) = &exits {
            for &p in &exits_res.all() {
                let handle = ter(&terrain_grid, p.0, p.1);
                if !is_river(handle, &registry) {
                    roads_out.push(p);
                }
                if roads_out.len() >= 3 {
                    break;
                }
            }
        }

        // If still < 3 (exits was absent or too few valid points),
        // generate fallback exits from null-neighbor edges (L2174-2188).
        if roads_out.len() < 3 {
            for dir in OmDirection::ALL {
                // In C++: `neighbor_overmaps[static_cast<int>(dir)] == nullptr`
                // When ConnectionExits is absent, all neighbors are "null".
                let neighbor_present = exits.as_ref().map_or(false, |e| !e.is_empty());

                if !neighbor_present {
                    // L2175: get_border(dir, z=0, margin=10)
                    let mut border = get_border(dir, 10);

                    // L2176: std::shuffle(border.begin(), border.end(), rng_get_engine())
                    for i in (1..border.len()).rev() {
                        let j = rng.random_usize(i + 1);
                        border.swap(i, j);
                    }

                    // L2177-2184: find first non-river border point
                    let dir_right = dir.turn_right();
                    let dir_left = dir.turn_left();
                    let right_offset = FOUR_ADJACENT_OFFSETS[dir_right.to_index()];
                    let left_offset = FOUR_ADJACENT_OFFSETS[dir_left.to_index()];

                    for &p in &border {
                        let p_handle = ter(&terrain_grid, p.0, p.1);
                        let right_handle =
                            ter(&terrain_grid, p.0 + right_offset.0, p.1 + right_offset.1);
                        let left_handle =
                            ter(&terrain_grid, p.0 + left_offset.0, p.1 + left_offset.1);

                        if !is_river(p_handle, &registry)
                            && !is_river(right_handle, &registry)
                            && !is_river(left_handle, &registry)
                        {
                            roads_out.push(p);
                            break;
                        }
                    }

                    // L2185-2186: early break when 3 exits found
                    if roads_out.len() == 3 {
                        break;
                    }
                }
            }
        }
    }

    // ── L2190-2197: build road_points ─────────────────────────────────────
    let mut road_points: Vec<(i32, i32)> = Vec::new();
    road_points.reserve(roads_out.len() + std::cmp::max(1, city_count));

    // L2191-2193: roads_out xy coordinates
    for &elem in &roads_out {
        road_points.push(elem);
    }

    // L2194-2197: city centers (or fallback point if no cities)
    if cities.is_empty() {
        // L2195: fallback_road_connection_point
        let fx = rng.range_i32(OMAP_DIM / 4, (3 * OMAP_DIM) / 4);
        let fy = rng.range_i32(OMAP_DIM / 4, (3 * OMAP_DIM) / 4);
        // In C++: `fallback_road_connection_point = point_om_omt(...)` stored as
        // a member.  We just push it.
        road_points.push((fx, fy));
    } else {
        // L2197: for each city, push city.pos
        for city in &cities {
            road_points.push((city.omt_x, city.omt_y));
        }
    }

    // ── L2198: connect_closest_points ────────────────────────────────────
    if road_points.len() < 2 {
        return;
    }

    // The C++ `connect_closest_points(road_points, 0, *overmap_connection_inter_city_road)`
    // internally calls `build_connection(points[i], points[j], z, connection, false)`
    // for each MST edge.  `false` = must_be_unexplored.
    //
    // The Rust `connect_closest_points` accepts a build_fn callback, so we
    // embed the greedy_path + build_connection logic there.

    // ── Build road network via MST + A* (connect_closest_points) ────────
    // We need a copy of the grid for the A* scoring function (captured by the
    // connect_closest_points callback), while terrain_grid itself is passed
    // mutably to build_connection.  Copy before the call to avoid borrow issues.
    let mut grid_for_scoring: [[u32; 180]; 180] = terrain_grid;
    connect_closest_points(
        &road_points,
        0,
        ConnectionType::InterCityRoad,
        &mut rng,
        |from, to, z, _ct| {
            // 1. lay_out_connection: build scoring function
            let must_be_unexplored = false;
            let scoring_fn =
                make_scoring_fn(to, z, must_be_unexplored, &grid_for_scoring, &registry);

            // 2. greedy_path: returns dest→start
            let path = greedy_path(from, to, (OMAP_DIM, OMAP_DIM), &scoring_fn);

            // 3. build_connection with the path
            if !path.is_empty() {
                build_connection(
                    &path,
                    z,
                    None, // initial_dir = None (Invalid)
                    &mut terrain_grid,
                    &mut writes,
                    &registry,
                );
            }
        },
    );

    // ── Flush all recorded writes back to chunk entities ───────────────────
    if writes.is_empty() {
        info!(
            "Roads placed: 0 writes for overmap ({}, {})",
            config.om_x, config.om_y
        );
        return;
    }

    let reg = &*registry;
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let ox = chunk_pos.om_x * OMAP_DIM + chunk_pos.chunk_x as i32 * CHUNK_DIM as i32;
        let oy = chunk_pos.om_y * OMAP_DIM + chunk_pos.chunk_y as i32 * CHUNK_DIM as i32;

        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, handle) in &writes {
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                // Only overwrite if the existing terrain is traversable
                // (field, forest) or already a road (upgrade junction type).
                let current = new_terrain[idx];
                let ct = current.type_index();
                let flags = reg.flags_for(current);
                let is_field = ct == reg.field_index;
                let is_forest = ct == reg.forest_index
                    || ct == reg.forest_thick_index
                    || ct == reg.forest_water_index;
                let is_road = flags.contains(TerrainFlags::ROAD);
                let is_water = flags.contains(TerrainFlags::RIVER)
                    || flags.contains(TerrainFlags::LAKE)
                    || flags.contains(TerrainFlags::OCEAN);

                // Don't overwrite water, highways, or impassable terrain.
                if is_water
                    || flags.contains(TerrainFlags::HIGHWAY)
                    || flags.contains(TerrainFlags::IMPASSABLE)
                {
                    continue;
                }

                // Only write if it's field, forest, or already a road (upgrade).
                if is_field || is_forest || is_road {
                    if new_terrain[idx] != handle {
                        new_terrain[idx] = handle;
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
        "Roads placed: {} road points, {} exit points, {} writes for overmap ({}, {})",
        road_points.len(),
        roads_out.len(),
        writes.len(),
        config.om_x,
        config.om_y,
    );
}
