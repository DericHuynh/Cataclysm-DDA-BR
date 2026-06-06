//! River placement and shore construction — verbatim port of CDDA master's
//! river generation system.
//!
//! ## C++ references
//!
//! | Function | File | Lines |
//! |---|---|---|
//! | `place_rivers()` | overmap_water.cpp | L558-654 |
//! | `place_river()` | overmap_water.cpp | L47-226 |
//! | `river_meander()` | overmap_water.cpp | L228-256 |
//! | `polish_river()` | overmap.cpp | L2735-2742 |
//! | `build_river_shores()` | overmap_water.cpp | L656-792 |
//! | `setup_adjacent_river()` | overmap_water.cpp | L496-556 |
//!
//! ## Pipeline position
//!
//! `place_rivers` runs in NaturalTerrain (before lakes/oceans/forests/swamps/ravines).
//! `polish_river` runs TWICE: after ravines and after forest trailheads.
//! Both calls rebuild all river shores from scratch via `build_river_shores`.
//!
//! ## Neighbor simplification
//!
//! Since we can't read actual neighbor overmaps in the Rust ECS port, we use a
//! simplified approach: generate rivers with random start/end points on the
//! edges, using Bezier curves and meandering. `ConnectionExits` exit points
//! (from `neighbor_connections.rs`) serve as optional starting hints for edge
//! placement. Shore construction treats out-of-bounds neighbors as river for
//! border continuity.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::connections::{inbounds_omt, line_between, trig_dist};
use cdda_overmap::direction::FOUR_ADJACENT_OFFSETS;
use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::neighbor_connections::ConnectionExits;

// ---------------------------------------------------------------------------
// Constants — matching C++ `RIVER_BORDER` / `RIVER_Z`
// ---------------------------------------------------------------------------

/// Border margin where river nodes are placed on edges (C++ `RIVER_BORDER`).
const RIVER_BORDER: i32 = 10;

/// Z-level for rivers (C++ `RIVER_Z`).
const RIVER_Z: i32 = 0;

/// Edge coordinate for the far side of the overmap.
const OMAPX_EDGE: i32 = OMAP_DIM - 1;
const OMAPY_EDGE: i32 = OMAP_DIM - 1;

/// Maximum number of major rivers per overmap (C++ `max_rivers`).
const MAX_RIVERS: usize = 2;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A placed river node entity — mirrors C++ `overmap_river_node`.
#[derive(Component, Debug, Clone)]
pub struct RiverNode {
    pub start: (i32, i32),
    pub end: (i32, i32),
}

/// Stores control points for river Bezier curves between adjacent overmaps.
#[derive(Resource, Debug, Clone, Default)]
pub struct RiverBorderData {
    /// River node start points from neighbor overmaps (for continuity).
    pub border_river_nodes_omt: Vec<(i32, i32)>,
    /// Control point data for smooth curves across boundaries.
    /// `(start_control, end_control)`
    pub border_control_points: Vec<((i32, i32), (i32, i32))>,
}

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

/// Build a dense `[[u32; 180]; 180]` grid and collect z=0 chunk entities.
fn build_omt_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
) -> ([[u32; 180]; 180], Vec<(Entity, ChunkPosition)>) {
    let mut grid = [[0u32; 180]; 180];
    let mut z0_chunks: Vec<(Entity, ChunkPosition)> = Vec::with_capacity(36);

    for (entity, pos, chunk) in chunks.iter() {
        if pos.z.0 as i32 != RIVER_Z {
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
    grid: &[[u32; 180]; 180],
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
                    new_terrain[idx] = TerrainHandle(grid[omt_y as usize][omt_x as usize]);
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

/// Returns `true` if the given handle represents a water body (not shore).
///
/// Matches C++ `is_water_body_not_shore()` — checks for RIVER, LAKE, or OCEAN
/// flags.
fn is_water_body_not_shore(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

/// Returns `true` if the handle has the RIVER flag.
///
/// Matches C++ `is_river()` / `oter_t::is_river()`.
#[inline]
fn is_river(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).contains(TerrainFlags::RIVER)
}

/// Returns `true` if the handle represents any water body (including shores).
///
/// Matches C++ `is_water_body()`.
fn is_water_body(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Chebyshev distance between two points.
///
/// Matches C++ `rl_dist()` with `trigdist == false` (the default).
/// C++ `square_dist()` = `max(|dx|, |dy|)`.
#[inline]
fn rl_dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = (a.0 - b.0).abs();
    let dy = (a.1 - b.1).abs();
    dx.max(dy)
}

/// Points within Euclidean radius of `center`.
///
/// Iterates the Chebyshev bounding box `[center - radius, center + radius]`
/// and returns all points where `trig_dist(center, pt) < radius + 0.5`.
/// Matches C++ `points_in_radius_circ()`.
fn points_in_radius_circ(center: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    let r = radius as f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let pt = (center.0 + dx, center.1 + dy);
            if trig_dist(center, pt) < r + 0.5 {
                pts.push(pt);
            }
        }
    }
    pts
}

/// Cubic Bezier curve.
///
/// Returns `n_segs + 1` points along a cubic Bezier defined by
/// `p0 → p1 → p2 → p3`. Matches C++ `cubic_bezier()` in `line.h` L213-227.
fn cubic_bezier(
    p0: (i32, i32),
    p1: (i32, i32),
    p2: (i32, i32),
    p3: (i32, i32),
    n_segs: i32,
) -> Vec<(i32, i32)> {
    let single_axis = |a: i32, b: i32, c: i32, d: i32, t: f64| -> i32 {
        // a(1-t)³ + 3bt(1-t)² + 3ct²(1-t) + dt³
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
            single_axis(p0.0, p1.0, p2.0, p3.0, t),
            single_axis(p0.1, p1.1, p2.1, p3.1, t),
        ));
    }
    pts
}

