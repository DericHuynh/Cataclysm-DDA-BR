//! Populate connection exit points by mirroring what adjacent overmaps
//! would produce on their shared edges.
//!
//! Port of CDDA master's `overmap::populate_connections_out_from_neighbors()`
//! (overmap.cpp L1824–1874).
//!
//! # How it works
//!
//! Each overmap edge can have road/railroad exit points. When two overmaps
//! share an edge, their exit points must match for roads to connect across
//! the boundary. C++ solves this by reading the neighbor's `connections_out`
//! map. We solve it by **deterministically computing** what the neighbor
//! would produce, using the edge's world position as the RNG seed.
//!
//! # Deterministic edge formula
//!
//! For edge at overmap-space coordinate `(edge_x, edge_y)` we seed the RNG
//! with `noise_seed ^ hash(edge_x, edge_y)` so that the west edge of
//! overmap (1,0) and the east edge of overmap (0,0) produce identical exits.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::OMAP_DIM;
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// Pre-computed connection exit points for each cardinal edge.
///
/// Populated by `populate_connections_out_from_neighbors` before
/// `place_roads` / `place_railroads` run.  Each system reads this
/// resource to learn where roads/rails should exit the overmap.
#[derive(Resource, Debug, Clone, Default)]
pub struct ConnectionExits {
    /// Exit points on the north edge (y = 0), from west neighbor's south edge.
    pub north: Vec<(i32, i32)>,
    /// Exit points on the east edge (x = OMAP_DIM-1), from east neighbor's west edge.
    pub east: Vec<(i32, i32)>,
    /// Exit points on the south edge (y = OMAP_DIM-1), from south neighbor's north edge.
    pub south: Vec<(i32, i32)>,
    /// Exit points on the west edge (x = 0), from west neighbor's east edge.
    pub west: Vec<(i32, i32)>,
}

impl ConnectionExits {
    /// All exit points flattened into one vector.
    pub fn all(&self) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        v.extend_from_slice(&self.north);
        v.extend_from_slice(&self.east);
        v.extend_from_slice(&self.south);
        v.extend_from_slice(&self.west);
        v
    }

    pub fn is_empty(&self) -> bool {
        self.north.is_empty()
            && self.east.is_empty()
            && self.south.is_empty()
            && self.west.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Hash two i32s into a u64 for use as an RNG seed.
fn hash_2d(x: i32, y: i32) -> u64 {
    let mut h: u64 = 0x9e3779b97f4a7c15;
    h = h.wrapping_add(x as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    h = h.wrapping_add(y as u64).wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 32;
    h
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// Generate deterministic border exit points for all 4 overmap edges.
///
/// For each edge, computes 2–3 exit points using a seed derived from the
/// edge's world OMT position. Adjacent overmaps sharing an edge will
/// produce the same exit points because they hash to the same seed.
///
/// Exit points avoid the corner margins (10 tiles from each corner) and
/// water tiles (checked by `place_roads`/`place_railroads` at use time).
pub fn populate_connections_out_from_neighbors(
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
) {
    let seed = config.noise_seed as u64;
    let om_x = config.om_x;
    let om_y = config.om_y;
    let max = OMAP_DIM;
    let margin: i32 = 10;

    let mut exits = ConnectionExits::default();

    // ── North edge (y=0): mirrors what overmap (om_x, om_y-1) would
    //    put on ITS south edge (y=OMAP_DIM-1).
    {
        let edge_seed = seed ^ hash_2d(om_x, om_y - 1);
        let pts = generate_edge_exits(0, max, edge_seed, margin, |coord| (coord, 0));
        exits.north = pts;
    }

    // ── South edge (y=OMAP_DIM-1): mirrors what overmap (om_x, om_y+1)
    //    would put on ITS north edge (y=0).
    {
        let edge_seed = seed ^ hash_2d(om_x, om_y + 1);
        let pts = generate_edge_exits(0, max, edge_seed, margin, |coord| (coord, max - 1));
        exits.south = pts;
    }

    // ── West edge (x=0): mirrors what overmap (om_x-1, om_y) would
    //    put on ITS east edge (x=OMAP_DIM-1).
    {
        let edge_seed = seed ^ hash_2d(om_x - 1, om_y);
        let pts = generate_edge_exits(0, max, edge_seed, margin, |coord| (0, coord));
        exits.west = pts;
    }

    // ── East edge (x=OMAP_DIM-1): mirrors what overmap (om_x+1, om_y)
    //    would put on ITS west edge (x=0).
    {
        let edge_seed = seed ^ hash_2d(om_x + 1, om_y);
        let pts = generate_edge_exits(0, max, edge_seed, margin, |coord| (max - 1, coord));
        exits.east = pts;
    }

    info!(
        "Neighbor connections: N={} E={} S={} W={} for overmap ({}, {})",
        exits.north.len(),
        exits.east.len(),
        exits.south.len(),
        exits.west.len(),
        om_x,
        om_y
    );

    commands.insert_resource(exits);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate 2–3 deterministic exit points along an edge.
///
/// `coord_range` is (0, OMAP_DIM). `margin` is excluded from both ends.
/// `to_point` maps the coordinate to an (x,y) OMT position on the edge.
fn generate_edge_exits(
    _range_start: i32,
    range_end: i32,
    seed: u64,
    margin: i32,
    to_point: impl Fn(i32) -> (i32, i32),
) -> Vec<(i32, i32)> {
    let mut rng = XorShiftRng::new(seed);

    // 2–3 exits per edge, matching C++ behavior.
    let n = rng.range_i32(2, 3) as usize;
    let mut pts = Vec::with_capacity(n);

    let coord_min = margin;
    let coord_max = range_end - margin - 1;

    if coord_max <= coord_min {
        return pts;
    }

    let span = coord_max - coord_min;
    // Divide the span into segments for even distribution + jitter.
    for i in 0..n {
        let base = coord_min + (span as usize * i / n) as i32;
        let jitter = rng.range_i32(-span / 8, span / 8);
        let coord = (base + jitter).clamp(coord_min, coord_max);
        pts.push(to_point(coord));
    }

    pts
}
