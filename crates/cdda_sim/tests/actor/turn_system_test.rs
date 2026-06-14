//! Tests for the turn scheduling system — `tick_move_points`, `spend_move_points`,
//! `effective_move_cost`, `TurnQueue`, and the top-level `game_tick` orchestrator.

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_sim::actor::turn::*;
use cdda_sim::runtime::state::GameTime;
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// tick_move_points
// ---------------------------------------------------------------------------

#[test]
fn tick_move_points_grants_mp() {
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
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Entity starts with current=50, speed=100 → should have 150 after tick
    test.spawn((
        IsAlive,
        ActionPoints {
            current: 50,
            speed: 100,
        },
    ));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors[0].move_points, 150);
}

#[test]
fn tick_move_points_debt_floor() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.add_message::<cdda_components::messages::TurnAdvanced>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Entity deeply in debt: current=-300, speed=100 → debt floor = -200
    test.spawn((
        IsAlive,
        ActionPoints {
            current: -300,
            speed: 100,
        },
    ));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    // -300 + 100 = -200, which equals the debt floor of -200
    assert_eq!(queue.actors[0].move_points, -200);
}

#[test]
fn tick_move_points_advances_time() {
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

#[test]
fn tick_move_points_ignores_non_alive() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<ActionPoints>();
    test.insert_resource(TurnQueue::default());
    test.insert_resource(GameTime::default());

    // Entity without IsAlive — should be skipped
    test.spawn((ActionPoints::default(),));

    test.run_system(tick_move_points);

    let queue = test.resource::<TurnQueue>();
    assert_eq!(queue.actors.len(), 0);
}

// ---------------------------------------------------------------------------
// spend_move_points (via direct mutation)
// ---------------------------------------------------------------------------

#[test]
fn spend_mp_reduces_and_returns_true() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints {
        current: 100,
        speed: 100,
    },));

    let mut ap = test.world_mut().get_mut::<ActionPoints>(e).unwrap();
    ap.spend(30);
    let can_act = ap.current >= MP_MIN_FLOOR;
    drop(ap);

    assert!(can_act);
    assert_eq!(test.get::<ActionPoints>(e).unwrap().current, 70);
}

#[test]
fn spend_mp_insufficient() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints {
        current: 20,
        speed: 100,
    },));

    let mut ap = test.world_mut().get_mut::<ActionPoints>(e).unwrap();
    ap.spend(30);
    let can_act = ap.current >= MP_MIN_FLOOR;
    drop(ap);

    assert!(!can_act);
    assert_eq!(test.get::<ActionPoints>(e).unwrap().current, -10);
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

    assert_eq!(queue.pop_highest().unwrap().entity, e2);
    assert_eq!(queue.pop_highest().unwrap().entity, e3);
    assert_eq!(queue.pop_highest().unwrap().entity, e1);
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
