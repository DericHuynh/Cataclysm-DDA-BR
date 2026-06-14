//! Action-point system tests — ported from Cataclysm-DDA-master:
//!   tests/move_cost_test.cpp  (walking / crouching / prone / downed costs)
//!   src/creature.cpp          (process_turn: moves += get_speed())
//!
//! CDDA reference:
//!   Creature::process_turn() → moves += get_speed()
//!   Default speed = 100 → 100 AP per turn
//!   Walk = 100 AP, Crouch = 200 AP (2×), Prone = 600 AP (6×)
//!   Downed status: 3× movement cost
//!   Terrain cost scales movement: rough terrain (200) → 2× base cost

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::def::IsDef;
use cdda_sim::actor::turn::tick_move_points;
use cdda_sim::actor::turn::{
    effective_move_cost, ActorTurn, TurnQueue, MOVE_COST_CROUCH, MOVE_COST_DOWNED_MULTIPLIER,
    MOVE_COST_PRONE, MOVE_COST_WALK, MP_MIN_FLOOR,
};
use cdda_sim::runtime::state::GameTime;
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Constants — match CDDA baseline values
// ---------------------------------------------------------------------------

/// Normal terrain movecost in CDDA (t_floor, t_pavement etc.).
const TERRAIN_NORMAL: i32 = 100;
/// Rough terrain movecost (CDDA: difficult ground).
const TERRAIN_ROUGH: i32 = 200;
/// Impassable terrain movecost (CDDA: walls, 0 = impassable).
const TERRAIN_IMPASSABLE: i32 = 0;

// ---------------------------------------------------------------------------
// Constants match CDDA values
// ---------------------------------------------------------------------------

/// CDDA: default character speed = 100 (from `get_speed_base()`).
#[test]
fn default_speed_is_100() {
    let ap = ActionPoints::default();
    assert_eq!(ap.speed, 100);
}

/// CDDA `run_cost(100) == 100` for a character wearing sneakers (baseline walk).
#[test]
fn walk_cost_constant_is_100() {
    assert_eq!(MOVE_COST_WALK, 100);
}

/// CDDA: `move_mode_crouch` doubles the walk cost → 200.
/// `run_cost(100) == 200` when crouching (uninjured, no encumbrance).
#[test]
fn crouch_cost_is_double_walk() {
    assert_eq!(MOVE_COST_CROUCH, MOVE_COST_WALK * 2);
}

/// CDDA: `move_mode_prone` = 6× walk → 600.
/// `run_cost(100) == 600` when prone (uninjured, no encumbrance).
#[test]
fn prone_cost_is_six_times_walk() {
    assert_eq!(MOVE_COST_PRONE, MOVE_COST_WALK * 6);
}

/// CDDA: `effect_downed` triples movement cost.
/// `run_cost(100) == 300` when knocked down.
#[test]
fn downed_triples_walk_cost() {
    let downed_cost = MOVE_COST_WALK * MOVE_COST_DOWNED_MULTIPLIER;
    assert_eq!(downed_cost, 300);
}

/// CDDA: downed also triples crouch and prone costs.
#[test]
fn downed_triples_crouch_cost() {
    assert_eq!(MOVE_COST_CROUCH * MOVE_COST_DOWNED_MULTIPLIER, 600);
}

#[test]
fn downed_triples_prone_cost() {
    assert_eq!(MOVE_COST_PRONE * MOVE_COST_DOWNED_MULTIPLIER, 1800);
}

// ---------------------------------------------------------------------------
// AP tick — ported from creature.cpp `process_turn`: moves += get_speed()
// ---------------------------------------------------------------------------

/// CDDA: each turn a creature gains exactly `get_speed()` move points.
/// With default speed=100, one tick grants 100 AP.
#[test]
fn tick_grants_speed_ap_per_turn() {
    let mut ap = ActionPoints::default(); // current=0, speed=100
    ap.tick();
    assert_eq!(ap.current, 100);
}

/// CDDA: AP accumulates over multiple turns.
/// After 3 ticks at speed=100: 300 AP.
#[test]
fn ap_accumulates_across_turns() {
    let mut ap = ActionPoints::default();
    ap.tick();
    ap.tick();
    ap.tick();
    assert_eq!(ap.current, 300);
}

/// AP starts at 50 (existing debt-free balance); one tick should add speed.
#[test]
fn tick_accumulates_on_existing_ap() {
    let mut ap = ActionPoints {
        current: 50,
        speed: 100,
    };
    ap.tick();
    assert_eq!(ap.current, 150);
}

/// Slow actor (speed=50) gains only 50 AP per tick.
#[test]
fn slow_actor_gains_less_ap_per_tick() {
    let mut ap = ActionPoints::new(50);
    ap.tick();
    assert_eq!(ap.current, 50);
}

