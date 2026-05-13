//! Step: River generation — Bezier curves, meandering, shore building.
//!
//! Port of CDDA master's `overmap_water.cpp` rivers section:
//! - `place_rivers()` (L544-689)
//! - `place_river()` (L47-226)
//! - `river_meander()` (L228-256)
//! - `build_river_shores()` (L731-791)
//! - `polish_river()` (overmap.cpp L2735-2742)
//! - `setup_adjacent_river()` (L692-728) — simplified, no neighbor overmaps
//!
//! ## Implementation notes
//!
//! Since we don't have neighbor overmaps in Bevy yet, river start/end
//! points are always on this overmap's border edges. River continuity
//! across overmap boundaries will be added when neighbor overmap access
//! is available.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_core_types::rng::SeededRng;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::inbounds_omt;
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use tracing::info;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum distance from overmap edge where rivers can start/end.
/// Port of C++ `RIVER_BORDER` (overmap.h L174).
const RIVER_BORDER: i32 = 10;

/// Maximum number of major rivers per overmap.
const MAX_RIVERS: usize = 2;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A river node — marks a river's start and end points on this overmap.
///
/// Spawned as an entity so downstream systems can query river locations
/// for bridge placement, floodplain buffering, etc.
#[derive(Component, Debug, Clone)]
pub struct RiverNode {
    /// OMT coordinates of the river start (entry point).
    pub start: (i32, i32),
    /// OMT coordinates of the river end (exit point).
    pub end: (i32, i32),
}

/// Borrowed river settings extracted from `OvermapRegionSettings` at
/// the start of generation.
#[derive(Debug, Clone)]
struct RiverSettings {
    /// Raw river scale from region settings. 0 = no rivers.
    river_scale: u32,
    /// 1-in-N chance for a river branch to spawn.
    river_branch_chance: i32,
    /// 1-in-N chance for a branch to remerge with the main river.
    river_branch_remerge_chance: i32,
    /// How much the river scale decreases per branch level.
    river_branch_scale_decrease: i32,
}

impl Default for RiverSettings {
    fn default() -> Self {
        Self {
            river_scale: 1,
            river_branch_chance: 5,
            river_branch_remerge_chance: 3,
            river_branch_scale_decrease: 2,
        }
    }
}

impl RiverSettings {
    fn from_region(settings: &OvermapRegionSettings) -> Self {
        Self {
            river_scale: settings.river_scale,
            ..Default::default()
        }
    }

    /// Effective river scale (width). 0 = no rivers.
    /// Port of C++: `river_scale = 1 + std::max(1, river_scale)`.
    fn effective_scale(&self) -> i32 {
        if self.river_scale == 0 {
            0
        } else {
            1 + (self.river_scale as i32).max(1)
        }
    }
}

// ---------------------------------------------------------------------------
// RNG helpers
// ---------------------------------------------------------------------------

/// Generate a random i32 in `[lo, hi]` inclusive (handles reversed bounds).
fn rng_range(rng: &mut SeededRng, lo: i32, hi: i32) -> i32 {
    if lo > hi {
        return rng_range(rng, hi, lo);
    }
    let range = (hi - lo) as u32;
    lo + rng.gen_range(0, range) as i32
}

/// 1-in-N chance.
fn one_in(rng: &mut SeededRng, n: i32) -> bool {
    if n <= 0 {
        return false;
    }
    rng.gen_bool(1.0 / n as f64)
}

