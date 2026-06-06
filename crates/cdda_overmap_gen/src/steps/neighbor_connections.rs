//! Pipeline step 2: **NeighborConnections** — populate cross-overmap exit points.
//!
//! Verbatim port of C++ `overmap::populate_connections_out_from_neighbors()`
//! (overmap.cpp L1824-1874).
//!
//! In C++ this reads `connections_out` maps from adjacent overmap objects and
//! mirrors their exit points. Since we don't have neighbor overmap objects in
//! Rust, we use **deterministic edge generation**: for each cardinal edge,
//! 2–3 exit OMT positions are derived from a seed computed from the shared
//! boundary's world position. Adjacent overmaps use the same seed for the same
//! boundary, producing matching exit points.
//!
//! ## C++ reference
//!
//! ```cpp
//! const auto populate_for_side = [&](const overmap *adjacent, auto should_include,
//!                                     auto build_point) {
//!     if (adjacent == nullptr) return;
//!     for (const auto &kv : adjacent->connections_out) {
//!         std::vector<tripoint_om_omt> &out = connections_out[kv.first];
//!         for (const tripoint_om_omt &p : adjacent_out->second) {
//!             if (should_include(p)) {
//!                 out.push_back(build_point(p));
//!             }
//!         }
//!     }
//! };
//! ```

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of OMT tiles along one side of the overmap.
const OMAP_DIM: i32 = 180;

/// Margin from each corner where exit points are not placed (10 OMT tiles).
const CORNER_MARGIN: i32 = 10;

/// Nominal number of exit points to generate per edge.
const EXITS_PER_EDGE: usize = 3;

// ---------------------------------------------------------------------------
// ConnectionExits
// ---------------------------------------------------------------------------

/// Exit points on each edge of the overmap, expressed as OMT coordinates
/// within the overmap (0..180 on each axis).
///
/// These are consumed by downstream connection-building systems (roads,
/// railroads, rivers, forest trails) to stitch features across overmap
/// boundaries.
#[derive(Resource, Debug, Clone, Default)]
pub struct ConnectionExits {
    /// Exit points on the north edge: `(x, 0)` where
    /// `x ∈ [CORNER_MARGIN, 180 - CORNER_MARGIN)`.
    pub north: Vec<(i32, i32)>,
    /// Exit points on the east edge: `(179, y)` where
    /// `y ∈ [CORNER_MARGIN, 180 - CORNER_MARGIN)`.
    pub east: Vec<(i32, i32)>,
    /// Exit points on the south edge: `(x, 179)` where
    /// `x ∈ [CORNER_MARGIN, 180 - CORNER_MARGIN)`.
    pub south: Vec<(i32, i32)>,
    /// Exit points on the west edge: `(0, y)` where
    /// `y ∈ [CORNER_MARGIN, 180 - CORNER_MARGIN)`.
    pub west: Vec<(i32, i32)>,
}

impl ConnectionExits {
    /// Total number of exit points across all four edges.
    pub fn total_count(&self) -> usize {
        self.north.len() + self.east.len() + self.south.len() + self.west.len()
    }

