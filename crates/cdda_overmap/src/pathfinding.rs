//! A* pathfinding — mirrors CDDA's `pf::greedy_path` in `simple_pathfinding.cpp`.
//!
//! The core entry point is [`greedy_path`], which accepts a start/end position,
//! a bounding box, and a [`TwoNodeScoringFn`] that evaluates candidate nodes.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::direction::OmDirection;

// ---------------------------------------------------------------------------
// DirectedNode
// ---------------------------------------------------------------------------

/// A node in a directed path, containing position and the direction of travel
/// used to reach this node (or `Invalid` for the start node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectedNode {
    pub pos: (i32, i32),
    pub dir: OmDirection,
}

impl DirectedNode {
    /// Create a start node — no previous direction.
    pub fn start(pos: (i32, i32)) -> Self {
        Self {
            pos,
            dir: OmDirection::Invalid,
        }
    }
}

// ---------------------------------------------------------------------------
// NodeScore
// ---------------------------------------------------------------------------

/// Score returned by a node scoring function.
///
/// `node_cost < 0` means the node is rejected (impassable).
/// `estimated_dest_cost` is the heuristic estimate to the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeScore {
    pub node_cost: i32,
    pub estimated_dest_cost: i32,
}

impl NodeScore {
    /// Sentinel value for rejected / impassable nodes.
    pub const REJECTED: Self = Self {
        node_cost: -1,
        estimated_dest_cost: -1,
    };

    /// Create a valid score.
    #[inline]
    pub fn new(node_cost: i32, estimated_dest_cost: i32) -> Self {
        Self {
            node_cost,
            estimated_dest_cost,
        }
    }

    /// Returns `true` if this node is rejected (impassable).
    #[inline]
    pub fn is_rejected(&self) -> bool {
        self.node_cost < 0
    }

    /// The total priority for the priority queue (= node_cost + estimated_dest_cost).
    #[inline]
    pub fn priority(&self) -> i32 {
        self.node_cost + self.estimated_dest_cost
    }
}

// ---------------------------------------------------------------------------
// TwoNodeScoringFn
// ---------------------------------------------------------------------------

/// A scoring function that evaluates a candidate node, optionally given the
/// previous node for context (direction-change penalties, etc.).
pub type TwoNodeScoringFn = Box<dyn Fn(DirectedNode, Option<DirectedNode>) -> NodeScore>;

// ---------------------------------------------------------------------------
// Internal priority-queue entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PqEntry {
    pos: (i32, i32),
    dir: OmDirection,
    priority: i32,
}

impl PartialEq for PqEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PqEntry {}

impl PartialOrd for PqEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PqEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse so BinaryHeap acts as a min-heap.
        other.priority.cmp(&self.priority)
    }
}

// ---------------------------------------------------------------------------
// greedy_path
// ---------------------------------------------------------------------------