/// Generate a random f64 in [0.0, 1.0).
fn rng_f64(rng: &mut SeededRng) -> f64 {
    rng.gen_f64()
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Cubic Bezier curve.
///
/// Port of C++ `cubic_bezier()` from `line.h` L213-227.
/// Returns `n_segs + 1` points along the curve.
fn cubic_bezier(
    pa: (i32, i32),
    pb: (i32, i32),
    pc: (i32, i32),
    pd: (i32, i32),
    n_segs: i32,
) -> Vec<(i32, i32)> {
    let cubic_axis = |a: i32, b: i32, c: i32, d: i32, t: f64| -> i32 {
        // a(1-t)^3 + 3bt(1-t)^2 + 3ct^2(1-t) + dt^3
        let u = 1.0 - t;
        (u.powi(3) * a as f64
            + 3.0 * t * u.powi(2) * b as f64
            + 3.0 * t.powi(2) * u * c as f64
            + t.powi(3) * d as f64) as i32
    };

    let mut pts = Vec::with_capacity((n_segs + 1) as usize);
    for i in 0..=n_segs {
        let t = i as f64 / n_segs as f64;
        pts.push((
            cubic_axis(pa.0, pb.0, pc.0, pd.0, t),
            cubic_axis(pa.1, pb.1, pc.1, pd.1, t),
        ));
    }
    pts
}

/// Bresenham line from `p1` to `p2` (inclusive of both endpoints).
///
/// Port of C++ `line_to()` from `line.cpp` L224-238.
fn line_to(p1: (i32, i32), p2: (i32, i32)) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    let (mut x, mut y) = p1;
    let (x2, y2) = p2;
    let dx = (x2 - x).abs();
    let dy = -(y2 - y).abs();
    let sx: i32 = if x < x2 { 1 } else { -1 };
    let sy: i32 = if y < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        points.push((x, y));
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    points
}

/// Points within a circular radius of `center`.
///
/// Port of C++ `points_in_radius_circ()` from `map_iterator.h` L163-172.
/// Uses Euclidean distance — all integer points where `dist < radius + 0.5`.
fn points_in_radius_circ(center: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    let r2 = (radius as f64 + 0.5).powi(2);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let x = center.0 + dx;
            let y = center.1 + dy;
            if inbounds_omt((x, y)) && ((dx * dx + dy * dy) as f64) < r2 {
                pts.push((x, y));
            }
        }
    }
    pts
}

/// Manhattan / "rldist" distance.
fn rl_dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

// ---------------------------------------------------------------------------
// Cardinal direction offsets
// ---------------------------------------------------------------------------

/// Cardinal offsets in N, E, S, W order (matching C++ `four_adjacent_offsets`).
const FOUR_ADJACENT: [(i32, i32); 4] = [
    (0, -1), // N
    (1, 0),  // E
    (0, 1),  // S
    (-1, 0), // W
];

/// Ordinal (diagonal) offsets in NE, SE, SW, NW order.
const FOUR_ORDINAL: [(i32, i32); 4] = [
    (1, -1),  // NE
    (1, 1),   // SE
    (-1, 1),  // SW
    (-1, -1), // NW
];

// ---------------------------------------------------------------------------
// Terrain helpers
// ---------------------------------------------------------------------------

/// Returns true if the terrain handle has the RIVER flag.
fn is_river(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).contains(TerrainFlags::RIVER)
}

/// Returns true if the terrain is a water body (lake or ocean).
///
/// Port of C++ `is_water_body_not_shore()` — checks for lake/ocean
/// water tiles that rivers should stop at or avoid overwriting.
fn is_water_body(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::LAKE) || flags.contains(TerrainFlags::OCEAN)
}

// ---------------------------------------------------------------------------
// River shore terrain lookup
// ---------------------------------------------------------------------------