/// River meander — perturbs `current` toward `river_end`.
///
/// Matches C++ `overmap::river_meander()` (overmap_water.cpp L228-256).
///
/// As distance to the river end decreases, meander closer to the end.
/// Random meander is applied for `river_scale > 1`.
fn river_meander(
    rng: &mut XorShiftRng,
    river_end: (i32, i32),
    current: &mut (i32, i32),
    river_scale: i32,
) {
    let random_uniform = |rng: &mut XorShiftRng, i: i32| -> bool {
        rng.range_i32(0, (OMAP_DIM as f64 * 1.2) as i32 - 1) < i
    };
    let random_close = |rng: &mut XorShiftRng, i: i32| -> bool {
        rng.range_i32(0, (OMAP_DIM as f64 * 0.2) as i32 - 1) > i
    };

    let abs_dist_x = (river_end.0 - current.0).abs();
    let abs_dist_y = (river_end.1 - current.1).abs();

    // As distance to river end decreases, meander closer to the river end.
    if current.0 != river_end.0
        && (random_uniform(rng, abs_dist_x)
            || (random_close(rng, abs_dist_x) && random_close(rng, abs_dist_y)))
    {
        if river_end.0 > current.0 {
            current.0 += 1;
        } else {
            current.0 -= 1;
        }
    }
    if current.1 != river_end.1
        && (random_uniform(rng, abs_dist_y)
            || (random_close(rng, abs_dist_y) && random_close(rng, abs_dist_x)))
    {
        if river_end.1 > current.1 {
            current.1 += 1;
        } else {
            current.1 -= 1;
        }
    }

    // Meander randomly, but not for rivers of size 1 (would exceed above meander).
    if river_scale > 1 {
        current.0 += rng.range_i32(-1, 1);
        current.1 += rng.range_i32(-1, 1);
    }
}

// ---------------------------------------------------------------------------
// place_rivers — system entry point
// ---------------------------------------------------------------------------

