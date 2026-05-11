//! Tests for the grid-based spatial index.
//!
//! `EntitySpatialIndex` is a pure data structure (no ECS systems) that
//! maps world positions to entities for efficient radius queries.
//! All tests use `Entity::from_bits` for entity handles and
//! `WorldPos::new` for positions.

use bevy_ecs::entity::Entity;
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_map::spatial::EntitySpatialIndex;

/// Helper: create an entity from a numeric id.
fn ent(id: u64) -> Entity {
    Entity::from_bits(id)
}

/// Helper: create a `WorldPos` at `(x, y)` on z-level 0.
fn wp(x: i32, y: i32) -> WorldPos {
    WorldPos::new(x, y, ZLevel::new(0))
}

/// Helper: create a `WorldPos` at `(x, y, z)`.
fn wp_z(x: i32, y: i32, z: i8) -> WorldPos {
    WorldPos::new(x, y, ZLevel::new(z))
}

// ---------------------------------------------------------------------------
// Basic operations
// ---------------------------------------------------------------------------

#[test]
fn new_index_empty() {
    let idx = EntitySpatialIndex::new();
    assert_eq!(idx.entity_count(), 0);
    assert_eq!(idx.cell_count(), 0);
}

#[test]
fn insert_one_entity() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(1);

    idx.update_position(e, wp(0, 0));

    assert_eq!(idx.entity_count(), 1);
    assert_eq!(idx.cell_count(), 1);

    let found = idx.query_radius(wp(0, 0), 1.0);
    assert_eq!(found, vec![e]);
}

#[test]
fn insert_two_entities() {
    let mut idx = EntitySpatialIndex::new();
    let e1 = ent(1);
    let e2 = ent(2);

    idx.update_position(e1, wp(0, 0));
    idx.update_position(e2, wp(100, 50));

    let found_0 = idx.query_radius(wp(0, 0), 1.0);
    assert!(found_0.contains(&e1));
    assert!(!found_0.contains(&e2));

    let found_100 = idx.query_radius(wp(100, 50), 1.0);
    assert!(found_100.contains(&e2));
    assert!(!found_100.contains(&e1));
}

#[test]
fn entity_count_reflects_inserts() {
    let mut idx = EntitySpatialIndex::new();
    assert_eq!(idx.entity_count(), 0);

    idx.update_position(ent(1), wp(0, 0));
    assert_eq!(idx.entity_count(), 1);

    idx.update_position(ent(2), wp(10, 10));
    assert_eq!(idx.entity_count(), 2);

    idx.update_position(ent(3), wp(20, 20));
    assert_eq!(idx.entity_count(), 3);
}

#[test]
fn remove_entity() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(42);

    idx.update_position(e, wp(5, 5));
    assert_eq!(idx.entity_count(), 1);

    idx.remove_entity(e);
    assert_eq!(idx.entity_count(), 0);
    assert!(idx.query_radius(wp(5, 5), 10.0).is_empty());
}

#[test]
fn update_position() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(7);

    // Place at (0, 0)
    idx.update_position(e, wp(0, 0));
    assert!(idx.query_radius(wp(0, 0), 1.0).contains(&e));

    // Move to (100, 50)
    idx.update_position(e, wp(100, 50));
    // Old position should no longer return it
    assert!(!idx.query_radius(wp(0, 0), 1.0).contains(&e));
    // New position should return it
    assert!(idx.query_radius(wp(100, 50), 1.0).contains(&e));
}

#[test]
fn update_same_position_noop() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(9);

    idx.update_position(e, wp(10, 10));
    assert_eq!(idx.entity_count(), 1);
    assert_eq!(idx.cell_count(), 1);

    // Updating to the same position should not change counts
    idx.update_position(e, wp(10, 10));
    assert_eq!(idx.entity_count(), 1);
    assert_eq!(idx.cell_count(), 1);
}

// ---------------------------------------------------------------------------
// Radius queries
// ---------------------------------------------------------------------------

#[test]
fn query_radius_finds_nearby() {
    let mut idx = EntitySpatialIndex::new();
    let a = ent(1);
    let b = ent(2);

    // Two entities very close together
    idx.update_position(a, wp(10, 10));
    idx.update_position(b, wp(12, 12));

    let found = idx.query_radius(wp(11, 11), 5.0);
    assert!(found.contains(&a));
    assert!(found.contains(&b));
}

#[test]
fn query_radius_excludes_far() {
    let mut idx = EntitySpatialIndex::new();
    let close = ent(1);
    let far = ent(2);

    idx.update_position(close, wp(10, 10));
    idx.update_position(far, wp(1000, 1000));

    let found = idx.query_radius(wp(10, 10), 20.0);
    assert!(found.contains(&close));
    assert!(!found.contains(&far));
}