/// Map a 4-bit connection mask to the appropriate river shore terrain.
///
/// Port of C++ `build_river_shores()` L757-789.
///
/// Mask bits (matching C++ loop order):
/// - bit 0 (value 1): N neighbor is river
/// - bit 1 (value 2): E neighbor is river
/// - bit 2 (value 4): S neighbor is river
/// - bit 3 (value 8): W neighbor is river
///
/// The 16-entry lookup table maps each connection pattern to a terrain:
///
/// | Mask | Connections | Terrain |
/// |------|-------------|---------|
/// | 0000 | none | forest_water |
/// | 0001 | N | river_south |
/// | 0010 | E | river_west |
/// | 0011 | N+E | river_sw |
/// | 0100 | S | river_north |
/// | 0101 | N+S | forest_water |
/// | 0110 | E+S | river_nw |
/// | 0111 | N+E+S | river_west |
/// | 1000 | W | river_east |
/// | 1001 | N+W | river_se |
/// | 1010 | E+W | forest_water |
/// | 1011 | N+E+W | river_south |
/// | 1100 | S+W | river_ne |
/// | 1101 | N+S+W | river_east |
/// | 1110 | E+S+W | river_north |
/// | 1111 | N+E+S+W | river_center |
fn river_shore_terrain(mask: u8, registry: &TerrainRegistry) -> TerrainHandle {
    let names: [&str; 16] = [
        "forest_water", // 0000
        "river_south",  // 0001 — N only
        "river_west",   // 0010 — E only
        "river_sw",     // 0011 — N+E
        "river_north",  // 0100 — S only
        "forest_water", // 0101 — N+S (unused)
        "river_nw",     // 0110 — E+S
        "river_west",   // 0111 — N+E+S
        "river_east",   // 1000 — W only
        "river_se",     // 1001 — N+W
        "forest_water", // 1010 — E+W (unused)
        "river_south",  // 1011 — N+E+W
        "river_ne",     // 1100 — S+W
        "river_east",   // 1101 — N+S+W
        "river_north",  // 1110 — E+S+W
        "river_center", // 1111 — N+E+S+W
    ];
    registry
        .handle_by_id(names[mask as usize])
        .unwrap_or(TerrainHandle::NULL)
}

