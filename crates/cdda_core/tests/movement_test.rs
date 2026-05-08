//! Movement tests — translated from CDDA's movement/speed testing patterns.
//!
//! Tests move points, speed, terrain move cost, and CDDA-derived movement
//! formulas.

use cdda_core::core::components::actor::{MovePoints, Speed};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Pure helper functions — CDDA-derived movement formulas
// ---------------------------------------------------------------------------

/// Base move cost for the given terrain (turns to enter).
/// Cost 0 is impassable.
fn terrain_move_cost(cost: i32) -> i32 {
    if cost == 0 {
        i32::MAX
    } else {
        cost
    }
}

/// Total move points per turn based on speed.
fn mp_per_turn(speed: i32) -> i32 {
    speed
}

/// Whether the entity can act (has positive move points and is not stunned).
fn can_act(mp: i32, stunned: bool) -> bool {
    mp > 0 && !stunned
}

/// Move cost modifier from bleeding (bleeding adds 25% cost).
fn bleeding_move_modifier(bleeding: bool, base_cost: i32) -> i32 {
    if bleeding {
        (base_cost as f64 * 1.25).ceil() as i32
    } else {
        base_cost
    }
}

/// Stamina cost for moving into a tile (higher for rough terrain).
fn stamina_cost(terrain_cost: i32) -> i32 {
    if terrain_cost == 0 || terrain_cost == i32::MAX {
        0
    } else if terrain_cost <= 50 {
        terrain_cost / 2
    } else {
        terrain_cost
    }
}

// ---------------------------------------------------------------------------
// Basic component tests
// ---------------------------------------------------------------------------

#[test]
fn move_points_default() {
    let mut test = TestBed::new();
    test.register::<MovePoints>();

    let e = test.spawn((MovePoints(0),));
    let mp = test.get::<MovePoints>(e).unwrap();
    assert_eq!(mp.0, 0);
}

#[test]
fn move_points_positive() {
    let mut test = TestBed::new();
    test.register::<MovePoints>();

    let e = test.spawn((MovePoints(100),));
    let mp = test.get::<MovePoints>(e).unwrap();
    assert_eq!(mp.0, 100);
}

#[test]
fn move_points_negative() {
    let mut test = TestBed::new();
    test.register::<MovePoints>();

    let e = test.spawn((MovePoints(-50),));
    let mp = test.get::<MovePoints>(e).unwrap();
    assert_eq!(mp.0, -50);
}

#[test]
fn speed_default() {
    let mut test = TestBed::new();
    test.register::<Speed>();

    let e = test.spawn((Speed(100),));
    let speed = test.get::<Speed>(e).unwrap();
    assert_eq!(speed.0, 100);
}

#[test]
fn speed_custom() {
    let mut test = TestBed::new();
    test.register::<Speed>();

    let e1 = test.spawn((Speed(80),));
    let e2 = test.spawn((Speed(120),));
    let e3 = test.spawn((Speed(200),));

    assert_eq!(test.get::<Speed>(e1).unwrap().0, 80);
    assert_eq!(test.get::<Speed>(e2).unwrap().0, 120);
    assert_eq!(test.get::<Speed>(e3).unwrap().0, 200);
}

#[test]
fn speed_zero() {
    let mut test = TestBed::new();
    test.register::<Speed>();

    let e = test.spawn((Speed(0),));
    let speed = test.get::<Speed>(e).unwrap();
    assert_eq!(speed.0, 0);
}

// ---------------------------------------------------------------------------
// Pure function formula tests
// ---------------------------------------------------------------------------

#[test]
fn terrain_impassable() {
    assert_eq!(terrain_move_cost(0), i32::MAX);
}

#[test]
fn terrain_passable() {
    assert_eq!(terrain_move_cost(100), 100);
}

#[test]
fn terrain_road() {
    assert_eq!(terrain_move_cost(30), 30);
}

#[test]
fn mp_per_turn_normal() {
    assert_eq!(mp_per_turn(100), 100);
}

#[test]
fn mp_per_turn_slow() {
    assert_eq!(mp_per_turn(50), 50);
}

#[test]
fn can_act_with_mp() {
    assert!(can_act(10, false));
    assert!(can_act(1, false));
}

#[test]
fn can_act_stunned() {
    assert!(!can_act(10, true));
    assert!(!can_act(100, true));
    // Also cannot act if mp <= 0 even if not stunned
    assert!(!can_act(0, false));
    assert!(!can_act(-5, false));
}

#[test]
fn bleeding_move_cost() {
    // Base cost 100, bleeding → 125
    assert_eq!(bleeding_move_modifier(true, 100), 125);
    // No bleeding → no modifier
    assert_eq!(bleeding_move_modifier(false, 100), 100);
    // Base cost 30, bleeding → 38 (ceil(30 * 1.25) = ceil(37.5) = 38)
    assert_eq!(bleeding_move_modifier(true, 30), 38);
}

#[test]
fn stamina_cost_tests() {
    // Impassable terrain → 0
    assert_eq!(stamina_cost(0), 0);
    assert_eq!(stamina_cost(i32::MAX), 0);
    // Light terrain (≤50) → half
    assert_eq!(stamina_cost(50), 25);
    assert_eq!(stamina_cost(30), 15);
    // Rough terrain (>50) → full cost
    assert_eq!(stamina_cost(100), 100);
    assert_eq!(stamina_cost(80), 80);
}