/// A* / Best-First Search over a 2D grid of cardinal moves.
///
/// Matches CDDA's `pf::greedy_path` in `simple_pathfinding.cpp` L48–136.
///
/// `scoring_fn` is called for every candidate node (including the start).
/// If it returns [`NodeScore::REJECTED`], that node is skipped.
///
/// # Algorithm
///
/// 1. Reject if start == end or either is out of bounds.
/// 2. Score the start node; reject if impassable.
/// 3. Push the start node onto a min-heap (by `priority = node_cost + estimated_dest_cost`).
/// 4. Loop:
///    a. Pop the best-looking open node.
///    b. Mark it closed (visited).
///    c. If it's the destination, reconstruct and return the reverse path.
///    d. For each of the 4 cardinal directions:
///       - Compute neighbour position.
///       - Skip if out of bounds or already closed.
///       - Score via `scoring_fn`.
///       - Skip if rejected.
///       - If this is a better path than any previously recorded, update
///         `open_score` / `dirs` and push onto the heap.
/// 5. If the heap empties without reaching `end`, return an empty path.
///
/// The returned path is in order from **destination to start** (matching CDDA
/// convention), so callers that want start→end must `reverse()`.
pub fn greedy_path<F>(
    start: (i32, i32),
    end: (i32, i32),
    max: (i32, i32),
    scoring_fn: &F,
) -> Vec<DirectedNode>
where
    F: Fn(DirectedNode, Option<DirectedNode>) -> NodeScore,
{
    // Early exits.
    if start == end {
        return Vec::new();
    }
    if !inbounds(start, max) || !inbounds(end, max) {
        return Vec::new();
    }

    let start_node = DirectedNode::start(start);
    let score = scoring_fn(start_node, None);
    if score.is_rejected() {
        return Vec::new();
    }

    let map_size = (max.0 as usize) * (max.1 as usize);

    // Per-cell storage.
    let mut closed = vec![false; map_size];
    // open_score[idx] == 0 means unvisited; otherwise stores the best priority seen.
    let mut open_score = vec![0i32; map_size];
    // dirs stores the direction that was taken TO this cell (for backtracking).
    let mut dirs = vec![0u8; map_size];

    let mut heap = BinaryHeap::<PqEntry>::with_capacity(1024);

    let idx = |x: i32, y: i32| -> usize { y as usize * max.0 as usize + x as usize };

    let start_idx = idx(start.0, start.1);
    open_score[start_idx] = i32::MAX; // sentinel so first push always wins
    heap.push(PqEntry {
        pos: start,
        dir: OmDirection::Invalid,
        priority: score.priority(),
    });

    while let Some(entry) = heap.pop() {
        let i = idx(entry.pos.0, entry.pos.1);

        // Stale entry — already closed or a better path was found.
        if closed[i] || open_score[i] < entry.priority {
            continue;
        }

        closed[i] = true;

        // Destination reached — reconstruct path (dest → start).
        if entry.pos == end {
            let mut path = Vec::new();
            let mut p = end;
            while p != start {
                let pi = idx(p.0, p.1);
                let dir = OmDirection::from_index(dirs[pi] as usize);
                path.push(DirectedNode { pos: p, dir });
                // Step backward along the stored direction.
                let rev = dir.opposite();
                let disp = rev.displace(1);
                p = (p.0 + disp.0, p.1 + disp.1);
            }
            path.push(DirectedNode::start(start));
            return path;
        }

        // Expand neighbours (4 cardinal).
        for &dir in &OmDirection::ALL {
            let disp = dir.displace(1);
            let np = (entry.pos.0 + disp.0, entry.pos.1 + disp.1);

            if !inbounds(np, max) {
                continue;
            }
            let ni = idx(np.0, np.1);
            if closed[ni] {
                continue;
            }

            let cur_node = DirectedNode { pos: np, dir };
            let prev_node = if entry.dir == OmDirection::Invalid {
                None
            } else {
                Some(DirectedNode {
                    pos: entry.pos,
                    dir: entry.dir,
                })
            };

            let ns = scoring_fn(cur_node, prev_node);
            if ns.is_rejected() {
                continue;
            }

            let priority = ns.priority();

            if open_score[ni] == 0 || priority < open_score[ni] {
                open_score[ni] = priority;
                // Store the direction that got us here (for backtracking).
                dirs[ni] = dir.to_index() as u8;
                heap.push(PqEntry {
                    pos: np,
                    dir,
                    priority,
                });
            }
        }
    }

    // No path found.
    Vec::new()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn inbounds(p: (i32, i32), max: (i32, i32)) -> bool {
    p.0 >= 0 && p.0 < max.0 && p.1 >= 0 && p.1 < max.1
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_line_horizontal() {
        // From (1,1) to (4,1) with uniform cost.
        let scorer = |node: DirectedNode, _prev: Option<DirectedNode>| {
            let dist = (4 - node.pos.0).abs() + (1 - node.pos.1).abs();
            NodeScore::new(1, dist)
        };

        let path = greedy_path((1, 1), (4, 1), (10, 10), &scorer);
        assert!(!path.is_empty(), "should find a path");

        // Path is dest→start, so first node should be (4,1).
        assert_eq!(path[0].pos, (4, 1));
        // Last node should be (1,1).
        assert_eq!(path.last().unwrap().pos, (1, 1));
    }

    #[test]
    fn start_equals_end_returns_empty() {
        let scorer = |_node: DirectedNode, _prev: Option<DirectedNode>| NodeScore::new(0, 0);
        let path = greedy_path((3, 3), (3, 3), (10, 10), &scorer);
        assert!(path.is_empty());
    }

    #[test]
    fn out_of_bounds_start_returns_empty() {
        let scorer = |_: DirectedNode, _: Option<DirectedNode>| NodeScore::new(1, 0);
        let path = greedy_path((-1, 0), (5, 5), (10, 10), &scorer);
        assert!(path.is_empty());
    }

    #[test]
    fn rejected_start_returns_empty() {
        let scorer = |_: DirectedNode, _: Option<DirectedNode>| NodeScore::REJECTED;
        let path = greedy_path((1, 1), (5, 5), (10, 10), &scorer);
        assert!(path.is_empty());
    }

    #[test]
    fn blocked_path_returns_empty() {
        // Reject all nodes east of x=3, forcing no path from (1,1) to (5,1).
        let scorer = |node: DirectedNode, _prev: Option<DirectedNode>| {
            if node.pos.0 > 3 {
                NodeScore::REJECTED
            } else {
                let dist = (5 - node.pos.0).abs() + (1 - node.pos.1).abs();
                NodeScore::new(1, dist)
            }
        };
        let path = greedy_path((1, 1), (5, 1), (10, 10), &scorer);
        assert!(path.is_empty());
    }
}