/// Fast actor (speed=200) gains 200 AP per tick.
#[test]
fn fast_actor_gains_more_ap_per_tick() {
    let mut ap = ActionPoints::new(200);
    ap.tick();
    assert_eq!(ap.current, 200);
}

// ---------------------------------------------------------------------------
// Debt floor — CDDA analogue: LIMIT_RUNNING_SPRINT = -200
// Our implementation: floor = -(speed * 2).max(50)
// ---------------------------------------------------------------------------

/// At default speed=100, the debt floor is -200.
#[test]
fn debt_floor_default_speed_is_negative_200() {
    let mut ap = ActionPoints {
        current: -300,
        speed: 100,
    };
    ap.tick(); // -300 + 100 = -200 (clamped to floor)
    assert_eq!(ap.current, -200);
}

/// Deeply negative AP is clamped to the floor after ticking.
#[test]
fn deeply_negative_ap_clamped_to_floor() {
    let mut ap = ActionPoints {
        current: -10_000,
        speed: 100,
    };
    ap.tick(); // would be -9900 but clamped to -200
    assert_eq!(ap.current, -200);
}

/// At speed=200, the debt floor is -400.
#[test]
fn debt_floor_scales_with_speed() {
    let mut ap = ActionPoints {
        current: -1000,
        speed: 200,
    };
    ap.tick(); // would be -800 but clamped to -(200*2) = -400
    assert_eq!(ap.current, -400);
}

// ---------------------------------------------------------------------------
// Spending AP
// ---------------------------------------------------------------------------

/// CDDA: `mod_moves(-100)` after walking one tile; actor still can act.
#[test]
fn spending_100_ap_leaves_actor_able_to_act() {
    let mut ap = ActionPoints {
        current: 100,
        speed: 100,
    };
    ap.spend(MOVE_COST_WALK);
    assert_eq!(ap.current, 0);
    // MP_MIN_FLOOR = 25; 0 < 25 means cannot act — that's intentional
    assert!(ap.current < MP_MIN_FLOOR);
}

/// CDDA: spending more than available drives moves negative.
#[test]
fn spending_more_than_available_goes_negative() {
    let mut ap = ActionPoints {
        current: 20,
        speed: 100,
    };
    ap.spend(MOVE_COST_WALK); // 20 - 100 = -80
    assert_eq!(ap.current, -80);
}

/// Actors at or above MP_MIN_FLOOR can act; below cannot.
#[test]
fn mp_min_floor_gate() {
    assert!(MP_MIN_FLOOR > 0, "min floor must be positive");
    let ready = ActionPoints {
        current: MP_MIN_FLOOR,
        speed: 100,
    };
    assert!(ready.current >= MP_MIN_FLOOR);

    let not_ready = ActionPoints {
        current: MP_MIN_FLOOR - 1,
        speed: 100,
    };
    assert!(not_ready.current < MP_MIN_FLOOR);
}

// ---------------------------------------------------------------------------
// effective_move_cost — ported from move_cost_test.cpp / run_cost() analogue
// ---------------------------------------------------------------------------

/// CDDA: normal terrain (movecost=100) does not change the base walk cost.
/// `effective_move_cost(100, 100) == 100`.
#[test]
fn normal_terrain_preserves_walk_cost() {
    assert_eq!(effective_move_cost(MOVE_COST_WALK, TERRAIN_NORMAL), 100);
}

/// CDDA: rough terrain (movecost=200) doubles the walk cost.
/// `effective_move_cost(100, 200) == 200`.
#[test]
fn rough_terrain_doubles_walk_cost() {
    assert_eq!(effective_move_cost(MOVE_COST_WALK, TERRAIN_ROUGH), 200);
}

/// CDDA: impassable terrain (movecost=0) cannot be entered.
/// `effective_move_cost(100, 0) == i32::MAX`.
#[test]
fn impassable_terrain_returns_max() {
    assert_eq!(
        effective_move_cost(MOVE_COST_WALK, TERRAIN_IMPASSABLE),
        i32::MAX
    );
}

/// Crouch on normal terrain: still costs MOVE_COST_CROUCH.
#[test]
fn crouch_on_normal_terrain_costs_200() {
    assert_eq!(effective_move_cost(MOVE_COST_CROUCH, TERRAIN_NORMAL), 200);
}

/// Crouch on rough terrain: MOVE_COST_CROUCH * (terrain/100).
#[test]
fn crouch_on_rough_terrain_costs_400() {
    assert_eq!(effective_move_cost(MOVE_COST_CROUCH, TERRAIN_ROUGH), 400);
}

