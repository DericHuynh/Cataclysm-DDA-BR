//! Overmap connection system — pathfinding for roads, trails, and railroads.
//!
//! Port of CDDA master's `connect_closest_points` (overmap.cpp L2662-2733)
//! and `build_connection` (overmap.cpp L2563-2648).
//!
//! Uses a minimum-spanning-tree approach with optional loop edges (1-in-10 chance)
//! to connect a set of points into a network. Each connected pair is passed to
//! a caller-supplied build function that places terrain along the straight-line path.

use std::collections::VecDeque;

use crate::rng::XorShiftRng;

// ---------------------------------------------------------------------------
// ConnectionType
// ---------------------------------------------------------------------------

/// Type of overmap connection being built.
///
/// Determines which terrain subtypes are placed along the path and how
/// intersections with existing terrain are resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// Major road connecting two cities across the wilderness.
    InterCityRoad,
    /// Road segment within a single city.
    IntraCityRoad,
    /// Railroad connecting stations / cities.
    Railroad,
    /// Dirt trail through forested areas.
    ForestTrail,
    /// Underground sewer tunnel.
    Sewer,
    /// Underground subway/metro tunnel.
    Subway,
}

// ---------------------------------------------------------------------------
// Path utilities
// ---------------------------------------------------------------------------

/// Compute all integer points on the line from `(x0, y0)` to `(x1, y1)`
/// using Bresenham's algorithm.
///
/// The resulting `Vec` contains both endpoints and every pixel the line
/// passes through in order from `from` to `to`.
pub fn line_between(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (x0, y0) = from;
    let (x1, y1) = to;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut points = Vec::new();
    let mut x = x0;
    let mut y = y0;

    loop {
        points.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }

    points
}

/// Euclidean distance between two OMT points.
///
/// Matches CDDA's `trig_dist` — returns `sqrt(dx^2 + dy^2)` as f32.
#[inline]
pub fn trig_dist(a: (i32, i32), b: (i32, i32)) -> f32 {
    let dx = (a.0 - b.0) as f32;
    let dy = (a.1 - b.1) as f32;
    (dx * dx + dy * dy).sqrt()
}

/// Squared Euclidean distance — cheaper when ordering is all you need.
#[inline]
pub fn square_dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

/// Return all integer points within `max_radius` of `center`, sorted by
/// distance (closest first).
///
/// Uses a spiral-outward (Chebyshev ring) search so that callers can
/// early-exit when they find the first match.
pub fn closest_points_first(center: (i32, i32), max_radius: i32) -> Vec<(i32, i32)> {
    let (cx, cy) = center;
    let mut points = Vec::with_capacity(((2 * max_radius + 1) * (2 * max_radius + 1)) as usize);

    for r in 0..=max_radius {
        // Top and bottom edges of the square at radius r
        for dx in -r..=r {
            points.push((cx + dx, cy - r));
            if r > 0 {
                points.push((cx + dx, cy + r));
            }
        }
        // Left and right edges (excluding corners already pushed)
        for dy in (-r + 1)..=(r - 1) {
            points.push((cx - r, cy + dy));
            if r > 0 {
                points.push((cx + r, cy + dy));
            }
        }
    }

    points
}

/// 4-connected flood fill from `start` within the given bounds.
///
/// Returns all reachable points for which `predicate` returns `true`.
/// Bounds are `(x_min, y_min, x_max_exclusive, y_max_exclusive)`.
pub fn point_flood_fill_4(
    start: (i32, i32),
    bounds: (i32, i32, i32, i32),
    predicate: impl Fn((i32, i32)) -> bool,
) -> Vec<(i32, i32)> {
    let (x_min, y_min, x_max, y_max) = bounds;
    let mut result = Vec::new();
    let mut queue = VecDeque::new();

    if !inbounds_rect(start, bounds) || !predicate(start) {
        return result;
    }

    // Use a simple visited set keyed by flat index.
    let w = (x_max - x_min) as usize;
    let mut visited = vec![false; w * (y_max - y_min) as usize];

    let idx = |x: i32, y: i32| -> usize { (y - y_min) as usize * w + (x - x_min) as usize };

    queue.push_back(start);
    visited[idx(start.0, start.1)] = true;

    const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    while let Some(p) = queue.pop_front() {
        result.push(p);
        for (dx, dy) in DIRS {
            let np = (p.0 + dx, p.1 + dy);
            if !inbounds_rect(np, bounds) {
                continue;
            }
            let i = idx(np.0, np.1);
            if visited[i] {
                continue;
            }
            visited[i] = true;
            if predicate(np) {
                queue.push_back(np);
            }
        }
    }

    result
}