/// For mask 15 (all 4 connections), check if any ordinal corner is
/// non-water and return the appropriate trimmed-corner terrain.
///
/// Port of C++ `build_river_shores()` L780-789.
fn trimmed_corner_terrain(
    p: (i32, i32),
    is_river_center: &[[bool; 180]; 180],
    registry: &TerrainRegistry,
) -> Option<TerrainHandle> {
    let names: [&str; 4] = [
        "river_c_not_ne",
        "river_c_not_se",
        "river_c_not_sw",
        "river_c_not_nw",
    ];
    for i in 0..4 {
        let (dx, dy) = FOUR_ORDINAL[i];
        let cx = p.0 + dx;
        let cy = p.1 + dy;
        if inbounds_omt((cx, cy)) && !is_river_center[cx as usize][cy as usize] {
            return registry.handle_by_id(names[i]);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// River meander
// ---------------------------------------------------------------------------

/// Apply random meandering to a river point.
///
/// Port of C++ `river_meander()` L228-256.
///
/// As distance to the river end decreases, the point meanders closer
/// toward the end. Additional random jitter is applied for rivers with
/// scale > 1.
fn river_meander(
    rng: &mut SeededRng,
    river_end: (i32, i32),
    current: &mut (i32, i32),
    river_scale: i32,
) {
    let omap_dim = OMAP_DIM;
    // Port of random_uniform: rng(0, OMAPX*1.2-1) < i
    let random_uniform = |rng: &mut SeededRng, i: i32| -> bool {
        let max = (omap_dim as f64 * 1.2) as i32 - 1;
        rng_range(rng, 0, max) < i
    };
    // Port of random_close: rng(0, OMAPX*0.2-1) > i
    let random_close = |rng: &mut SeededRng, i: i32| -> bool {
        let max = (omap_dim as f64 * 0.2) as i32 - 1;
        if max <= 0 {
            return false;
        }
        rng_range(rng, 0, max) > i
    };

    let abs_dx = (river_end.0 - current.0).abs();
    let abs_dy = (river_end.1 - current.1).abs();

    // As distance decreases, meander closer to river end
    if current.0 != river_end.0
        && (random_uniform(rng, abs_dx) || (random_close(rng, abs_dx) && random_close(rng, abs_dy)))
    {
        if river_end.0 > current.0 {
            current.0 += 1;
        } else {
            current.0 -= 1;
        }
    }
    if current.1 != river_end.1
        && (random_uniform(rng, abs_dy) || (random_close(rng, abs_dy) && random_close(rng, abs_dx)))
    {
        if river_end.1 > current.1 {
            current.1 += 1;
        } else {
            current.1 -= 1;
        }
    }
    // Random jitter for wider rivers
    if river_scale > 1 {
        current.0 += rng_range(rng, -1, 1);
        current.1 += rng_range(rng, -1, 1);
    }
    // Clamp to overmap bounds
    current.0 = current.0.clamp(0, OMAP_DIM - 1);
    current.1 = current.1.clamp(0, OMAP_DIM - 1);
}

// ---------------------------------------------------------------------------
// Single river placement
// ---------------------------------------------------------------------------

/// Place a single river from `start` to `end` using Bezier curves.
///
/// Port of C++ `place_river()` L47-226 (simplified — no neighbor overmaps).
///
/// Returns the river center tiles that were placed.
fn place_single_river(
    rng: &mut SeededRng,
    start: (i32, i32),
    end: (i32, i32),
    river_scale: i32,
    river_settings: &RiverSettings,
    registry: &TerrainRegistry,
    terrain: &mut [[TerrainHandle; 180]; 180],
    river_center_mask: &mut [[bool; 180]; 180],
    current_type_indices: &[[u32; 180]; 180],
) -> Option<RiverNode> {
    let river_center = registry
        .handle_by_id("river_center")
        .unwrap_or(TerrainHandle::NULL);
    if river_center == TerrainHandle::NULL {
        return None;
    }

    let omap_edge = OMAP_DIM - 1;
    let distance = rl_dist(start, end);
    let amplitude = distance / 2;
    let n_segs = distance / 2;

    // Need at least 4 segments
    if n_segs < 4 {
        return None;
    }

    // One-third point: 1/3 of the way from start to end
    let one_third_x = ((end.0 - start.0).abs() as f64 * (1.0 / 3.0)) as i32;
    let one_third_y = ((end.1 - start.1).abs() as f64 * (1.0 / 3.0)) as i32;

    // Control point 1: near the one-third point, perturbed randomly
    let one_third_point = (
        start.0 + one_third_x * (if end.0 >= start.0 { 1 } else { -1 }),
        start.1 + one_third_y * (if end.1 >= start.1 { 1 } else { -1 }),
    );
    let control_p1 = (
        (one_third_point.0 + rng_range(rng, 0, amplitude)).clamp(0, omap_edge),
        (one_third_point.1 + rng_range(rng, 0, amplitude)).clamp(0, omap_edge),
    );

    // Control point 2: near the two-thirds point, perturbed the opposite way
    let two_third_point = (
        start.0 + one_third_x * 2 * (if end.0 >= start.0 { 1 } else { -1 }),
        start.1 + one_third_y * 2 * (if end.1 >= start.1 { 1 } else { -1 }),
    );
    let control_p2 = (
        (two_third_point.0 + rng_range(rng, -amplitude, 0)).clamp(0, omap_edge),
        (two_third_point.1 + rng_range(rng, -amplitude, 0)).clamp(0, omap_edge),
    );

    // Generate Bezier curve segments
    let mut segmented_curve = cubic_bezier(start, control_p1, control_p2, end, n_segs);

    // Remove consecutive duplicates
    segmented_curve.dedup();

    // The Bezier doesn't include the start point, so prepend it
    if segmented_curve.first() != Some(&start) {
        segmented_curve.insert(0, start);
    }

    let curve_size = segmented_curve.len().saturating_sub(1);
    if curve_size < 4 {
        return None;
    }

    // Check first third of the curve: if any point is already water, abort
    let check_limit = (curve_size / 3).min(curve_size);
    let mut river_check_index = 0;
    for i in 0..check_limit {
        let pt = segmented_curve[i];
        if !inbounds_omt(pt) {
            continue;
        }
        let handle = terrain[pt.0 as usize][pt.1 as usize];
        if is_water_body(handle, registry) || is_river(handle, registry) {
            break;
        }
        river_check_index = i + 1;
    }
    if river_check_index == check_limit {
        // First third is all water — abort
        return None;
    }

    // Check remaining points: if water, truncate the river
    let mut actual_end = end;
    let mut actual_curve_size = curve_size;
    for i in river_check_index..curve_size {
        let pt = segmented_curve[i];
        if !inbounds_omt(pt) {
            continue;
        }
        let handle = terrain[pt.0 as usize][pt.1 as usize];
        if is_water_body(handle, registry) {
            actual_end = pt;
            actual_curve_size = i;
            break;
        }
    }

    // Draw the river by filling between consecutive Bezier points
    let mut river_size: usize = 0;
    for i in 0..actual_curve_size {
        let seg_start = segmented_curve[i];
        let seg_end = segmented_curve[i + 1];
        let bezier_segment = line_to(seg_start, seg_end);

        for &bezier_point in &bezier_segment {
            let mut meandered = bezier_point;
            // No meander for first/last segment endpoints
            if i != 0 && i != actual_curve_size - 1 {
                river_meander(rng, actual_end, &mut meandered, river_scale);
            }

            // Draw river in radius [0, river_scale] around the meandered point
            for pt in points_in_radius_circ(meandered, river_scale) {
                let (px, py) = pt;
                let handle = terrain[px as usize][py as usize];
                // Don't overwrite lakes/oceans
                if !is_water_body(handle, registry) {
                    terrain[px as usize][py as usize] = river_center;
                    river_center_mask[px as usize][py as usize] = true;
                    river_size += 1;
                }
            }
        }
    }

    // Create river branches
    let branch_ahead_points = 2.max(actual_curve_size as i32 / 5) as usize;
    let mut branch_last_end: usize = 0;
    for i in 0..actual_curve_size {
        let bezier_point = segmented_curve[i];
        if !inbounds_omt(bezier_point) {
            continue;
        }
        // Only branch if we're far enough from the edge
        let margin = river_scale + 1;
        if bezier_point.0 >= margin
            && bezier_point.0 < OMAP_DIM - margin
            && bezier_point.1 >= margin
            && bezier_point.1 < OMAP_DIM - margin
            && one_in(rng, river_settings.river_branch_chance)
        {
            let mut branch_end = None;

            if i > branch_last_end {
                if one_in(rng, river_settings.river_branch_remerge_chance) {
                    // Re-merge branch: pick a point further along the curve
                    let end_idx = rng_range(
                        rng,
                        (i + branch_ahead_points) as i32,
                        (i + branch_ahead_points * 2) as i32,
                    ) as usize;
                    if end_idx < actual_curve_size {
                        branch_end = Some(segmented_curve[end_idx]);
                        branch_last_end = end_idx;
                    }
                } else {
                    // Random branch: pick a point in a 64-tile radius
                    let rad: i32 = 64;
                    branch_end = Some((
                        rng_range(rng, bezier_point.0 + rad / 2, bezier_point.0 + rad),
                        rng_range(rng, bezier_point.1 + rad / 2, bezier_point.1 + rad),
                    ));
                }
            }

            if let Some(bep) = branch_end {
                if inbounds_omt(bep) {
                    let branch_scale = river_scale - river_settings.river_branch_scale_decrease;
                    if branch_scale > 0 {
                        // Recursive branch placement
                        place_single_river(
                            rng,
                            bezier_point,
                            bep,
                            branch_scale,
                            river_settings,
                            registry,
                            terrain,
                            river_center_mask,
                            current_type_indices,
                        );
                    }
                }
            }
        }
    }

    info!(
        "River placed: start={:?}, end={:?}, tiles={}",
        start, actual_end, river_size
    );

    Some(RiverNode {
        start,
        end: actual_end,
    })
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Main river placement system.
///
/// Port of C++ `place_rivers()` L544-689.
///
/// 1. Reads river scale from region settings
/// 2. Picks start/end points on opposite overmap edges
/// 3. For each river, calls `place_single_river` to generate the Bezier path
/// 4. Writes river_center terrain to affected chunks
pub fn place_rivers(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let river_settings = RiverSettings::from_region(&settings);
    let effective_scale = river_settings.effective_scale();
    if effective_scale == 0 {
        info!("River scale is 0 — skipping river placement");
        return;
    }

    let river_center = registry
        .handle_by_id("river_center")
        .unwrap_or(TerrainHandle::NULL);
    if river_center == TerrainHandle::NULL {
        info!("river_center terrain not registered — skipping");
        return;
    }

    let omap_edge = OMAP_DIM - 1;
    let mut rng = SeededRng::new(config.noise_seed as u64 + 173);

    // Read current terrain into dense arrays from the immutable query
    let mut terrain_dense = [[TerrainHandle::NULL; 180]; 180];
    let mut river_center_mask = [[false; 180]; 180];
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
                    let handle = chunk.get(lx, ly);
                    terrain_dense[gx][gy] = handle;
                    if is_river(handle, &registry) {
                        river_center_mask[gx][gy] = true;
                    }
                }
            }
        }
    }
    let current_type_indices: [[u32; 180]; 180] = {
        let mut arr = [[0u32; 180]; 180];
        for x in 0..180 {
            for y in 0..180 {
                arr[x][y] = terrain_dense[x][y].type_index();
            }
        }
        arr
    };

    // Simplified start/end selection — no neighbor overmaps.
    // For each of up to MAX_RIVERS, pick:
    // - start on N or W edge
    // - end on E or S edge
    //
    // This avoids rivers crossing (N→E, W→S preferred).

    let mut river_nodes: Vec<RiverNode> = Vec::new();

    for _ in 0..MAX_RIVERS {
        // Pick start side: prefer N (dir 0), then W (dir 3)
        let start_side = if rng_f64(&mut rng) < 0.5 { 0 } else { 3 };
        let start = match start_side {
            0 => (
                rng_range(&mut rng, RIVER_BORDER, omap_edge - RIVER_BORDER),
                0,
            ),
            _ => (
                0,
                rng_range(&mut rng, RIVER_BORDER, omap_edge - RIVER_BORDER),
            ),
        };

        // Pick end side: prefer E (dir 1), then S (dir 2)
        let end_side = if rng_f64(&mut rng) < 0.5 { 1 } else { 2 };
        let end = match end_side {
            1 => (
                omap_edge,
                rng_range(&mut rng, RIVER_BORDER, omap_edge - RIVER_BORDER),
            ),
            _ => (
                rng_range(&mut rng, RIVER_BORDER, omap_edge - RIVER_BORDER),
                omap_edge,
            ),
        };

        // Ensure start and end are on different edges (no same-edge rivers)
        if (start.0 == 0 && end.0 == 0) || (start.1 == 0 && end.1 == 0) {
            continue;
        }

        if let Some(node) = place_single_river(
            &mut rng,
            start,
            end,
            effective_scale,
            &river_settings,
            &registry,
            &mut terrain_dense,
            &mut river_center_mask,
            &current_type_indices,
        ) {
            river_nodes.push(node);
        }
    }

    // Write terrain back to chunks via par_iter
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
                if gx < 180 && gy < 180 {
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    let new_val = terrain_dense[gx][gy];
                    if new_terrain[idx] != new_val {
                        new_terrain[idx] = new_val;
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

    // Spawn RiverNode entities
    for node in &river_nodes {
        commands.spawn(node.clone());
    }

    info!(
        "Rivers placed: {} rivers for overmap ({}, {})",
        river_nodes.len(),
        config.om_x,
        config.om_y
    );
}

/// Build river shore tiles around all river center tiles.
///
/// Port of C++ `build_river_shores()` L731-791.
///
/// For each river center tile at z=0, checks the 4-connected neighbors
/// and computes a connection mask. The mask is used to look up the
/// appropriate shore terrain from a 16-entry table.
///
/// Must run after `place_rivers` because it reads `TerrainFlags::RIVER`
/// to identify river tiles.
pub fn build_river_shores(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
) {
    // Read terrain into dense array
    let mut terrain = [[TerrainHandle::NULL; 180]; 180];
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
                    terrain[gx][gy] = chunk.get(lx, ly);
                }
            }
        }
    }

    // Build river center mask: which tiles are actual river (not shores yet)?
    let mut is_river_center = [[false; 180]; 180];
    for x in 0..180 {
        for y in 0..180 {
            is_river_center[x][y] = is_river(terrain[x][y], &registry);
        }
    }

    // For each river tile, compute the connection mask and assign shore terrain.
    // We operate on a copy so that shore assignments don't affect neighbor checks.
    let mut new_terrain = terrain;

    for x in 0..180i32 {
        for y in 0..180i32 {
            if !is_river_center[x as usize][y as usize] {
                continue;
            }

            let mut mask: u8 = 0;
            let mut multiplier: u8 = 1;

            for &(dx, dy) in &FOUR_ADJACENT {
                let nx = x + dx;
                let ny = y + dy;

                // Out-of-bounds neighbors count as river (border continuity)
                if !inbounds_omt((nx, ny)) || is_river(terrain[nx as usize][ny as usize], &registry)
                {
                    mask += multiplier;
                }
                multiplier *= 2;
            }

            // For mask 15 (all 4 connections), check trimmed corners
            if mask == 15 {
                if let Some(trimmed) = trimmed_corner_terrain((x, y), &is_river_center, &registry) {
                    new_terrain[x as usize][y as usize] = trimmed;
                    continue;
                }
            }

            new_terrain[x as usize][y as usize] = river_shore_terrain(mask, &registry);
        }
    }

    // Write back via par_iter
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut chunk_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    let new_val = new_terrain[gx][gy];
                    if chunk_terrain[idx] != new_val {
                        chunk_terrain[idx] = new_val;
                        modified = true;
                    }
                }
            }
        }

        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: chunk_terrain,
                });
            });
        }
    });
}

