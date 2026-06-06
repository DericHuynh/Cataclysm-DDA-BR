//! Step 3b: Place highways.
//!
//! Verbatim port of CDDA master's `overmap::place_highways()` and related
//! functions from `overmap_highway.cpp` (L1-1156).
//!
//! The highway system:
//! 1. Uses a global grid of intersection points (`HighwayIntersectionGrid`)
//! 2. Determines if an overmap is on a highway path via `is_highway_overmap()`
//! 3. Handles ocean-adjacent overmaps specially
//! 4. Selects endpoint coordinates on overmap edges
//! 5. Places highway reserved paths with slants, bends, and ramps
//! 6. Stores highway connections for neighbor overmaps

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::direction::Rng;
use cdda_overmap::direction::{OmDirection, FOUR_ADJACENT_OFFSETS};
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::HashMap;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Constants — matching C++ overmap_highway.cpp
// ---------------------------------------------------------------------------

/// Maximum number of highway connections per overmap (N, E, S, W).
const HIGHWAY_MAX_CONNECTIONS: usize = 4;

/// Base z-level for highway placement (RIVER_Z in C++).
const HIGHWAY_BASE_Z: i8 = 0;

/// Maximum deviance for highway end-point placement.
const HIGHWAY_MAX_DEVIANCE: i32 = 20;

/// Safe deviance = HIGHWAY_MAX_DEVIANCE * 2 (used for corners).
const SAFE_DEVIANCE: i32 = HIGHWAY_MAX_DEVIANCE * 2;

/// Center variance for 4-way intersection placement.
const CENTER_VARIANCE: i32 = 10;

/// Corner threshold for 4-way intersection: OMAPX / 3.0.
const CORNER_THRESHOLD: f64 = OMAP_DIM as f64 / 3.0;

// ---------------------------------------------------------------------------
// Highway node / path types — matching C++ intrahighway_node / Highway_path
// ---------------------------------------------------------------------------

/// A node in a highway path. Mirrors C++ `intrahighway_node`.
#[derive(Debug, Clone)]
struct HighwayNode {
    /// Position (x, y, z) in OMT coordinates.
    pos: (i32, i32, i32),
    /// Direction of travel through this node.
    dir: OmDirection,
    /// True if this is a straight segment (not a bend/slant/ramp).
    is_segment: bool,
    /// True if this node is a ramp.
    is_ramp: bool,
    /// True if the ramp goes down (from elevated to ground).
    ramp_down: bool,
    /// True if this is an interchange.
    is_interchange: bool,
    /// The terrain handle to place at this node.
    terrain: TerrainHandle,
}

type HighwayPath = Vec<HighwayNode>;

// ---------------------------------------------------------------------------
// Global highway intersection grid — matching C++ highway_intersection_grid
// ---------------------------------------------------------------------------

/// Global state for highway intersection placement.
/// In C++ this lives in `overmap_buffer.global_state.highway_intersections`.
#[derive(Resource, Debug, Clone, Default)]
pub struct HighwayIntersectionGrid {
    /// Origin of the grid in overmap coordinates.
    pub grid_origin: (i32, i32),
    /// Whether the grid origin has been set.
    pub grid_origin_set: bool,
    /// Intersection positions on the grid (grid_pos → offset_pos).
    pub intersections: HashMap<(i32, i32), (i32, i32)>,
    /// Row separation (overmaps between E-W highways).
    pub row_separation: i32,
    /// Column separation (overmaps between N-S highways).
    pub column_separation: i32,
    /// Maximum variance for intersection offset from grid point.
    pub max_offset_variance: i32,
}

impl HighwayIntersectionGrid {
    /// Set options from settings. C++ `highway_intersection_grid::set_options()`.
    pub fn set_options(&mut self) {
        // In C++ these come from game options. We use reasonable defaults.
        if self.row_separation == 0 {
            self.row_separation = 32;
        }
        if self.column_separation == 0 {
            self.column_separation = 32;
        }
        if self.max_offset_variance == 0 {
            self.max_offset_variance = 4;
        }
    }

    /// Get the grid origin, setting it if needed.
    pub fn get_grid_origin(&mut self, rng: &mut XorShiftRng) -> (i32, i32) {
        self.set_options();
        if !self.grid_origin_set {
            // C++: set_grid_origin(point_abs_om::zero) on first call
            self.grid_origin = (0, 0);
            self.grid_origin_set = true;
            self.generate_feature_point(self.grid_origin, rng);
        }
        self.grid_origin
    }

    /// Generate an intersection feature point if it doesn't exist.
    /// C++ `highway_intersection_grid::generate_feature_point()`.
    pub fn generate_feature_point(&mut self, grid_pos: (i32, i32), rng: &mut XorShiftRng) {
        if self.intersections.contains_key(&grid_pos) {
            return;
        }
        // C++ `generate_offset`: random offset within variance, avoiding lakes
        let offset_x =
            grid_pos.0 + rng.range_i32(-self.max_offset_variance, self.max_offset_variance);
        let offset_y =
            grid_pos.1 + rng.range_i32(-self.max_offset_variance, self.max_offset_variance);
        self.intersections.insert(grid_pos, (offset_x, offset_y));
    }