/// Prone on normal terrain.
#[test]
fn prone_on_normal_terrain_costs_600() {
    assert_eq!(effective_move_cost(MOVE_COST_PRONE, TERRAIN_NORMAL), 600);
}

/// Downed walk on normal terrain.
#[test]
fn downed_walk_on_normal_terrain_costs_300() {
    let downed_walk = MOVE_COST_WALK * MOVE_COST_DOWNED_MULTIPLIER;
    assert_eq!(effective_move_cost(downed_walk, TERRAIN_NORMAL), 300);
}

// ---------------------------------------------------------------------------
// TurnQueue integration — system-level tick
// ---------------------------------------------------------------------------

/// `tick_move_points` system grants AP to all alive actors.
/// CDDA: all actors gain their speed in move points at the start of each turn.
#[test]
fn tick_move_points_system_grants_ap_to_alive_actors() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    let _a = test.spawn((IsAlive, ActionPoints::default()));
    let _b = test.spawn((IsAlive, ActionPoints::default()));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors.len(), 2);
    for actor in &queue.actors {
        assert_eq!(
            actor.move_points, 100,
            "each actor should gain speed(100) AP"
        );
    }
}

/// Dead actors (no `IsAlive`) are not added to the turn queue.
/// CDDA: dead creatures do not act.
#[test]
fn dead_actors_excluded_from_turn_queue() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    test.spawn((ActionPoints::default(),)); // no IsAlive
    let alive = test.spawn((IsAlive, ActionPoints::default()));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors.len(), 1);
    assert_eq!(queue.actors[0].entity, alive);
}

/// Faster actors have higher MP in the queue and act first.
/// CDDA: higher-speed actors get more moves per turn.
#[test]
fn faster_actor_has_higher_mp_in_queue() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    let fast = test.spawn((IsAlive, ActionPoints::new(200)));
    let slow = test.spawn((IsAlive, ActionPoints::new(50)));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    let fast_mp = queue
        .actors
        .iter()
        .find(|a| a.entity == fast)
        .unwrap()
        .move_points;
    let slow_mp = queue
        .actors
        .iter()
        .find(|a| a.entity == slow)
        .unwrap()
        .move_points;
    assert!(
        fast_mp > slow_mp,
        "faster actor (200 speed) should have more AP than slow (50)"
    );
    assert_eq!(fast_mp, 200);
    assert_eq!(slow_mp, 50);
}

/// Def entities (IsDef marker) are excluded from the turn queue.
#[test]
fn def_entities_excluded_from_turn_queue() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.register::<IsDef>();
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    test.spawn((IsAlive, ActionPoints::default(), IsDef));
    let non_def = test.spawn((IsAlive, ActionPoints::default()));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors.len(), 1);
    assert_eq!(queue.actors[0].entity, non_def);
}

/// `tick_move_points` increments `GameTime.turn` each call.
#[test]
fn tick_advances_game_time() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());
    test.spawn((IsAlive, ActionPoints::default()));

    test.run_system(tick_move_points);
    assert_eq!(test.resource::<GameTime>().turn, 1);
    test.run_system(tick_move_points);
    assert_eq!(test.resource::<GameTime>().turn, 2);
}

// ---------------------------------------------------------------------------
// TurnQueue priority — highest MP acts first
// ---------------------------------------------------------------------------

/// CDDA: actors are ordered highest-move-points-first.
#[test]
fn turn_queue_pop_returns_highest_mp_first() {
    let mut queue = TurnQueue::default();
    let e1 = Entity::from_bits(1);
    let e2 = Entity::from_bits(2);
    let e3 = Entity::from_bits(3);

    queue.actors = vec![
        ActorTurn {
            move_points: 50,
            entity: e1,
        },
        ActorTurn {
            move_points: 100,
            entity: e2,
        },
        ActorTurn {
            move_points: 75,
            entity: e3,
        },
    ];

    assert_eq!(queue.pop_highest().unwrap().entity, e2); // 100 first
    assert_eq!(queue.pop_highest().unwrap().entity, e3); // then 75
    assert_eq!(queue.pop_highest().unwrap().entity, e1); // then 50
    assert!(queue.pop_highest().is_none());
}

/// `has_actors_ready` is true when any actor is at or above MP_MIN_FLOOR.
#[test]
fn queue_has_actors_ready_when_mp_above_floor() {
    let e = Entity::from_bits(1);
    let mut queue = TurnQueue::default();

    queue.actors = vec![ActorTurn {
        move_points: MP_MIN_FLOOR,
        entity: e,
    }];
    assert!(queue.has_actors_ready());

    queue.actors = vec![ActorTurn {
        move_points: MP_MIN_FLOOR - 1,
        entity: e,
    }];
    assert!(!queue.has_actors_ready());
}