/// Check whether a point lies within `(x_min, y_min, x_max_exclusive, y_max_exclusive)`.
#[inline]
fn inbounds_rect(p: (i32, i32), bounds: (i32, i32, i32, i32)) -> bool {
    p.0 >= bounds.0 && p.0 < bounds.2 && p.1 >= bounds.1 && p.1 < bounds.3
}

/// Check if a point is within the standard overmap bounds (0..180, 0..180).
#[inline]
pub fn inbounds_omt(p: (i32, i32)) -> bool {
    p.0 >= 0 && p.0 < 180 && p.1 >= 0 && p.1 < 180
}

/// Check if a point is within the overmap bounds with a margin.
///
/// `margin` tiles on each side are excluded.
#[inline]
pub fn inbounds_omt_margin(p: (i32, i32), margin: i32) -> bool {
    p.0 >= margin && p.0 < 180 - margin && p.1 >= margin && p.1 < 180 - margin
}

// ---------------------------------------------------------------------------
// connect_closest_points — MST-based network builder
// ---------------------------------------------------------------------------

/// Connect a set of `(x, y)` OMT points into a network using a minimum
/// spanning tree with optional loop edges (1-in-10 chance).
///
/// # Algorithm (port of `overmap::connect_closest_points`)
///
/// 1. Enumerate every pairwise edge, weighted by `trig_dist`.
/// 2. Sort edges shortest-first.
/// 3. Use union-find on point indices to track subgraphs:
///    - Neither connected -> new subgraph; connect.
///    - One connected -> add to that subgraph; connect.
///    - Both connected, different subgraphs -> merge; connect.
///    - Both connected, same subgraph -> 1-in-10 loop edge; connect.
/// 4. Call `build_fn` for every connected pair.
///
/// # Parameters
///
/// - `points`: the OMT tile positions to connect.
/// - `z`: the z-level on which to build the connections.
/// - `connection_type`: the type of connection (road, rail, trail, etc.).
/// - `rng`: deterministic RNG for loop-edge decisions.
/// - `build_fn`: called for each connected pair `(from, to, z, conn_type)`.
pub fn connect_closest_points(
    points: &[(i32, i32)],
    z: i32,
    connection_type: ConnectionType,
    rng: &mut XorShiftRng,
    mut build_fn: impl FnMut((i32, i32), (i32, i32), i32, ConnectionType),
) {
    if points.len() < 2 {
        return;
    }

    let n = points.len();

    // 1. Build every pairwise edge with trig_dist as weight.
    let mut edges: Vec<(f32, usize, usize)> = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n - 1 {
        for j in i + 1..n {
            let dist = trig_dist(points[i], points[j]);
            edges.push((dist, i, j));
        }
    }

    // 2. Sort from shortest to longest.
    edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Union-find: subgraphs[i] tracks which subgraph point i belongs to.
    //    -1 means unconnected.
    let mut subgraphs: Vec<i32> = vec![-1; n];

    for (_dist, i, j) in edges {
        let connect = if subgraphs[i] < 0 && subgraphs[j] < 0 {
            // Neither point connected — create a new subgraph.
            subgraphs[i] = i as i32;
            subgraphs[j] = i as i32;
            true
        } else if subgraphs[i] < 0 {
            // i is new, j is connected — add i to j's subgraph.
            subgraphs[i] = subgraphs[j];
            true
        } else if subgraphs[j] < 0 {
            // j is new, i is connected — add j to i's subgraph.
            subgraphs[j] = subgraphs[i];
            true
        } else if subgraphs[i] != subgraphs[j] {
            // Different subgraphs — merge them.
            let dead = subgraphs[j];
            let alive = subgraphs[i];
            for k in 0..n {
                if subgraphs[k] == dead {
                    subgraphs[k] = alive;
                }
            }
            true
        } else {
            // Same subgraph — 1-in-10 chance for a loop edge.
            rng.one_in(10)
        };

        if connect {
            build_fn(points[i], points[j], z, connection_type);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // trig_dist / square_dist
    // -----------------------------------------------------------------------

    #[test]
    fn trig_dist_basic() {
        assert!((trig_dist((0, 0), (3, 4)) - 5.0).abs() < 1e-6);
        assert_eq!(trig_dist((5, 5), (5, 5)), 0.0);
    }

    #[test]
    fn square_dist_basic() {
        assert_eq!(square_dist((0, 0), (3, 4)), 25);
        assert_eq!(square_dist((5, 5), (5, 5)), 0);
        assert_eq!(square_dist((0, 0), (0, 0)), 0);
    }

    // -----------------------------------------------------------------------
    // line_between
    // -----------------------------------------------------------------------

    #[test]
    fn line_horizontal() {
        let pts = line_between((0, 0), (5, 0));
        assert_eq!(pts.len(), 6);
        assert_eq!(pts[0], (0, 0));
        assert_eq!(pts[5], (5, 0));
        for p in &pts {
            assert_eq!(p.1, 0);
        }
    }

    #[test]
    fn line_vertical() {
        let pts = line_between((3, 1), (3, 5));
        assert_eq!(pts.len(), 5);
        assert_eq!(pts[0], (3, 1));
        assert!(pts.last().copied() == Some((3, 5)));
    }

    #[test]
    fn line_diagonal() {
        let pts = line_between((0, 0), (3, 3));
        assert!(pts.len() >= 4, "got {} points", pts.len());
        assert_eq!(pts[0], (0, 0));
    }

    #[test]
    fn line_negative_direction() {
        let pts = line_between((5, 5), (2, 2));
        assert_eq!(pts[0], (5, 5));
        assert_eq!(*pts.last().unwrap(), (2, 2));
    }

    // -----------------------------------------------------------------------
    // inbounds
    // -----------------------------------------------------------------------

    #[test]
    fn inbounds_basic() {
        assert!(inbounds_omt((0, 0)));
        assert!(inbounds_omt((179, 179)));
        assert!(!inbounds_omt((-1, 0)));
        assert!(!inbounds_omt((0, 180)));
        assert!(!inbounds_omt((180, 0)));
    }

    #[test]
    fn inbounds_margin() {
        assert!(!inbounds_omt_margin((0, 0), 5));
        assert!(inbounds_omt_margin((5, 5), 5));
        assert!(!inbounds_omt_margin((179, 179), 5));
        assert!(inbounds_omt_margin((174, 174), 5));
    }

    // -----------------------------------------------------------------------
    // closest_points_first
    // -----------------------------------------------------------------------

    #[test]
    fn closest_points_first_includes_center() {
        let pts = closest_points_first((10, 10), 3);
        assert!(pts.contains(&(10, 10)));
    }

    #[test]
    fn closest_points_first_sorted_by_radius() {
        let pts = closest_points_first((0, 0), 2);
        // Radius 0 first
        assert_eq!(pts[0], (0, 0));
        // All radius-1 points (Chebyshev = 1) are the 8 points with
        // max(|dx|,|dy|) == 1. They appear before radius-2 points.
        let cheb = |p: (i32, i32)| p.0.abs().max(p.1.abs());
        let r1_end = 1 + 8; // center + 8 points at Chebyshev radius 1
        for i in 1..r1_end {
            assert_eq!(cheb(pts[i]), 1, "point {:?} should be Chebyshev radius 1", pts[i]);
        }
        // Everything after is radius 2.
        for i in r1_end..pts.len() {
            assert_eq!(cheb(pts[i]), 2, "point {:?} should be Chebyshev radius 2", pts[i]);
        }
    }

    // -----------------------------------------------------------------------
    // point_flood_fill_4
    // -----------------------------------------------------------------------

    #[test]
    fn flood_fill_empty_if_predicate_fails_on_start() {
        let pts = point_flood_fill_4((5, 5), (0, 0, 10, 10), |_| false);
        assert!(pts.is_empty());
    }

    #[test]
    fn flood_fill_out_of_bounds_start() {
        let pts = point_flood_fill_4((-1, 5), (0, 0, 10, 10), |_| true);
        assert!(pts.is_empty());
    }

    #[test]
    fn flood_fill_4_full_region() {
        let pts = point_flood_fill_4((0, 0), (0, 0, 3, 3), |_| true);
        assert_eq!(pts.len(), 9);
        assert!(pts.contains(&(0, 0)));
        assert!(pts.contains(&(2, 2)));
    }

    #[test]
    fn flood_fill_respects_predicate() {
        // Only even coordinates pass, starting at (0,0).
        // Because 4-connected steps require going through odd coordinates
        // to reach (2,0) or (2,2), only (0,0) is reachable.
        let pts = point_flood_fill_4((0, 0), (0, 0, 3, 3), |p| p.0 % 2 == 0 && p.1 % 2 == 0);
        assert_eq!(pts, vec![(0, 0)]);

        // With a permissive predicate, all 9 tiles in 3x3 are reachable.
        let all = point_flood_fill_4((0, 0), (0, 0, 3, 3), |_| true);
        assert_eq!(all.len(), 9);
    }

    // -----------------------------------------------------------------------
    // connect_closest_points
    // -----------------------------------------------------------------------

    #[test]
    fn connect_two_points_calls_build_fn_once() {
        let points = vec![(0, 0), (10, 10)];
        let mut count = 0u32;
        connect_closest_points(
            &points,
            0,
            ConnectionType::InterCityRoad,
            &mut XorShiftRng::new(1),
            |_, _, _, _| {
                count += 1;
            },
        );
        assert_eq!(count, 1);
    }

    #[test]
    fn connect_zero_or_one_point_does_nothing() {
        let mut called = false;
        connect_closest_points(
            &[],
            0,
            ConnectionType::Railroad,
            &mut XorShiftRng::new(1),
            |_, _, _, _| called = true,
        );
        assert!(!called);

        connect_closest_points(
            &[(5, 5)],
            0,
            ConnectionType::Railroad,
            &mut XorShiftRng::new(1),
            |_, _, _, _| called = true,
        );
        assert!(!called);
    }

    #[test]
    fn connect_three_points_forms_connected_graph() {
        // Three points in a line — MST should connect all with >=2 edges
        // (barring loop-edge luck for a 3rd).
        let points = vec![(0, 0), (5, 0), (10, 0)];
        let mut connections: Vec<((i32, i32), (i32, i32))> = Vec::new();
        connect_closest_points(
            &points,
            0,
            ConnectionType::ForestTrail,
            &mut XorShiftRng::new(42),
            |a, b, _, _| {
                connections.push((a, b));
            },
        );
        // At minimum the MST gives 2 edges. Loops may add more.
        assert!(connections.len() >= 2, "got {} connections", connections.len());

        // Verify spanning property: all points in one component.
        let n = points.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], x: usize) -> usize {
            if p[x] != x {
                p[x] = find(p, p[x]);
            }
            p[x]
        }
        fn union(p: &mut [usize], a: usize, b: usize) {
            let ra = find(p, a);
            let rb = find(p, b);
            if ra != rb {
                p[ra] = rb;
            }
        }

        for (a, b) in &connections {
            let ia = points.iter().position(|&p| p == *a).unwrap();
            let ib = points.iter().position(|&p| p == *b).unwrap();
            union(&mut parent, ia, ib);
        }

        let root = find(&mut parent, 0);
        for i in 1..n {
            assert_eq!(find(&mut parent, i), root, "point {i} not connected");
        }
    }

    #[test]
    fn connect_deterministic_with_same_seed() {
        let points: Vec<(i32, i32)> = (0..8).map(|i| (i * 20, ((i as i32 * 7) % 50))).collect();

        let run = |seed| {
            let mut conns = Vec::new();
            connect_closest_points(
                &points,
                -1,
                ConnectionType::Sewer,
                &mut XorShiftRng::new(seed),
                |a, b, _, _| {
                    conns.push((a, b));
                },
            );
            conns
        };

        let a = run(12345);
        let b = run(12345);
        assert_eq!(a, b);
    }
}
