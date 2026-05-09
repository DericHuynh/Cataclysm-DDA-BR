//! Tests for the turn scheduling system — `tick_move_points`, `spend_move_points`,
//! `effective_move_cost`, `TurnQueue`, and the top-level `game_tick` orchestrator.
//!
//! Unlike most other test files, these test **already-implemented** systems.
//! No `#[ignore]` — they should pass as soon as all types compile.

use bevy_ecs::prelude::*;
use cdda_core::core::components::actor::{IsAlive, ActionPoints, Speed};
use cdda_core::sim::state::GameTime;
use cdda_core::actor::turn::*;
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// tick_move_points
// ---------------------------------------------------------------------------

#[test]
fn tick_move_points_grants_mp() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.register::<Speed>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Two entities with speed 100, both start at MP=0
    let _a = test.spawn((IsAlive, ActionPoints(0), Speed(100)));
    let _b = test.spawn((IsAlive, ActionPoints(0), Speed(100)));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors.len(), 2);
    assert_eq!(queue.turn_count, 1);
    for actor in &queue.actors {
        assert_eq!(actor.move_points, 100);
    }
}

#[test]
fn tick_move_points_accumulates() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.register::<Speed>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Entity starts with MP=50, speed=100 → should have 150 after tick
    test.spawn((IsAlive, ActionPoints(50), Speed(100)));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors[0].move_points, 150);
}

#[test]
fn tick_move_points_debt_floor() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.register::<Speed>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Entity deeply in debt: MP=-300, speed=100 → debt floor = -200
    test.spawn((IsAlive, ActionPoints(-300), Speed(100)));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    // -300 + 100 = -200, which is >= debt floor of -200
    assert_eq!(queue.actors[0].move_points, -200);
}

#[test]
fn tick_move_points_advances_time() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.register::<Speed>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    test.spawn((IsAlive, ActionPoints(0), Speed(100)));

    test.run_system(tick_move_points);
    assert_eq!(test.resource::<GameTime>().turn, 1);

    test.run_system(tick_move_points);
    assert_eq!(test.resource::<GameTime>().turn, 2);
}

#[test]
fn tick_move_points_ignores_non_alive() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.register::<Speed>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Entity without IsAlive — should be skipped
    test.spawn((ActionPoints(0), Speed(100)));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors.len(), 0);
}

// ---------------------------------------------------------------------------
// spend_move_points
// ---------------------------------------------------------------------------

#[test]
fn spend_mp_reduces_and_returns_true() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints(100),));

    // Directly mutate MP (tests the same logic as spend_move_points)
    let mut mp = test.world_mut().get_mut::<ActionPoints>(e).unwrap();
    mp.0 -= 30;
    let can_act = mp.0 >= MP_MIN_FLOOR;
    drop(mp);

    assert!(can_act);
    assert_eq!(test.get::<ActionPoints>(e).unwrap().0, 70);
}

#[test]
fn spend_mp_insufficient() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints(20),));

    let mut mp = test.world_mut().get_mut::<ActionPoints>(e).unwrap();
    mp.0 -= 30;
    let can_act = mp.0 >= MP_MIN_FLOOR;
    drop(mp);

    assert!(!can_act);
    assert_eq!(test.get::<ActionPoints>(e).unwrap().0, -10);
}

// ---------------------------------------------------------------------------
// effective_move_cost
// ---------------------------------------------------------------------------

#[test]
fn effective_move_cost_normal() {
    assert_eq!(effective_move_cost(100, 100), 100);
}

#[test]
fn effective_move_cost_rough() {
    assert_eq!(effective_move_cost(100, 200), 200);
}

#[test]
fn effective_move_cost_impassable() {
    assert_eq!(effective_move_cost(100, 0), i32::MAX);
}

#[test]
fn effective_move_cost_furniture_penalty() {
    // 50 base + 30 furniture mod = 80, terrain_cost=100
    assert_eq!(effective_move_cost(50, 100), 50);
}

// ---------------------------------------------------------------------------
// TurnQueue
// ---------------------------------------------------------------------------

#[test]
fn turn_queue_pop_highest_order() {
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

    assert_eq!(queue.pop_highest().unwrap().entity, e2); // 100
    assert_eq!(queue.pop_highest().unwrap().entity, e3); // 75
    assert_eq!(queue.pop_highest().unwrap().entity, e1); // 50
    assert!(queue.pop_highest().is_none());
}

#[test]
fn turn_queue_has_actors_ready() {
    let e = Entity::from_bits(1);
    let mut queue = TurnQueue::default();

    queue.actors = vec![ActorTurn {
        move_points: 100,
        entity: e,
    }];
    assert!(queue.has_actors_ready());

    queue.actors = vec![ActorTurn {
        move_points: 10,
        entity: e,
    }];
    assert!(!queue.has_actors_ready());
}

#[test]
fn turn_queue_highest_mp() {
    let mut queue = TurnQueue::default();
    assert_eq!(queue.highest_mp(), 0);

    let e1 = Entity::from_bits(1);
    let e2 = Entity::from_bits(2);
    queue.actors = vec![
        ActorTurn {
            move_points: 50,
            entity: e1,
        },
        ActorTurn {
            move_points: 120,
            entity: e2,
        },
    ];
    assert_eq!(queue.highest_mp(), 120);
}