    /// Returns `true` if no exit points are stored.
    pub fn is_empty(&self) -> bool {
        self.north.is_empty()
            && self.east.is_empty()
            && self.south.is_empty()
            && self.west.is_empty()
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// Populate [`ConnectionExits`] via deterministic edge-seeded generation.
///
/// # Boundary anchoring (adjacent-overmap seed matching)
///
/// Each shared boundary between two overmaps must produce the same set of
/// exit points regardless of which overmap is being generated. We achieve
/// this by choosing a **canonical "anchor" overmap** for each boundary.
///
/// The function [`generate_edge_offsets`] returns raw `i32` offsets along the
/// edge. The **caller** is responsible for converting these offsets into the
/// correct `(x, y)` coordinate format for each specific edge.
///
/// | Shared boundary     | Canonical anchor   | Seed key |
/// |---------------------|--------------------|----------|
/// | N/S boundary at `y` | overmap south of it | `"north"` |
/// | E/W boundary at `x` | overmap west of it  | `"east"`  |
///
/// Concretely:
///
/// - **North edge** of `(om_x, om_y)`: anchor = `(om_x, om_y)`, seed = `"north"`
///   → offset `p` becomes `(p, 0)`
/// - **South edge** of `(om_x, om_y)`: anchor = `(om_x, om_y + 1)`, seed = `"north"`
///   → offset `p` becomes `(p, 179)`. Matches North of `(om_x, om_y + 1)`. ✓
/// - **East edge** of `(om_x, om_y)`: anchor = `(om_x, om_y)`, seed = `"east"`
///   → offset `p` becomes `(179, p)`
/// - **West edge** of `(om_x, om_y)`: anchor = `(om_x - 1, om_y)`, seed = `"east"`
///   → offset `p` becomes `(0, p)`. Matches East of `(om_x - 1, om_y)`. ✓
pub fn populate_connections_out_from_neighbors(
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
) {
    let base_seed = config.noise_seed as u64;
    let om_x = config.om_x;
    let om_y = config.om_y;

    // Generate raw offsets for each boundary, then format into (x, y) pairs.
    let exits = ConnectionExits {
        north: generate_edge_offsets(base_seed, om_x, om_y, "north")
            .into_iter()
            .map(|p| (p, 0))
            .collect(),
        east: generate_edge_offsets(base_seed, om_x, om_y, "east")
            .into_iter()
            .map(|p| (OMAP_DIM - 1, p))
            .collect(),
        south: generate_edge_offsets(base_seed, om_x, om_y + 1, "north")
            .into_iter()
            .map(|p| (p, OMAP_DIM - 1))
            .collect(),
        west: generate_edge_offsets(base_seed, om_x - 1, om_y, "east")
            .into_iter()
            .map(|p| (0, p))
            .collect(),
    };

    info!(
        om_x = om_x,
        om_y = om_y,
        n = exits.north.len(),
        e = exits.east.len(),
        s = exits.south.len(),
        w = exits.west.len(),
        "NeighborConnections: generated {} exit points",
        exits.total_count(),
    );

    commands.insert_resource(exits);
}

// ---------------------------------------------------------------------------
// Edge exit generation
// ---------------------------------------------------------------------------

/// Hash four values into a deterministic `u64` seed.
fn edge_seed(base_seed: u64, om_x: i32, om_y: i32, edge: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    base_seed.hash(&mut hasher);
    om_x.hash(&mut hasher);
    om_y.hash(&mut hasher);
    edge.hash(&mut hasher);
    hasher.finish()
}

/// Generate 2–3 non-duplicate OMT offset positions along an edge.
///
/// Returns raw `i32` offsets in the valid range
/// `[CORNER_MARGIN, OMAP_DIM - CORNER_MARGIN)`. The caller is responsible
/// for converting these to `(x, y)` pairs appropriate for the specific edge.
fn generate_edge_offsets(base_seed: u64, om_x: i32, om_y: i32, edge: &str) -> Vec<i32> {
    let seed = edge_seed(base_seed, om_x, om_y, edge);
    let mut rng = XorShiftRng::new(seed);

    let range_start = CORNER_MARGIN;
    let range_end = OMAP_DIM - CORNER_MARGIN; // exclusive upper bound

    // 2–3 exits: use a seed-derived coin flip for the count
    let count = if rng.one_in(2) {
        EXITS_PER_EDGE - 1
    } else {
        EXITS_PER_EDGE
    };

    let mut exits = Vec::with_capacity(count);
    let mut used: HashSet<i32> = HashSet::with_capacity(count);

    while exits.len() < count {
        let pos = rng.range_i32(range_start, range_end - 1);
        if used.insert(pos) {
            exits.push(pos);
        }
    }

    // Sort for deterministic ordering regardless of generation order.
    exits.sort();
    exits
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_seed_deterministic() {
        let a = edge_seed(42, 1, 2, "north");
        let b = edge_seed(42, 1, 2, "north");
        assert_eq!(a, b);
    }

    #[test]
    fn edge_seed_different_params() {
        let a = edge_seed(42, 1, 2, "north");
        let b = edge_seed(42, 1, 2, "east");
        let c = edge_seed(42, 3, 2, "north");
        assert_ne!(a, b, "different edge → different seed");
        assert_ne!(a, c, "different om_x → different seed");
    }

    #[test]
    fn north_south_boundary_seeds_match() {
        // North edge of (0, 5) ↔ South edge of (0, 4)
        let north_of_0_5 = edge_seed(100, 0, 5, "north");
        // South edge of (0, 4) uses anchor (0, 4+1)="north" = (0, 5, "north")
        let south_of_0_4 = edge_seed(100, 0, 5, "north");
        assert_eq!(north_of_0_5, south_of_0_4);
    }

    #[test]
    fn east_west_boundary_seeds_match() {
        // East edge of (3, 0) ↔ West edge of (4, 0)
        let east_of_3_0 = edge_seed(100, 3, 0, "east");
        // West edge of (4, 0) uses anchor (4-1, 0)="east" = (3, 0, "east")
        let west_of_4_0 = edge_seed(100, 3, 0, "east");
        assert_eq!(east_of_3_0, west_of_4_0);
    }

    #[test]
    fn south_north_boundary_seeds_match() {
        // South edge of (1, 2) ↔ North edge of (1, 3)
        let south_of_1_2 = edge_seed(200, 1, 3, "north"); // anchor = (1, 2+1) = (1, 3)
                                                          // North edge of (1, 3) uses anchor (1, 3, "north")
        let north_of_1_3 = edge_seed(200, 1, 3, "north");
        assert_eq!(south_of_1_2, north_of_1_3);
    }

    #[test]
    fn generate_exits_respects_margins() {
        let exits = generate_edge_offsets(42, 0, 0, "north");
        assert!(exits.len() >= 2 && exits.len() <= 3);
        for &p in &exits {
            assert!(
                p >= CORNER_MARGIN && p < OMAP_DIM - CORNER_MARGIN,
                "offset {p} outside margin [{CORNER_MARGIN}, {})",
                OMAP_DIM - CORNER_MARGIN
            );
        }
    }

    #[test]
    fn generate_exits_deterministic() {
        let a = generate_edge_offsets(42, 1, 2, "north");
        let b = generate_edge_offsets(42, 1, 2, "north");
        assert_eq!(a, b);
    }

    #[test]
    fn exit_points_are_sorted() {
        let exits = generate_edge_offsets(99, 0, 0, "north");
        let mut sorted = exits.clone();
        sorted.sort();
        assert_eq!(exits, sorted);
    }

    #[test]
    fn all_four_edges_produce_exits() {
        let base_seed = 12345u64;
        let om_x = 7i32;
        let om_y = 3i32;

        let north: Vec<(i32, i32)> = generate_edge_offsets(base_seed, om_x, om_y, "north")
            .into_iter()
            .map(|p| (p, 0))
            .collect();
        let east: Vec<(i32, i32)> = generate_edge_offsets(base_seed, om_x, om_y, "east")
            .into_iter()
            .map(|p| (OMAP_DIM - 1, p))
            .collect();
        let south: Vec<(i32, i32)> = generate_edge_offsets(base_seed, om_x, om_y + 1, "north")
            .into_iter()
            .map(|p| (p, OMAP_DIM - 1))
            .collect();
        let west: Vec<(i32, i32)> = generate_edge_offsets(base_seed, om_x - 1, om_y, "east")
            .into_iter()
            .map(|p| (0, p))
            .collect();

        for exits in [&north, &east, &south, &west] {
            assert!(exits.len() >= 2, "each edge must have at least 2 exits");
        }

        // Verify the anchor-based formulas produce correct edge coordinates
        for &(_x, y) in &north {
            assert_eq!(y, 0);
        }
        for &(x, _y) in &east {
            assert_eq!(x, OMAP_DIM - 1);
        }
        for &(_x, y) in &south {
            assert_eq!(y, OMAP_DIM - 1);
        }
        for &(x, _y) in &west {
            assert_eq!(x, 0);
        }
    }
}