/// Polish rivers — re-run shore building on all tiles.
///
/// Port of C++ `polish_river()` (overmap.cpp L2735-2742).
///
/// This is called after roads and specials are placed to ensure
/// river shores are consistent. Roads/specials may have overwritten
/// some shore tiles.
///
/// In our pipeline, this runs in `OvermapGenSet::Finalize`.
pub fn polish_river(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
) {
    // Same logic as build_river_shores — rebuild all shores from scratch
    let mut terrain = [[TerrainHandle::NULL; 180]; 180];
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
                    terrain[gx][gy] = chunk.get(lx, ly);
                }
            }
        }
    }

    let mut is_river_center = [[false; 180]; 180];
    for x in 0..180 {
        for y in 0..180 {
            is_river_center[x][y] = is_river(terrain[x][y], &registry);
        }
    }

    let mut new_terrain = terrain;

    for x in 0..180i32 {
        for y in 0..180i32 {
            if !is_river_center[x as usize][y as usize] {
                continue;
            }

            let mut mask: u8 = 0;
            let mut multiplier: u8 = 1;

            for &(dx, dy) in &FOUR_ADJACENT {
                let nx = x + dx;
                let ny = y + dy;

                if !inbounds_omt((nx, ny)) || is_river(terrain[nx as usize][ny as usize], &registry)
                {
                    mask += multiplier;
                }
                multiplier *= 2;
            }

            if mask == 15 {
                if let Some(trimmed) = trimmed_corner_terrain((x, y), &is_river_center, &registry) {
                    new_terrain[x as usize][y as usize] = trimmed;
                    continue;
                }
            }

            new_terrain[x as usize][y as usize] = river_shore_terrain(mask, &registry);
        }
    }

    // Write back via par_iter
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut chunk_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    let new_val = new_terrain[gx][gy];
                    if chunk_terrain[idx] != new_val {
                        chunk_terrain[idx] = new_val;
                        modified = true;
                    }
                }
            }
        }

        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: chunk_terrain,
                });
            });
        }
    });
}