/// Place river terrain on the overmap.
///
/// Verbatim port of C++ `overmap::place_rivers()` (overmap_water.cpp L558-654)
/// and `overmap::place_river()` (overmap_water.cpp L47-226).
///
/// # Simplified neighbor handling
///
/// Since we can't read actual neighbor overmaps, we generate rivers with
/// random start/end points on edges. If [`ConnectionExits`] is present, its
/// exit points provide candidate edge positions; otherwise purely random
/// positions are used.
pub fn place_rivers(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
    exits: Option<Res<ConnectionExits>>,
) {
    if !settings.overmap_river {
        info!("place_rivers: skipped — overmap_river is false");
        return;
    }

    let settings_river = &settings.river;
    if settings_river.river_scale == 0 {
        info!("place_rivers: skipped — river_scale is 0");
        return;
    }

    // C++ L568: `river_scale = 1 + std::max(1, river_scale)`
    let effective_scale = 1 + settings_river.river_scale.max(1);

    // --- Build grid -----------------------------------------------------------
    let (mut grid, z0_chunks) = build_omt_grid(&chunks);
    let river_center_raw = core_terrains.river_center.0;

    // --- RNG seeded for deterministic results ---------------------------------
    // C++ uses `rng` calls; we seed with `noise_seed + 173` for river-specific
    // determinism.
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 173);

    // --- Determine major river count for frequency check -----------------------
    // C++ L573-579: `x_in_y(1.0, pow(river_frequency, major_river_count))`
    // Since we don't have a cross-overmap buffer, use a simplified check:
    // first overmap always passes (major_river_count = 0 → pow(freq, 0) = 1.0).
    // We track major rivers locally within this overmap.
    let mut major_river_count: i32 = 0;

    // --- Generate river start/end points ---------------------------------------
    // C++ L581-653

    // Start points on North (y=0) or West (x=0) edges.
    // End points on East (x=179) or South (y=179) edges.
    let mut river_start: [Option<(i32, i32)>; MAX_RIVERS] = [None, None];
    let mut river_end: [Option<(i32, i32)>; MAX_RIVERS] = [None, None];

    // Helper: generate a random point on a specific edge or anywhere.
    // dir: 0=north, 1=east, 2=south, 3=west, -1=anywhere in margins.
    let generate_edge_point = |rng: &mut XorShiftRng, dir: i32| -> (i32, i32) {
        match dir {
            0 => (rng.range_i32(RIVER_BORDER, OMAPX_EDGE - RIVER_BORDER), 0),
            1 => (
                OMAPX_EDGE,
                rng.range_i32(RIVER_BORDER, OMAPY_EDGE - RIVER_BORDER),
            ),
            2 => (
                rng.range_i32(RIVER_BORDER, OMAPX_EDGE - RIVER_BORDER),
                OMAPY_EDGE,
            ),
            3 => (0, rng.range_i32(RIVER_BORDER, OMAPY_EDGE - RIVER_BORDER)),
            _ => (
                rng.range_i32(RIVER_BORDER, OMAPX_EDGE - RIVER_BORDER),
                rng.range_i32(RIVER_BORDER, OMAPY_EDGE - RIVER_BORDER),
            ),
        }
    };

    // Determine which edges have "node present" — i.e. have connection exits.
    // In C++ this comes from neighbor overmap rivers; we use ConnectionExits.
    let node_present: [bool; 4] = if let Some(ref exits) = exits {
        [
            !exits.north.is_empty(), // dir 0: north
            !exits.east.is_empty(),  // dir 1: east
            !exits.south.is_empty(), // dir 2: south
            !exits.west.is_empty(),  // dir 3: west
        ]
    } else {
        [false; 4]
    };

    let no_neighboring_rivers = !node_present.iter().any(|&b| b);

    // C++ L579: frequency check when no neighboring rivers
    if no_neighboring_rivers {
        let freq = settings_river.river_frequency;
        let adjusted = freq.powi(major_river_count);
        if !rng.x_in_y(1, adjusted as i32) && adjusted > 0.0 {
            // x_in_y(1, adjusted) — if adjusted < 1, this can fail.
            // In C++: `x_in_y(1.0, pow(freq, count))` where x_in_y takes doubles.
            // Simplified: roll with probability 1 / freq^count.
            // Actually x_in_y(1.0, X) returns true with prob 1/X.
            // If adjusted is e.g. 1.5, x_in_y(1.0, 1.5) uses roll_remainder effectively.
            // We approximate: if adjusted <= 1.0 → always true; else 1-in-adjusted.
            if adjusted <= 1.0 || rng.one_in(adjusted as i32) {
                // pass — river generation continues
            } else {
                info!(
                    om_x = config.om_x,
                    om_y = config.om_y,
                    river_frequency = settings_river.river_frequency,
                    major_river_count,
                    "place_rivers: frequency check failed, no rivers placed"
                );
                return;
            }
        }
    }

    // C++ L617-653: generate river start/end nodes
    // Simplified: generate one river with start on N/W edge and end on E/S edge.

    // No neighbor rivers: generate one river with sensible start/end edges.
    if no_neighboring_rivers {
        // Pick a start edge: prefer North (0) or West (3)
        if node_present[0] && (!node_present[3] || rng.one_in(2)) {
            river_start[0] = Some(generate_edge_point(&mut rng, 0));
        } else if node_present[3] {
            river_start[0] = Some(generate_edge_point(&mut rng, 3));
        } else {
            // Random: 50% north, 50% west
            if rng.one_in(2) {
                river_start[0] = Some(generate_edge_point(&mut rng, 0));
            } else {
                river_start[0] = Some(generate_edge_point(&mut rng, 3));
            }
        }

        // Pick an end edge: prefer South (2) or East (1)
        if node_present[2] && (!node_present[1] || rng.one_in(2)) {
            river_end[0] = Some(generate_edge_point(&mut rng, 2));
        } else if node_present[1] {
            river_end[0] = Some(generate_edge_point(&mut rng, 1));
        } else {
            // Random: 50% east, 50% south
            if rng.one_in(2) {
                river_end[0] = Some(generate_edge_point(&mut rng, 1));
            } else {
                river_end[0] = Some(generate_edge_point(&mut rng, 2));
            }
        }
    } else {
        // Has neighbor exits: use them as hints for edge selection.
        // North/West exits → start points; East/South exits → end points.
        if node_present[0] {
            river_start[0] = Some(generate_edge_point(&mut rng, 0));
        } else if node_present[3] {
            river_start[0] = Some(generate_edge_point(&mut rng, 3));
        } else {
            // Fallback: random start
            if rng.one_in(2) {
                river_start[0] = Some(generate_edge_point(&mut rng, 0));
            } else {
                river_start[0] = Some(generate_edge_point(&mut rng, 3));
            }
        }

        if node_present[2] {
            river_end[0] = Some(generate_edge_point(&mut rng, 2));
        } else if node_present[1] {
            river_end[0] = Some(generate_edge_point(&mut rng, 1));
        } else {
            if rng.one_in(2) {
                river_end[0] = Some(generate_edge_point(&mut rng, 1));
            } else {
                river_end[0] = Some(generate_edge_point(&mut rng, 2));
            }
        }
    }

    // --- Place each river -----------------------------------------------------
    let mut rivers_placed: usize = 0;

    for i in 0..MAX_RIVERS {
        let start = match river_start[i] {
            Some(s) => s,
            None => continue,
        };
        let end = match river_end[i] {
            Some(e) => e,
            None => continue,
        };

        place_single_river(
            start,
            end,
            effective_scale,
            &mut grid,
            river_center_raw,
            &registry,
            &mut rng,
            &settings_river,
        );

        rivers_placed += 1;
        if no_neighboring_rivers {
            _ = major_river_count; // C++ tracks this across overmaps; we're single-overmap
            major_river_count += 1;
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        rivers = rivers_placed,
        effective_scale,
        "place_rivers: terrain computed"
    );

    // --- Write back to chunks -------------------------------------------------
    write_back_grid(&grid, &z0_chunks, &mut commands);
}