#[test]
fn query_radius_zero() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(1);

    idx.update_position(e, wp(5, 5));

    // Radius 0 — query_radius still checks the current cell grid, so it
    // still finds the entity in the same cell.  This test documents the
    // current behaviour.
    let found = idx.query_radius(wp(5, 5), 0.0);
    // With radius 0, cell_radius = 1 (ceil(0/16) + 1), so the entity's
    // cell is still visited.
    assert!(found.contains(&e));
}

#[test]
fn query_radius_large() {
    let mut idx = EntitySpatialIndex::new();
    let e1 = ent(1);
    let e2 = ent(2);
    let e3 = ent(3);

    idx.update_position(e1, wp(-50, -50));
    idx.update_position(e2, wp(0, 0));
    idx.update_position(e3, wp(200, 200));

    // Large radius covering everything
    let found = idx.query_radius(wp(0, 0), 500.0);
    assert_eq!(found.len(), 3);
    assert!(found.contains(&e1));
    assert!(found.contains(&e2));
    assert!(found.contains(&e3));
}

#[test]
fn query_radius_boundary() {
    let mut idx = EntitySpatialIndex::new();
    let edge = ent(1);

    // CELL_SIZE = 16, so the cell boundary is at x=16.
    // Place entity just at x=15 (still in cell 0).
    idx.update_position(edge, wp(15, 0));

    // Query from x=0 with a small radius — should still find the entity
    // since it's in an adjacent cell.
    let found = idx.query_radius(wp(0, 0), 1.0);
    assert!(
        found.contains(&edge),
        "entity at boundary should be reachable from adjacent cell"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn remove_nonexistent_entity() {
    let mut idx = EntitySpatialIndex::new();
    let phantom = ent(999);

    // Removing an entity that was never inserted should not panic.
    idx.remove_entity(phantom);
    assert_eq!(idx.entity_count(), 0);
    assert_eq!(idx.cell_count(), 0);
}

#[test]
fn duplicate_entity_reinsertion() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(1);

    // Insert at (0, 0)
    idx.update_position(e, wp(0, 0));
    assert_eq!(idx.entity_count(), 1);

    // Re-insert same entity at (50, 50) — final position should be (50, 50)
    idx.update_position(e, wp(50, 50));
    assert_eq!(idx.entity_count(), 1);

    // Should only be found at the new position
    assert!(!idx.query_radius(wp(0, 0), 1.0).contains(&e));
    assert!(idx.query_radius(wp(50, 50), 1.0).contains(&e));
}

// ---------------------------------------------------------------------------
// 3D z-level separation
// ---------------------------------------------------------------------------

#[test]
fn entities_on_different_z_are_separate_cells() {
    let mut idx = EntitySpatialIndex::new();
    let ground = ent(1);
    let roof = ent(2);

    // Same x,y but different z-level
    idx.update_position(ground, wp(10, 10));
    idx.update_position(roof, wp_z(10, 10, 3));

    // query_radius_2d (same z) should only find ground
    let found_2d = idx.query_radius_2d(wp(10, 10), 5.0);
    assert!(found_2d.contains(&ground));
    assert!(!found_2d.contains(&roof));

    // query_radius 3D with large radius should find both
    let found_3d = idx.query_radius(wp(10, 10), 100.0);
    assert!(found_3d.contains(&ground));
    assert!(found_3d.contains(&roof));
}

#[test]
fn entity_moves_across_z_levels() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(1);

    // Start on ground
    idx.update_position(e, wp_z(0, 0, 0));
    assert!(idx.query_radius(wp_z(0, 0, 0), 1.0).contains(&e));
    assert!(!idx.query_radius(wp_z(0, 0, 3), 1.0).contains(&e));

    // Move up 3 levels
    idx.update_position(e, wp_z(0, 0, 3));
    assert!(!idx.query_radius(wp_z(0, 0, 0), 1.0).contains(&e));
    assert!(idx.query_radius(wp_z(0, 0, 3), 1.0).contains(&e));
}

#[test]
fn query_radius_3d_includes_adjacent_z() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(1);

    // Entity on z=1
    idx.update_position(e, wp_z(15, 15, 1));

    // Query from z=0 with a radius that covers the vertical distance
    // Z_CELL_SIZE = 1, so entity is in cell z=1, query originates from z=0
    // cell_radius at least 1 → dz range [-1, 1] → covers cell z=1
    let found = idx.query_radius(wp_z(15, 15, 0), 5.0);
    assert!(
        found.contains(&e),
        "3D query from z=0 should reach entity on z=1 when radius covers it"
    );
}

#[test]
fn query_radius_2d_excludes_other_z() {
    let mut idx = EntitySpatialIndex::new();
    let e = ent(1);

    idx.update_position(e, wp_z(15, 15, 2));

    // 2D query from z=0 should NOT find entity on z=2
    let found = idx.query_radius_2d(wp_z(15, 15, 0), 100.0);
    assert!(
        !found.contains(&e),
        "2D query from z=0 should NOT reach entity on z=2"
    );
}