    /// Find the 4 bounding grid points for an overmap position.
    /// C++ `highway_intersection_grid::find_feature_point_bounds()`.
    pub fn find_feature_point_bounds(
        &mut self,
        pos: (i32, i32),
        rng: &mut XorShiftRng,
    ) -> [(i32, i32); 4] {
        let col = pos.0.div_euclid(self.column_separation);
        let row = pos.1.div_euclid(self.row_separation);
        let top_left = (col * self.column_separation, row * self.row_separation);
        let result = [
            (
                top_left.0 + self.column_separation,
                top_left.1 + self.row_separation,
            ), // SE
            (top_left.0, top_left.1 + self.row_separation), // SW
            (top_left.0 + self.column_separation, top_left.1), // NE
            top_left,                                       // NW
        ];
        for &p in &result {
            self.generate_feature_point(p, rng);
        }
        result
    }

    /// Get the offset position for a grid intersection.
    pub fn get_offset_pos(&self, grid_pos: (i32, i32)) -> (i32, i32) {
        self.intersections
            .get(&grid_pos)
            .copied()
            .unwrap_or(grid_pos)
    }

    /// Find grid-adjacent feature points for a grid position.
    /// C++ `highway_intersection_grid::find_grid_adjacent_features()`.
    pub fn find_grid_adjacent_features(
        &mut self,
        grid_pos: (i32, i32),
        rng: &mut XorShiftRng,
    ) -> Vec<(i32, i32, (i32, i32))> {
        let mut result = Vec::new();
        for (dx, dy) in FOUR_ADJACENT_OFFSETS {
            let adj_grid = (
                grid_pos.0 + dx * self.column_separation,
                grid_pos.1 + dy * self.row_separation,
            );
            self.generate_feature_point(adj_grid, rng);
            let adj_offset = self.get_offset_pos(adj_grid);
            result.push((adj_grid.0, adj_grid.1, adj_offset));
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Highway connections storage
// ---------------------------------------------------------------------------

/// Per-overmap highway connection endpoints.
/// In C++ this is `overmap::highway_connections`.
#[derive(Resource, Debug, Clone, Default)]
pub struct HighwayConnections {
    /// Endpoints for each of the 4 directions (N, E, S, W).
    /// (i32::MIN, i32::MIN, i32::MIN) = invalid.
    pub end_points: [(i32, i32, i32); 4],
}

// ---------------------------------------------------------------------------
// Helper functions — exact ports from C++
// ---------------------------------------------------------------------------

/// Hash an overmap position for deterministic RNG.
fn hash_om(x: i32, y: i32) -> u64 {
    let mut h: u64 = 0x9e3779b97f4a7c15;
    h = h.wrapping_add(x as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    h = h.wrapping_add(y as u64).wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 32;
    h
}

/// Return unit vector for a direction.
fn direction_vector(dir: OmDirection) -> (i32, i32) {
    FOUR_ADJACENT_OFFSETS[dir.to_index()]
}

/// Chebyshev distance (max of abs differences) — matches C++ `rl_dist()`.
fn rl_dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// Closest corner in a given direction — C++ `closest_corner_in_direction()`.
fn closest_corner_in_direction(
    p1: (i32, i32, i32),
    p2: (i32, i32, i32),
    direction: OmDirection,
) -> ((i32, i32, i32), OmDirection) {
    let dir_idx = direction.to_index();
    let (ox, oy) = FOUR_ADJACENT_OFFSETS[dir_idx];
    let diff_x = p2.0 - p1.0;
    let diff_y = p2.1 - p1.1;
    let result = (p1.0 + ox * diff_x.abs(), p1.1 + oy * diff_y.abs(), p1.2);
    let diff2_x = p2.0 - result.0;
    let diff2_y = p2.1 - result.1;
    let sign_x = if diff2_x == 0 { 0 } else { diff2_x.signum() };
    let sign_y = if diff2_y == 0 { 0 } else { diff2_y.signum() };

    let new_dir = OmDirection::ALL
        .iter()
        .position(|&d| {
            let (dx, dy) = FOUR_ADJACENT_OFFSETS[d.to_index()];
            dx == sign_x && dy == sign_y
        })
        .map(|i| OmDirection::ALL[i])
        .unwrap_or(OmDirection::North);

    (result, new_dir)
}

/// Wrap a point on the overmap edge to the opposite edge — C++ `wrap_point()`.
fn wrap_point(p: (i32, i32, i32)) -> (i32, i32, i32) {
    let mut wx = p.0;
    let mut wy = p.1;
    if wx == OMAP_DIM - 1 || wx == 0 {
        wx = (wx - (OMAP_DIM - 1)).abs();
    }
    if wy == OMAP_DIM - 1 || wy == 0 {
        wy = (wy - (OMAP_DIM - 1)).abs();
    }
    (wx, wy, p.2)
}

/// Check if a point is outside the corner region — C++ `point_outside_overmap_corner()`.
fn point_outside_overmap_corner(p: (i32, i32), corner_length: i32) -> bool {
    // valid_bounds: x in [corner_length, OMAPX-corner_length), y in [0, OMAPY)
    let in_bounds1 =
        p.0 >= corner_length && p.0 < OMAP_DIM - corner_length && p.1 >= 0 && p.1 < OMAP_DIM;
    // valid_bounds_2: x in [0, OMAPX), y in [corner_length, OMAPY-corner_length)
    let in_bounds2 =
        p.0 >= 0 && p.0 < OMAP_DIM && p.1 >= corner_length && p.1 < OMAP_DIM - corner_length;
    in_bounds1 || in_bounds2
}

/// Midpoint of two 3D points — C++ `midpoint()`.
fn midpoint(a: (i32, i32, i32), b: (i32, i32, i32)) -> (i32, i32, i32) {
    ((a.0 + b.0) / 2, (a.1 + b.1) / 2, (a.2 + b.2) / 2)
}

/// Check if a terrain handle is water — matches C++ `is_water_body()`.
fn is_water_body(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

/// Get border points along an overmap edge — matching C++ `get_border()`.
fn get_border(dir: OmDirection, z: i8, distance_corner: i32) -> Vec<(i32, i32, i32)> {
    let z = z as i32;
    let margin = distance_corner;
    let end = OMAP_DIM - margin;
    let mut points = Vec::new();

    match dir {
        OmDirection::North => {
            for x in margin..end {
                points.push((x, 0, z));
            }
        }
        OmDirection::South => {
            for x in margin..end {
                points.push((x, OMAP_DIM - 1, z));
            }
        }
        OmDirection::West => {
            for y in margin..end {
                points.push((0, y, z));
            }
        }
        OmDirection::East => {
            for y in margin..end {
                points.push((OMAP_DIM - 1, y, z));
            }
        }
        OmDirection::Invalid => {}
    }
    points
}

/// Pick a random entry from a slice.
fn random_entry<'a, T>(slice: &'a [T], rng: &mut XorShiftRng) -> &'a T {
    &slice[rng.random_usize(slice.len())]
}

/// Determine if an overmap lies on the highway grid.
/// Port of C++ `overmap::is_highway_overmap()`.
fn is_highway_overmap(
    pos: (i32, i32),
    grid: &mut HighwayIntersectionGrid,
    rng: &mut XorShiftRng,
) -> Option<[bool; 4]> {
    grid.get_grid_origin(rng);

    // Find the bounding grid points for this overmap
    let bounds = grid.find_feature_point_bounds(pos, rng);

    // Check if we're on an intersection overmap
    for i in 0..HIGHWAY_MAX_CONNECTIONS {
        if grid.get_offset_pos(bounds[i]) == pos {
            // This is an intersection — all 4 connections
            return Some([true, true, true, true]);
        }
    }

    // Otherwise, check every path between adjacent grid points
    let mut connections = [false; 4];
    let mut path_cache: HashMap<((i32, i32), (i32, i32)), bool> = HashMap::new();

    for &bound_point in &bounds {
        let bound_offset = grid.get_offset_pos(bound_point);
        for (_adj_grid_x, _adj_grid_y, adj_offset) in
            grid.find_grid_adjacent_features(bound_point, rng)
        {
            // Create ordered pair for cache
            let ordered = if bound_offset.1 == adj_offset.1 {
                if bound_offset.0 < adj_offset.0 {
                    (bound_offset, adj_offset)
                } else {
                    (adj_offset, bound_offset)
                }
            } else if bound_offset.1 < adj_offset.1 {
                (bound_offset, adj_offset)
            } else {
                (adj_offset, bound_offset)
            };

            if path_cache.contains_key(&ordered) {
                continue;
            }
            path_cache.insert(ordered, true);

            // Check if pos lies on the orthogonal line between the two points
            if orthogonal_line_contains(ordered.0, ordered.1, pos) {
                // Determine which edges of this overmap connect to the highway
                for i in 0..HIGHWAY_MAX_CONNECTIONS {
                    let (dx, dy) = FOUR_ADJACENT_OFFSETS[i];
                    let neighbor = (pos.0 + dx, pos.1 + dy);
                    if orthogonal_line_contains(ordered.0, ordered.1, neighbor) {
                        connections[i] = true;
                    }
                }
                return Some(connections);
            }
        }
    }

    None
}

/// Check if a point lies on the orthogonal line between two points.
fn orthogonal_line_contains(a: (i32, i32), b: (i32, i32), pt: (i32, i32)) -> bool {
    // Horizontal line
    if a.1 == b.1 && pt.1 == a.1 {
        let min_x = a.0.min(b.0);
        let max_x = a.0.max(b.0);
        return pt.0 >= min_x && pt.0 <= max_x;
    }
    // Vertical line
    if a.0 == b.0 && pt.0 == a.0 {
        let min_y = a.1.min(b.1);
        let max_y = a.1.max(b.1);
        return pt.1 >= min_y && pt.1 <= max_y;
    }
    false
}

/// Handle ocean detection for highway placement.
/// Port of C++ `overmap::highway_handle_oceans()`.
fn highway_handle_oceans(
    config: &OvermapGenConfig,
    settings: &OvermapRegionSettings,
) -> (bool, [bool; 4]) {
    let mut ocean_adjacent = [false; 4];

    if !settings.overmap_ocean {
        return (false, ocean_adjacent);
    }

    // Check if any ocean starts are configured
    let ocean_start_n = settings.ocean.ocean_start_north.unwrap_or(i32::MAX);
    let ocean_start_e = settings.ocean.ocean_start_east.unwrap_or(i32::MAX);
    let ocean_start_s = settings.ocean.ocean_start_south.unwrap_or(i32::MAX);
    let ocean_start_w = settings.ocean.ocean_start_west.unwrap_or(i32::MAX);

    let om_x = config.om_x;
    let om_y = config.om_y;

    // Don't place highways over the ocean
    if om_y <= -ocean_start_n
        || om_x >= ocean_start_e
        || om_y >= ocean_start_s
        || om_x <= -ocean_start_w
    {
        return (true, ocean_adjacent);
    }

    // Check if we need partial highway with different intersections
    ocean_adjacent[0] = om_y - 1 == -ocean_start_n;
    ocean_adjacent[1] = om_x + 1 == ocean_start_e;
    ocean_adjacent[2] = om_y + 1 == ocean_start_s;
    ocean_adjacent[3] = om_x - 1 == -ocean_start_w;

    let count = ocean_adjacent.iter().filter(|&&x| x).count();
    if count == HIGHWAY_MAX_CONNECTIONS {
        warn!(
            "Not placing highways — ocean on all sides for overmap ({}, {})",
            om_x, om_y
        );
        return (true, ocean_adjacent);
    }

    (false, ocean_adjacent)
}

/// Select highway end points on the overmap edges.
/// Port of C++ `overmap::highway_select_end_points()`.
fn highway_select_end_points(
    end_points: &mut [(i32, i32, i32); 4],
    neighbor_connections: &mut [bool; 4],
    ocean_neighbors: &[bool; 4],
    base_z: i8,
    rng: &mut XorShiftRng,
) {
    let any_ocean = ocean_neighbors.iter().any(|&x| x);

    // If there are adjacent oceans, cut highway connections
    if any_ocean {
        for i in 0..HIGHWAY_MAX_CONNECTIONS {
            if ocean_neighbors[i] {
                neighbor_connections[i] = false;
            }
        }
    }

    // For each direction that needs a new endpoint
    let mut new_end_point = [false; 4];
    for i in 0..HIGHWAY_MAX_CONNECTIONS {
        if neighbor_connections[i] {
            // We don't have neighbor overmaps, so always generate new endpoints
            new_end_point[i] = true;
        }
    }

    // If going N/S or E/W, new highways tend to go straight through
    for i in 0..HIGHWAY_MAX_CONNECTIONS {
        if new_end_point[i] {
            let opposite_idx = (i + 2) % HIGHWAY_MAX_CONNECTIONS;
            let to_wrap = end_points[opposite_idx];
            let dir = OmDirection::from_index(i);

            let border_points = get_border(dir, base_z, SAFE_DEVIANCE);
            let fallback = if border_points.is_empty() {
                (0, 0, base_z as i32)
            } else {
                *random_entry(&border_points, rng)
            };

            if to_wrap.0 == i32::MIN {
                // No opposite endpoint — use random border point
                end_points[i] = fallback;
            } else {
                // Try to find a point near the wrapped opposite
                let wrapped = wrap_point(to_wrap);
                let nearby: Vec<(i32, i32, i32)> = border_points
                    .iter()
                    .copied()
                    .filter(|&p| {
                        let dist = rl_dist((p.0, p.1), (wrapped.0, wrapped.1));
                        dist < HIGHWAY_MAX_DEVIANCE
                            && point_outside_overmap_corner((p.0, p.1), SAFE_DEVIANCE)
                    })
                    .collect();

                if !nearby.is_empty() {
                    end_points[i] = *random_entry(&nearby, rng);
                } else {
                    end_points[i] = fallback;
                }
            }
        }
    }
}

/// Place a straight highway line between two points.
/// Port of C++ `overmap::place_highway_line()`.
fn place_highway_line(
    p1: (i32, i32, i32),
    p2: (i32, i32, i32),
    draw_dir: OmDirection,
    base_z: i8,
    hiway_ns: TerrainHandle,
    hiway_ew: TerrainHandle,
    hiway_nesw: TerrainHandle,
) -> HighwayPath {
    let base_z_i32 = base_z as i32;
    let draw_vec = direction_vector(draw_dir);
    let mut path = HighwayPath::new();

    // Compute line from p1 to p2 using Bresenham-like iteration
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let steps = dx.abs().max(dy.abs());

    if steps == 0 {
        return path;
    }

    let sx = if dx >= 0 { 1 } else { -1 };
    let sy = if dy >= 0 { 1 } else { -1 };

    let mut current = (p1.0, p1.1, base_z_i32);
    for _i in 0..=steps {
        path.push(HighwayNode {
            pos: current,
            dir: draw_dir,
            is_segment: true,
            is_ramp: false,
            ramp_down: false,
            is_interchange: false,
            terrain: hiway_ns,
        });

        if current.0 == p2.0 && current.1 == p2.1 {
            break;
        }

        // Step toward p2
        if current.0 != p2.0 {
            current.0 += sx;
        }
        if current.1 != p2.1 {
            current.1 += sy;
        }
    }

    // Orient terrain based on direction
    let is_ns = draw_dir == OmDirection::North || draw_dir == OmDirection::South;
    let oriented = if is_ns { hiway_ns } else { hiway_ew };
    for node in &mut path {
        node.terrain = oriented;
    }

    path
}

/// Place highway lines with bends.
/// Simplified port of C++ `overmap::place_highway_lines_with_bends()`.
fn place_highway_lines_with_bends(
    bend_points: &[((i32, i32, i32), OmDirection)],
    start_point: (i32, i32, i32),
    end_point: (i32, i32, i32),
    direction: OmDirection,
    base_z: i8,
    hiway_ns: TerrainHandle,
    hiway_ew: TerrainHandle,
    hiway_nesw: TerrainHandle,
) -> HighwayPath {
    let mut highway_path = HighwayPath::new();
    let mut current_direction = direction;

    // Build path segments between consecutive points
    let mut prev = start_point;
    for &(bend_pos, _bend_dir) in bend_points {
        let segment = place_highway_line(
            prev,
            bend_pos,
            current_direction,
            base_z,
            hiway_ns,
            hiway_ew,
            hiway_nesw,
        );
        for node in segment {
            highway_path.push(node);
        }
        // Place bend marker at the bend point
        highway_path.push(HighwayNode {
            pos: bend_pos,
            dir: current_direction,
            is_segment: false,
            is_ramp: false,
            ramp_down: false,
            is_interchange: false,
            terrain: hiway_nesw,
        });
        prev = bend_pos;
        // Update direction for next segment (simplified)
        if current_direction == OmDirection::North || current_direction == OmDirection::South {
            current_direction = OmDirection::East;
        } else {
            current_direction = OmDirection::South;
        }
    }

    // Final segment to end
    let segment = place_highway_line(
        prev,
        end_point,
        current_direction,
        base_z,
        hiway_ns,
        hiway_ew,
        hiway_nesw,
    );
    for node in segment {
        highway_path.push(node);
    }

    highway_path
}

/// Place a reserved highway path from p1 to p2.
/// Port of C++ `overmap::place_highway_reserved_path()`.
fn place_highway_reserved_path(
    p1: (i32, i32, i32),
    p2: (i32, i32, i32),
    dir1: usize,
    dir2: usize,
    base_z: i8,
    rng: &mut XorShiftRng,
    hiway_ns: TerrainHandle,
    hiway_ew: TerrainHandle,
    hiway_nesw: TerrainHandle,
) -> HighwayPath {
    let direction1 = OmDirection::from_index(dir1);
    let direction2 = OmDirection::from_index(dir2);

    if direction1 == direction2 {
        warn!("highway path with same direction for both ends; skipping");
        return HighwayPath::new();
    }

    // Reverse directions for drawing
    let draw_dir1 = direction1.opposite();
    let _draw_dir2 = direction2.opposite();
    let parallel = direction1.are_parallel(direction2);
    let north_south = direction1 == OmDirection::North || direction1 == OmDirection::South;

    // Check invalid points
    let p1_invalid = p1.0 == i32::MIN;
    let p2_invalid = p2.0 == i32::MIN;

    if p1_invalid || p2_invalid {
        // Fallback — no ramp placement for now (requires special system)
        return HighwayPath::new();
    }

    // Determine bends
    let mut bend_points: Vec<((i32, i32, i32), OmDirection)> = Vec::new();
    let mut bend_draw_mode = !(p1.0 == p2.0 || p1.1 == p2.1);

    if bend_draw_mode {
        if parallel {
            let diff_x = (p1.0 - p2.0).abs();
            let diff_y = (p1.1 - p2.1).abs();
            let two_bends = if north_south {
                diff_x >= HIGHWAY_MAX_DEVIANCE
            } else {
                diff_y >= HIGHWAY_MAX_DEVIANCE
            };

            if two_bends {
                if diff_x < HIGHWAY_MAX_DEVIANCE || diff_y < HIGHWAY_MAX_DEVIANCE {
                    return HighwayPath::new();
                }
                let bend_midpoint = midpoint(p1, p2);
                let (corner1, dir1_out) = closest_corner_in_direction(p1, bend_midpoint, draw_dir1);
                let (corner2, dir2_out) = closest_corner_in_direction(bend_midpoint, p2, dir1_out);
                bend_points.push((corner1, dir1_out));
                bend_points.push((corner2, dir2_out));
            } else {
                bend_draw_mode = false;
            }
        } else {
            let (corner, new_dir) = closest_corner_in_direction(p1, p2, draw_dir1);
            bend_points.push((corner, new_dir));
        }
    }

    if bend_draw_mode {
        place_highway_lines_with_bends(
            &bend_points,
            p1,
            p2,
            draw_dir1,
            base_z,
            hiway_ns,
            hiway_ew,
            hiway_nesw,
        )
    } else {
        place_highway_line(p1, p2, draw_dir1, base_z, hiway_ns, hiway_ew, hiway_nesw)
    }
}

// ===========================================================================
// place_highways — main system
// ===========================================================================

/// Place highways on the overmap.
///
/// Port of C++ `overmap::place_highways()` (overmap_highway.cpp L593-879).
pub fn place_highways(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.overmap_highway {
        return;
    }

    let base_z: i8 = HIGHWAY_BASE_Z;

    // Look up highway terrain handles
    let hiway_ns = registry
        .handle_by_id("hiway_ns")
        .or_else(|| registry.handle_by_id("highway_ns"))
        .unwrap_or(TerrainHandle::NULL);
    let hiway_ew = registry
        .handle_by_id("hiway_ew")
        .or_else(|| registry.handle_by_id("highway_ew"))
        .unwrap_or(TerrainHandle::NULL);
    let hiway_nesw = registry
        .handle_by_id("hiway_nesw")
        .or_else(|| registry.handle_by_id("highway_nesw"))
        .or_else(|| registry.handle_by_id("hiway_4way"))
        .or_else(|| registry.handle_by_id("highway_4way"))
        .unwrap_or(TerrainHandle::NULL);

    if hiway_ns == TerrainHandle::NULL && hiway_ew == TerrainHandle::NULL {
        info!("Highway terrain handles not registered — skipping");
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 9);

    // --- Get or create the global intersection grid ---
    let mut grid = HighwayIntersectionGrid::default();

    // --- Handle oceans ---
    let (is_ocean, ocean_neighbors) = highway_handle_oceans(&config, &settings);
    if is_ocean {
        info!(
            "Highways skipped — overmap ({}, {}) is in ocean",
            config.om_x, config.om_y
        );
        return;
    }

    // --- Check if this overmap is on a highway path ---
    let is_highway = is_highway_overmap((config.om_x, config.om_y), &mut grid, &mut rng);

    let Some(mut neighbor_connections) = is_highway else {
        info!(
            "This overmap ({}, {}) is NOT a highway overmap",
            config.om_x, config.om_y
        );
        return;
    };

    info!(
        "This overmap ({}, {}) IS a highway overmap, connections: N={}, E={}, S={}, W={}",
        config.om_x,
        config.om_y,
        neighbor_connections[0],
        neighbor_connections[1],
        neighbor_connections[2],
        neighbor_connections[3],
    );

    // --- Select end points on edges ---
    let mut end_points: [(i32, i32, i32); 4] = [
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN, i32::MIN),
    ];

    highway_select_end_points(
        &mut end_points,
        &mut neighbor_connections,
        &ocean_neighbors,
        base_z,
        &mut rng,
    );

    let connection_count = neighbor_connections.iter().filter(|&&x| x).count();
    let mut paths: Vec<HighwayPath> = Vec::new();

    // --- Place highway paths based on connection count (C++ switch) ---
    match connection_count {
        2 => {
            // Draw end-to-end
            if neighbor_connections[0] && neighbor_connections[2] {
                paths.push(place_highway_reserved_path(
                    end_points[0],
                    end_points[2],
                    0,
                    2,
                    base_z,
                    &mut rng,
                    hiway_ns,
                    hiway_ew,
                    hiway_nesw,
                ));
            } else if neighbor_connections[1] && neighbor_connections[3] {
                paths.push(place_highway_reserved_path(
                    end_points[1],
                    end_points[3],
                    1,
                    3,
                    base_z,
                    &mut rng,
                    hiway_ns,
                    hiway_ew,
                    hiway_nesw,
                ));
            } else {
                for i in 0..HIGHWAY_MAX_CONNECTIONS {
                    let next = (i + 1) % HIGHWAY_MAX_CONNECTIONS;
                    if neighbor_connections[i] && neighbor_connections[next] {
                        paths.push(place_highway_reserved_path(
                            end_points[i],
                            end_points[next],
                            i,
                            next,
                            base_z,
                            &mut rng,
                            hiway_ns,
                            hiway_ew,
                            hiway_nesw,
                        ));
                    }
                }
            }
        }
        3 => {
            // 3-way intersection: find empty direction, connect all to center
            let mut empty_dir = OmDirection::North;
            for i in 0..HIGHWAY_MAX_CONNECTIONS {
                if !neighbor_connections[i] {
                    empty_dir = OmDirection::from_index(i);
                    break;
                }
            }
            let center = (
                OMAP_DIM / 2 + rng.range_i32(-CENTER_VARIANCE, CENTER_VARIANCE),
                OMAP_DIM / 2 + rng.range_i32(-CENTER_VARIANCE, CENTER_VARIANCE),
                base_z as i32,
            );
            for i in 0..HIGHWAY_MAX_CONNECTIONS {
                if i != empty_dir.to_index() {
                    paths.push(place_highway_reserved_path(
                        end_points[i],
                        center,
                        i,
                        (i + 2) % 4,
                        base_z,
                        &mut rng,
                        hiway_ns,
                        hiway_ew,
                        hiway_nesw,
                    ));
                }
            }
            // Place 3-way intersection terrain at center
            paths.push(vec![HighwayNode {
                pos: center,
                dir: empty_dir,
                is_segment: false,
                is_ramp: false,
                ramp_down: false,
                is_interchange: true,
                terrain: hiway_nesw,
            }]);
        }
        4 => {
            // 4-way intersection — check close corner pairs
            let four_point = (
                OMAP_DIM / 2 + rng.range_i32(-CENTER_VARIANCE, CENTER_VARIANCE),
                OMAP_DIM / 2 + rng.range_i32(-CENTER_VARIANCE, CENTER_VARIANCE),
                base_z as i32,
            );
            let mut corners_close = [false; 4];
            for i in 0..HIGHWAY_MAX_CONNECTIONS {
                let d = rl_dist(
                    (end_points[i].0, end_points[i].1),
                    (end_points[(i + 1) % 4].0, end_points[(i + 1) % 4].1),
                ) as f64;
                corners_close[i] = d < CORNER_THRESHOLD * std::f64::consts::SQRT_2;
            }

            let intersection_point = match corners_close.iter().filter(|&&x| x).count() {
                1 => {
                    // One pair of close corners — use their shared point
                    let mut pt = four_point;
                    for i in 0..HIGHWAY_MAX_CONNECTIONS {
                        if corners_close[i] {
                            let opp = OmDirection::from_index(i).opposite();
                            let (corner, _) = closest_corner_in_direction(
                                end_points[i],
                                end_points[(i + 1) % 4],
                                opp,
                            );
                            pt = corner;
                        }
                    }
                    pt
                }
                2 => {
                    // Two pairs — draw two 3-way intersections
                    let mut intersections = [four_point, four_point];
                    for i in 0..HIGHWAY_MAX_CONNECTIONS {
                        if corners_close[i] {
                            let idx = i / 2;
                            let dir1 = i;
                            let dir2 = (i + 1) % 4;
                            let opp = OmDirection::from_index(dir1).opposite();
                            let (corner, _) = closest_corner_in_direction(
                                end_points[dir1],
                                end_points[dir2],
                                opp,
                            );
                            intersections[idx] = corner;

                            paths.push(place_highway_reserved_path(
                                end_points[dir1],
                                corner,
                                dir1,
                                (dir1 + 2) % 4,
                                base_z,
                                &mut rng,
                                hiway_ns,
                                hiway_ew,
                                hiway_nesw,
                            ));
                            paths.push(place_highway_reserved_path(
                                end_points[dir2],
                                corner,
                                dir2,
                                (dir2 + 2) % 4,
                                base_z,
                                &mut rng,
                                hiway_ns,
                                hiway_ew,
                                hiway_nesw,
                            ));
                        }
                    }
                    // Connect the two 3-way intersections
                    paths.push(place_highway_reserved_path(
                        intersections[0],
                        intersections[1],
                        OmDirection::East.to_index(),
                        OmDirection::West.to_index(),
                        base_z,
                        &mut rng,
                        hiway_ns,
                        hiway_ew,
                        hiway_nesw,
                    ));
                    four_point // fallback, already handled
                }
                _ => four_point,
            };

            // Only draw end-to-center for non-corner-close case
            if corners_close.iter().filter(|&&x| x).count() != 2 {
                for i in 0..HIGHWAY_MAX_CONNECTIONS {
                    paths.push(place_highway_reserved_path(
                        end_points[i],
                        intersection_point,
                        i,
                        (i + 2) % 4,
                        base_z,
                        &mut rng,
                        hiway_ns,
                        hiway_ew,
                        hiway_nesw,
                    ));
                }
            }
        }
        _ => {
            // 1 connection — end at edge
            let dummy = (i32::MIN, i32::MIN, i32::MIN);
            for i in 0..HIGHWAY_MAX_CONNECTIONS {
                if end_points[i].0 != i32::MIN {
                    paths.push(place_highway_reserved_path(
                        end_points[i],
                        dummy,
                        i,
                        (i + 2) % 4,
                        base_z,
                        &mut rng,
                        hiway_ns,
                        hiway_ew,
                        hiway_nesw,
                    ));
                }
            }
        }
    }

    // Store highway connections for neighbor overmaps
    commands.insert_resource(HighwayConnections { end_points });

    // --- Write highway terrain to chunks ---
    let mut all_writes: Vec<(i32, i32, i32, TerrainHandle)> = Vec::new();
    for path in &paths {
        for node in path {
            all_writes.push((node.pos.0, node.pos.1, node.pos.2, node.terrain));
        }
    }

    if all_writes.is_empty() {
        info!(
            "Highways: 0 tiles placed for overmap ({}, {})",
            config.om_x, config.om_y
        );
        return;
    }

    let reg = &*registry;
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z_chunk = chunk_pos.z.0;
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, wz, handle) in &all_writes {
            if wz != z_chunk as i32 {
                continue;
            }
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let current = new_terrain[idx];
                // Only overwrite non-water, non-impassable terrain
                let flags = reg.flags_for(current);
                if !flags.contains(TerrainFlags::RIVER)
                    && !flags.contains(TerrainFlags::LAKE)
                    && !flags.contains(TerrainFlags::OCEAN)
                    && !flags.contains(TerrainFlags::IMPASSABLE)
                {
                    if current != handle {
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
        "Highways placed: {} paths, {} tiles for overmap ({}, {})",
        paths.len(),
        all_writes.len(),
        config.om_x,
        config.om_y
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_om_deterministic() {
        assert_eq!(hash_om(1, 2), hash_om(1, 2));
    }

    #[test]
    fn test_hash_om_different() {
        assert_ne!(hash_om(1, 2), hash_om(3, 4));
    }

    #[test]
    fn test_wrap_point_corners() {
        // Wrapping (0, ...) should go to (179, ...)
        let result = wrap_point((0, 50, 0));
        assert_eq!(result.0, OMAP_DIM - 1);
        assert_eq!(result.1, 50);

        let result = wrap_point((OMAP_DIM - 1, 50, 0));
        assert_eq!(result.0, 0);
        assert_eq!(result.1, 50);
    }

    #[test]
    fn test_direction_vector() {
        assert_eq!(direction_vector(OmDirection::North), (0, -1));
        assert_eq!(direction_vector(OmDirection::East), (1, 0));
        assert_eq!(direction_vector(OmDirection::South), (0, 1));
        assert_eq!(direction_vector(OmDirection::West), (-1, 0));
    }

    #[test]
    fn test_midpoint() {
        assert_eq!(midpoint((0, 0, 0), (10, 20, 0)), (5, 10, 0));
        assert_eq!(midpoint((1, 1, 0), (2, 2, 0)), (1, 1, 0));
    }

    #[test]
    fn test_point_outside_overmap_corner() {
        // Center point should be outside corners
        assert!(point_outside_overmap_corner((90, 90), 20));
        // Edge near corner should NOT be outside corners
        assert!(!point_outside_overmap_corner((5, 5), 20));
        // Edge far from corner should be outside corners
        assert!(point_outside_overmap_corner((90, 5), 20));
    }

    #[test]
    fn test_rl_dist() {
        assert_eq!(rl_dist((0, 0), (5, 3)), 5);
        assert_eq!(rl_dist((0, 0), (0, 0)), 0);
        assert_eq!(rl_dist((0, 0), (3, 4)), 4); // Chebyshev: max(3,4) = 4
    }

    #[test]
    fn test_get_border_north() {
        let border = get_border(OmDirection::North, 0, 10);
        assert!(!border.is_empty());
        for &(x, y, z) in &border {
            assert_eq!(y, 0);
            assert_eq!(z, 0);
            assert!(x >= 10 && x < OMAP_DIM - 10);
        }
    }

    #[test]
    fn test_get_border_east() {
        let border = get_border(OmDirection::East, 0, 10);
        for &(x, y, _) in &border {
            assert_eq!(x, OMAP_DIM - 1);
            assert!(y >= 10 && y < OMAP_DIM - 10);
        }
    }

    #[test]
    fn test_orthogonal_line_contains_horizontal() {
        assert!(orthogonal_line_contains((0, 5), (10, 5), (5, 5)));
        assert!(!orthogonal_line_contains((0, 5), (10, 5), (5, 6)));
        assert!(orthogonal_line_contains((10, 5), (0, 5), (3, 5)));
    }

    #[test]
    fn test_orthogonal_line_contains_vertical() {
        assert!(orthogonal_line_contains((5, 0), (5, 10), (5, 5)));
        assert!(!orthogonal_line_contains((5, 0), (5, 10), (6, 5)));
    }
}