// ---------------------------------------------------------------------------
// place_single_river — core Bezier river drawing
// ---------------------------------------------------------------------------

/// Draw a single river from `river_start` to `river_end`.
///
/// Verbatim port of C++ `overmap::place_river()` (overmap_water.cpp L47-226).
///
/// Returns `true` if the river was successfully placed.
fn place_single_river(
    river_start: (i32, i32),
    mut river_end: (i32, i32),
    river_scale: i32,
    grid: &mut [[u32; 180]; 180],
    river_center_raw: u32,
    registry: &TerrainRegistry,
    rng: &mut XorShiftRng,
    settings_river: &crate::region_settings::RegionSettingsRiver,
) {
    // C++ L52: if river_scale <= 0, return
    if river_scale <= 0 {
        return;
    }

    let distance = rl_dist(river_start, river_end);
    let amplitude = distance / 2;

    // C++ L80-99: compute control points at 1/3 and 2/3 of the way.
    let one_third_x = ((river_start.0 - river_end.0).abs() as f64 * (1.0 / 3.0)) as i32;
    let one_third_y = ((river_start.1 - river_end.1).abs() as f64 * (1.0 / 3.0)) as i32;

    let control_p1 = {
        let base_x = river_start.0 + one_third_x;
        let base_y = river_start.1 + one_third_y;
        let perturb_x = rng.range_i32(0, amplitude);
        let perturb_y = rng.range_i32(0, amplitude);
        (
            (base_x + perturb_x).clamp(0, OMAPX_EDGE),
            (base_y + perturb_y).clamp(0, OMAPY_EDGE),
        )
    };

    let control_p2 = {
        let base_x = river_start.0 + one_third_x * 2;
        let base_y = river_start.1 + one_third_y * 2;
        let perturb_x = rng.range_i32(-amplitude, 0);
        let perturb_y = rng.range_i32(-amplitude, 0);
        (
            (base_x + perturb_x).clamp(0, OMAPX_EDGE),
            (base_y + perturb_y).clamp(0, OMAPY_EDGE),
        )
    };

    // C++ L100-104: number of Bezier segments = distance / 2, minimum 4
    let n_segs = distance / 2;
    if n_segs < 4 {
        return;
    }

    // C++ L106-110: generate Bezier curve, remove adjacent duplicates
    let mut segmented_curve = cubic_bezier(river_start, control_p1, control_p2, river_end, n_segs);
    segmented_curve.dedup();
    // C++ L113: prepend start
    segmented_curve.insert(0, river_start);

    let curve_size = segmented_curve.len();
    let last_idx = curve_size - 1;

    // C++ L119-130: check first third of curve — if already water, abort
    let check_limit = (last_idx / 3).min(last_idx);
    let mut river_check_index = 0;
    for i in 0..check_limit {
        let pt = segmented_curve[i];
        if inbounds_omt(pt) {
            let handle = TerrainHandle(grid[pt.1 as usize][pt.0 as usize]);
            if !is_water_body_not_shore(handle, registry) {
                break;
            }
        }
        river_check_index = i + 1;
    }

    // If first third is all water, abort.
    if river_check_index >= check_limit && check_limit > 0 {
        return;
    }

    // C++ L132-142: check remaining points — if water encountered, truncate
    let mut effective_curve_size = curve_size;
    for i in river_check_index..last_idx {
        let pt = segmented_curve[i];
        if inbounds_omt(pt) {
            let handle = TerrainHandle(grid[pt.1 as usize][pt.0 as usize]);
            if is_water_body_not_shore(handle, registry) {
                river_end = pt;
                effective_curve_size = i + 1;
                break;
            }
        }
    }

    // C++ L145-177: draw river along each Bezier segment
    let end_idx = effective_curve_size - 1;
    for i in 0..end_idx {
        let seg_start = segmented_curve[i];
        let seg_end = segmented_curve[i + 1];

        // Bresenham line between consecutive Bezier points.
        let bezier_segment = line_between(seg_start, seg_end);

        for &bezier_point in &bezier_segment {
            let mut meandered = bezier_point;

            // No meander for first/last segment.
            if i != 0 && i != end_idx - 1 {
                river_meander(rng, river_end, &mut meandered, river_scale);
            }

            // Draw river in radius [-river_scale, +river_scale]
            for pt in points_in_radius_circ(meandered, river_scale) {
                if !inbounds_omt(pt) {
                    continue;
                }
                let handle = TerrainHandle(grid[pt.1 as usize][pt.0 as usize]);
                if !is_water_body_not_shore(handle, registry) {
                    grid[pt.1 as usize][pt.0 as usize] = river_center_raw;
                }
            }
        }
    }

    // C++ L180-212: create river branches
    let branch_ahead_points = (effective_curve_size / 5).max(2);
    let mut branch_last_end: usize = 0;

    for i in 0..end_idx {
        let bezier_point = segmented_curve[i];

        if !inbounds_omt(bezier_point) {
            continue;
        }
        // Check if within margin for branch placement.
        let margin = river_scale + 1;
        if bezier_point.0 < margin
            || bezier_point.0 >= OMAP_DIM - margin
            || bezier_point.1 < margin
            || bezier_point.1 >= OMAP_DIM - margin
        {
            continue;
        }

        if !rng.one_in(settings_river.river_branch_chance) {
            continue;
        }

        let branch_end_point =
            if i > branch_last_end && rng.one_in(settings_river.river_branch_remerge_chance) {
                // Re-merge branch: pick a point later along the curve.
                let end_node = rng.range_i32(
                    i as i32 + branch_ahead_points as i32,
                    i as i32 + branch_ahead_points as i32 * 2,
                );
                if end_node < end_idx as i32 {
                    branch_last_end = end_node as usize;
                    Some(segmented_curve[end_node as usize])
                } else {
                    None
                }
            } else {
                // Random branch: pick a point in a 64-radius area.
                let rad = 64;
                Some((
                    rng.range_i32(bezier_point.0 + rad / 2, bezier_point.0 + rad),
                    rng.range_i32(bezier_point.1 + rad / 2, bezier_point.1 + rad),
                ))
            };

        if let Some(branch_end) = branch_end_point {
            if inbounds_omt(branch_end) {
                let branch_scale = river_scale - settings_river.river_branch_scale_decrease;
                if branch_scale > 0 {
                    place_single_river(
                        bezier_point,
                        branch_end,
                        branch_scale,
                        grid,
                        river_center_raw,
                        registry,
                        rng,
                        settings_river,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// build_river_shores — convert river_center to proper shore variants
// ---------------------------------------------------------------------------

/// The shoreline terrain lookup table, indexed by 4-bit adjacency mask.
///
/// Matches the C++ `river_ters` array (overmap_water.cpp L735-759).
///
/// Bit layout (matching C++ `four_adjacent_offsets` order):
/// - bit 0 (mult 1): North neighbor (0, -1)
/// - bit 1 (mult 2): East  neighbor (1,  0)
/// - bit 2 (mult 4): South neighbor (0,  1)
/// - bit 3 (mult 8): West  neighbor (-1, 0)
///
/// Mask → terrain string ID:
///   0:  "forest_water"     (no adjacent rivers)
///   1:  "river_south"      (N only)
///   2:  "river_west"       (E only)
///   3:  "river_sw"         (N+E)
///   4:  "river_north"      (S only)
///   5:  "forest_water"     (N+S — no map)
///   6:  "river_nw"         (E+S)
///   7:  "river_west"       (N+E+S)
///   8:  "river_east"       (W only)
///   9:  "river_se"         (N+W)
///  10:  "forest_water"     (E+W — no map)
///  11:  "river_south"      (N+E+W)
///  12:  "river_ne"         (S+W)
///  13:  "river_east"       (N+S+W)
///  14:  "river_north"      (E+S+W)
///  15:  "river_center"     (N+E+S+W) — check trimmed corners
const RIVER_SHORE_TABLE: [&str; 16] = [
    /*  0 */ "forest_water",
    /*  1 */ "river_south",
    /*  2 */ "river_west",
    /*  3 */ "river_sw",
    /*  4 */ "river_north",
    /*  5 */ "forest_water",
    /*  6 */ "river_nw",
    /*  7 */ "river_west",
    /*  8 */ "river_east",
    /*  9 */ "river_se",
    /* 10 */ "forest_water",
    /* 11 */ "river_south",
    /* 12 */ "river_ne",
    /* 13 */ "river_east",
    /* 14 */ "river_north",
    /* 15 */ "river_center",
];

/// Trimmed corner variants for mask 15, indexed by ordinal direction.
///
/// C++ `four_ordinal_directions` order: NE, SE, SW, NW.
/// If the corner is NOT water, use the trimmed-corner terrain.
const TRIMMED_CORNER_TABLE: [&str; 4] = [
    "river_c_not_ne", // NE corner is not water → trim NE
    "river_c_not_se", // SE corner is not water → trim SE
    "river_c_not_sw", // SW corner is not water → trim SW
    "river_c_not_nw", // NW corner is not water → trim NW
];

/// Ordinal (diagonal) offsets matching C++ `four_ordinal_directions`:
/// NE, SE, SW, NW.
const FOUR_ORDINAL_OFFSETS: [(i32, i32); 4] = [(1, -1), (1, 1), (-1, 1), (-1, -1)];

/// Determine the shore terrain for a river tile at `pt` given the full terrain grid.
///
/// Matches C++ `overmap::build_river_shores()` (overmap_water.cpp L656-792).
///
/// Computes a 4-bit mask from the 4 cardinal neighbors. Out-of-bounds neighbors
/// count as river (for border continuity). Looks up the shore terrain from
/// the mask table. For mask 15 (all 4 connections), checks ordinal corners for
/// trimmed-corner variants.
fn compute_river_shore(
    pt: (i32, i32),
    grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
) -> u32 {
    let mut mask: usize = 0;
    let mut multiplier: usize = 1;

    // C++ L719-731: check 4 cardinal neighbors
    for &(dx, dy) in &FOUR_ADJACENT_OFFSETS {
        let np = (pt.0 + dx, pt.1 + dy);

        if !inbounds_omt(np) {
            // Out-of-bounds — treat as river (border continuity).
            mask += multiplier;
        } else {
            let handle = TerrainHandle(grid[np.1 as usize][np.0 as usize]);
            if is_water_body(handle, registry) {
                mask += multiplier;
            }
        }
        multiplier *= 2;
    }

    if mask == 15 {
        // C++ L773-787: check ordinal corners for trimmed-corner terrain
        for i in 0..4 {
            let (cdx, cdy) = FOUR_ORDINAL_OFFSETS[i];
            let corner = (pt.0 + cdx, pt.1 + cdy);
            if inbounds_omt(corner) {
                let handle = TerrainHandle(grid[corner.1 as usize][corner.0 as usize]);
                if !is_water_body(handle, registry) {
                    // Corner is not water → use trimmed-corner variant
                    if let Some(h) = registry.handle_by_id(TRIMMED_CORNER_TABLE[i]) {
                        return h.0;
                    }
                }
            }
        }
    }

    // Look up shore terrain from the mask table.
    let shore_id = RIVER_SHORE_TABLE[mask];
    registry.handle_by_id(shore_id).map(|h| h.0).unwrap_or(0)
}

/// Build river shores for all river tiles on the overmap.
///
/// Verbatim port of C++ `overmap::build_river_shores()` (overmap_water.cpp L656-792).
///
/// For every tile that has the RIVER flag, computes the appropriate shore
/// variant based on 4-way cardinal adjacency and replaces the tile's terrain.
pub fn build_river_shores(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    mut commands: Commands,
    registry: Res<TerrainRegistry>,
) {
    let (mut grid, z0_chunks) = build_omt_grid(&chunks);

    let mut modified: usize = 0;

    for y in 0..OMAP_DIM as usize {
        for x in 0..OMAP_DIM as usize {
            let handle = TerrainHandle(grid[y][x]);
            if !is_river(handle, &registry) {
                continue;
            }

            grid[y][x] = compute_river_shore((x as i32, y as i32), &grid, &registry);
            modified += 1;
        }
    }

    if modified > 0 {
        info!(
            river_tiles = modified,
            "build_river_shores: computed shores for {} tiles", modified
        );
        write_back_grid(&grid, &z0_chunks, &mut commands);
    }
}

/// Polish (recompute) river shores for ALL tiles.
///
/// Verbatim port of C++ `overmap::polish_river()` (overmap.cpp L2735-2742).
///
/// This runs `build_river_shores` across the entire overmap, rebuilding all
/// river shore variants from scratch. Called twice in the pipeline:
/// once after ravines and once after forest trailheads.
///
/// Both calls use the same algorithm — they rebuild shores based on current
/// terrain, handling any new water tiles that may have been placed by
/// intervening systems.
pub fn polish_river(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    commands: Commands,
    registry: Res<TerrainRegistry>,
) {
    // `polish_river` is `build_river_shores` applied to ALL tiles.
    // In C++: `for x, y { build_river_shores(neighbor_overmaps, {x, y, 0}); }`
    // This is equivalent to our `build_river_shores` which already iterates
    // all tiles.
    build_river_shores(chunks, commands, registry);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Geometry helpers -----------------------------------------------------

    #[test]
    fn rl_dist_chebyshev() {
        assert_eq!(rl_dist((0, 0), (3, 4)), 4); // max(3, 4) = 4
        assert_eq!(rl_dist((10, 10), (10, 10)), 0);
        assert_eq!(rl_dist((0, 0), (-5, 2)), 5); // max(5, 2) = 5
    }

    #[test]
    fn points_in_radius_circ_basic() {
        let pts = points_in_radius_circ((0, 0), 1);
        // Should include center and 4 cardinal neighbors
        assert!(pts.contains(&(0, 0)));
        assert!(pts.contains(&(1, 0)));
        assert!(pts.contains(&(-1, 0)));
        assert!(pts.contains(&(0, 1)));
        assert!(pts.contains(&(0, -1)));
        // Should NOT include diagonals (trig_dist = sqrt(2) ≈ 1.414 > 1.5? No, < 1.5).
        // Actually sqrt(2) ≈ 1.414 < 1.5, so diagonals ARE included.
        assert!(pts.contains(&(1, 1)));
        // (2, 0): trig_dist = 2.0 > 1.5, should NOT be included.
        assert!(!pts.contains(&(2, 0)));
    }

    #[test]
    fn cubic_bezier_straight_line() {
        // Straight line from (0,0) to (9,0) with control points on the line.
        let pts = cubic_bezier((0, 0), (3, 0), (6, 0), (9, 0), 9);
        assert_eq!(pts.len(), 10);
        for (i, &(x, y)) in pts.iter().enumerate() {
            assert_eq!(x, i as i32);
            assert_eq!(y, 0);
        }
    }

    #[test]
    fn cubic_bezier_endpoints() {
        let pts = cubic_bezier((10, 20), (30, 40), (50, 60), (70, 80), 5);
        assert_eq!(pts[0], (10, 20));
        assert_eq!(pts[5], (70, 80));
    }

    #[test]
    fn river_meander_moves_toward_end() {
        let mut rng = XorShiftRng::new(12345);
        let river_end = (100, 100);
        let mut current = (10, 10);

        // With high river_scale and many iterations, the point should tend
        // toward the river end.
        for _ in 0..100 {
            river_meander(&mut rng, river_end, &mut current, 3);
        }
        // After 100 meanders, current should be closer to river_end.
        let initial_dist = rl_dist((10, 10), river_end);
        let final_dist = rl_dist(current, river_end);
        // Should have moved at least somewhat toward the end (probabilistic).
        assert!(final_dist < initial_dist);
    }

    // --- Shore mask computation -----------------------------------------------

    #[test]
    fn shore_mask_all_river_neighbors() {
        // Create a 3x3 grid of river tiles around a center river tile.
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        // Register all shore variants so handle_by_id works.
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
            "river_c_not_ne",
            "river_c_not_se",
            "river_c_not_sw",
            "river_c_not_nw",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        // Build a small grid with a 3x3 patch of river at (1,1)-(3,3).
        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;
        for y in 1..=3 {
            for x in 1..=3 {
                grid[y][x] = river_raw;
            }
        }

        // Center tile (2,2) has all 4 cardinal neighbors as river → mask 15.
        let result = compute_river_shore((2, 2), &grid, &registry);
        let result_handle = TerrainHandle(result);
        let result_id = registry.string_id_for(result_handle);
        // Should be river_center if all ordinal corners are also water.
        assert!(
            result_id == Some("river_center"),
            "expected river_center, got {:?}",
            result_id
        );
    }

    #[test]
    fn shore_mask_no_neighbors() {
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;
        // Single isolated river tile at (5,5).
        grid[5][5] = river_raw;

        let result = compute_river_shore((5, 5), &grid, &registry);
        let result_handle = TerrainHandle(result);
        let result_id = registry.string_id_for(result_handle);
        assert_eq!(result_id, Some("forest_water"), "mask 0 → forest_water");
    }

    #[test]
    fn shore_mask_north_only() {
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;
        // River at (5,5) and north neighbor at (5,4).
        grid[5][5] = river_raw;
        grid[4][5] = river_raw; // north

        let result = compute_river_shore((5, 5), &grid, &registry);
        let result_id = registry.string_id_for(TerrainHandle(result));
        assert_eq!(
            result_id,
            Some("river_south"),
            "mask 1 (N only) → river_south"
        );
    }

    #[test]
    fn shore_mask_east_only() {
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;
        // River at (5,5) and east neighbor at (6,5).
        grid[5][5] = river_raw;
        grid[5][6] = river_raw; // east

        let result = compute_river_shore((5, 5), &grid, &registry);
        let result_id = registry.string_id_for(TerrainHandle(result));
        assert_eq!(
            result_id,
            Some("river_west"),
            "mask 2 (E only) → river_west"
        );
    }

    #[test]
    fn shore_mask_south_only() {
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;
        // River at (5,5) and south neighbor at (6,5).
        grid[5][5] = river_raw;
        grid[6][5] = river_raw; // south

        let result = compute_river_shore((5, 5), &grid, &registry);
        let result_id = registry.string_id_for(TerrainHandle(result));
        assert_eq!(
            result_id,
            Some("river_north"),
            "mask 4 (S only) → river_north"
        );
    }

    #[test]
    fn shore_mask_west_only() {
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;
        // River at (5,5) and west neighbor at (5,4).
        grid[5][5] = river_raw;
        grid[5][4] = river_raw; // west

        let result = compute_river_shore((5, 5), &grid, &registry);
        let result_id = registry.string_id_for(TerrainHandle(result));
        assert_eq!(
            result_id,
            Some("river_east"),
            "mask 8 (W only) → river_east"
        );
    }

    #[test]
    fn shore_mask_border_continuity() {
        // Tiles at the edge of the overmap treat out-of-bounds as river.
        let mut registry = TerrainRegistry::empty();
        let river_idx = registry.register_no_entity(
            "river_center",
            TerrainFlags::from_bits(TerrainFlags::RIVER),
            2,
            String::new(),
                0,
            );
        for id in &[
            "forest_water",
            "river_south",
            "river_west",
            "river_sw",
            "river_north",
            "river_nw",
            "river_east",
            "river_se",
            "river_ne",
            "river_center",
            "river_c_not_ne",
            "river_c_not_se",
            "river_c_not_sw",
            "river_c_not_nw",
        ] {
            registry.register_no_entity(
                id,
                TerrainFlags::from_bits(TerrainFlags::RIVER),
                2,
                String::new(),
                0,
            );
        }

        let mut grid = [[0u32; 180]; 180];
        let river_raw = TerrainHandle::new(river_idx, 0).0;

        // River at (0, 5) — west neighbor out of bounds → counts as river.
        grid[5][0] = river_raw;

        let result = compute_river_shore((0, 5), &grid, &registry);
        let result_id = registry.string_id_for(TerrainHandle(result));
        // West out-of-bounds = river, so at least mask has bit 3 (8).
        // No other neighbors are river, so mask = 8 → river_east.
        assert_eq!(
            result_id,
            Some("river_east"),
            "edge tile with OOB west → river_east"
        );
    }
}
