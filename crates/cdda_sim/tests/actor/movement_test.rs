//! Movement tests — translated from CDDA's movement/speed testing patterns.

use cdda_components::actor::ActionPoints;
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Pure helper functions — CDDA-derived movement formulas
// ---------------------------------------------------------------------------

fn terrain_move_cost(cost: i32) -> i32 {
    if cost == 0 { i32::MAX } else { cost }
}

fn mp_per_turn(speed: i32) -> i32 {
    speed
}

fn can_act(mp: i32, stunned: bool) -> bool {
    mp > 0 && !stunned
}

fn bleeding_move_modifier(bleeding: bool, base_cost: i32) -> i32 {
    if bleeding { (base_cost as f64 * 1.25).ceil() as i32 } else { base_cost }
}

fn stamina_cost(terrain_cost: i32) -> i32 {
    if terrain_cost == 0 || terrain_cost == i32::MAX { 0 }
    else if terrain_cost <= 50 { terrain_cost / 2 }
    else { terrain_cost }
}

// ---------------------------------------------------------------------------
// ActionPoints component tests
// ---------------------------------------------------------------------------

#[test]
fn move_points_default() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints { current: 0, speed: 100 },));
    let ap = test.get::<ActionPoints>(e).unwrap();
    assert_eq!(ap.current, 0);
}

#[test]
fn move_points_positive() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints { current: 100, speed: 100 },));
    let ap = test.get::<ActionPoints>(e).unwrap();
    assert_eq!(ap.current, 100);
}

#[test]
fn move_points_negative() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints { current: -50, speed: 100 },));
    let ap = test.get::<ActionPoints>(e).unwrap();
    assert_eq!(ap.current, -50);
}

#[test]
fn speed_default() {
    let ap = ActionPoints::default();
    assert_eq!(ap.speed, 100);
}

#[test]
fn speed_custom() {
    let e1 = ActionPoints::new(80);
    let e2 = ActionPoints::new(120);
    let e3 = ActionPoints::new(200);

    assert_eq!(e1.speed, 80);
    assert_eq!(e2.speed, 120);
    assert_eq!(e3.speed, 200);
}

#[test]
fn speed_zero() {
    let ap = ActionPoints::new(0);
    assert_eq!(ap.speed, 0);
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
    assert!(!can_act(0, false));
    assert!(!can_act(-5, false));
}

#[test]
fn bleeding_move_cost() {
    assert_eq!(bleeding_move_modifier(true, 100), 125);
    assert_eq!(bleeding_move_modifier(false, 100), 100);
    assert_eq!(bleeding_move_modifier(true, 30), 38);
}

#[test]
fn stamina_cost_tests() {
    assert_eq!(stamina_cost(0), 0);
    assert_eq!(stamina_cost(i32::MAX), 0);
    assert_eq!(stamina_cost(50), 25);
    assert_eq!(stamina_cost(30), 15);
    assert_eq!(stamina_cost(100), 100);
    assert_eq!(stamina_cost(80), 80);
}
