//! Movement system tests — integration tests for the movement phase.
//!
//! All tests are `#[ignore]` because the movement system is not yet implemented.

#![allow(unused_imports)]

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, Bleeding, IsAlive, Stunned};
use cdda_components::def::TerrainMoveCost;
use cdda_components::sim::{Solid, WorldPosition};
use cdda_components::WorldPos;
use cdda_sim::actor::movement::*;
use cdda_sim::runtime::test_utils::TestBed;

#[test]
#[ignore = "movement system not yet implemented"]
fn move_cost_flat_terrain() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();

    let mover = test.spawn((
        ActionPoints {
            current: 100,
            speed: 100,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(mover).unwrap();
    assert_eq!(
        ap.current, 0,
        "moving onto flat terrain should consume 100 AP"
    );
}

#[test]
#[ignore = "movement system not yet implemented"]
fn move_cost_rough_terrain() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();

    let mover = test.spawn((
        ActionPoints {
            current: 200,
            speed: 100,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(mover).unwrap();
    assert_eq!(
        ap.current, 0,
        "moving onto rough terrain should consume 200 AP"
    );
}

#[test]
#[ignore = "movement system not yet implemented"]
fn move_cost_impassable() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();

    let mover = test.spawn((
        ActionPoints {
            current: 100,
            speed: 100,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(mover).unwrap();
    assert_eq!(
        ap.current, 100,
        "impassable terrain should block movement, AP unspent"
    );
}

#[test]
#[ignore = "movement system not yet implemented"]
fn move_cost_speed_modifies() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();

    let fast = test.spawn((
        ActionPoints {
            current: 200,
            speed: 200,
        },
        IsAlive,
    ));
    let slow = test.spawn((
        ActionPoints {
            current: 200,
            speed: 50,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let fast_ap = test.get::<ActionPoints>(fast).unwrap().current;
    let slow_ap = test.get::<ActionPoints>(slow).unwrap().current;
    assert!(
        fast_ap > slow_ap,
        "higher speed should reduce effective move cost"
    );
}

#[test]
#[ignore = "movement system not yet implemented"]
fn move_cost_bleeding_penalty() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();
    test.register::<Bleeding>();

    let bleeding = test.spawn((
        ActionPoints {
            current: 125,
            speed: 100,
        },
        IsAlive,
        Bleeding,
    ));
    let healthy = test.spawn((
        ActionPoints {
            current: 100,
            speed: 100,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let bleed_ap = test.get::<ActionPoints>(bleeding).unwrap().current;
    let healthy_ap = test.get::<ActionPoints>(healthy).unwrap().current;
    assert!(
        bleed_ap < healthy_ap,
        "bleeding should add 25% to movement cost"
    );
}

#[test]
#[ignore = "movement system not yet implemented"]
fn move_cost_swimming_penalty() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();

    let swimmer = test.spawn((
        ActionPoints {
            current: 200,
            speed: 100,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(swimmer).unwrap();
    assert_eq!(ap.current, 0, "swimming should double the move cost");
}

#[test]
#[ignore = "movement system not yet implemented"]
fn spend_ap_reduces() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints {
        current: 100,
        speed: 100,
    },));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(e).unwrap();
    assert_eq!(ap.current, 70, "spending 30 AP should leave 70 remaining");
}

#[test]
#[ignore = "movement system not yet implemented"]
fn spend_ap_insufficient() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();

    let e = test.spawn((ActionPoints {
        current: 20,
        speed: 100,
    },));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(e).unwrap();
    assert_eq!(ap.current, 20, "insufficient AP should prevent movement");
}

#[test]
#[ignore = "movement system not yet implemented"]
fn gain_ap_at_start_of_turn() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();

    let e = test.spawn((
        ActionPoints {
            current: 0,
            speed: 100,
        },
        IsAlive,
    ));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(e).unwrap();
    assert_eq!(
        ap.current, 100,
        "start-of-turn AP gain should add speed value"
    );
}

#[test]
#[ignore = "movement system not yet implemented"]
fn is_passable_impassable_terrain() {
    let mut test = TestBed::new();
    test.register::<TerrainMoveCost>();
    test.register::<WorldPosition>();

    let impassable = test.spawn((
        TerrainMoveCost(0),
        WorldPosition(WorldPos::new(
            0,
            0,
            cdda_components::core::coords::ZLevel::new(0),
        )),
    ));
    test.run_system(movement_phase);

    assert!(test.get::<TerrainMoveCost>(impassable).is_some());
}

#[test]
#[ignore = "movement system not yet implemented"]
fn is_passable_normal_terrain() {
    let mut test = TestBed::new();
    test.register::<TerrainMoveCost>();
    test.register::<WorldPosition>();

    let passable = test.spawn((
        TerrainMoveCost(100),
        WorldPosition(WorldPos::new(
            0,
            0,
            cdda_components::core::coords::ZLevel::new(0),
        )),
    ));
    test.run_system(movement_phase);

    let cost = test.get::<TerrainMoveCost>(passable).unwrap();
    assert_eq!(cost.0, 100);
}

#[test]
#[ignore = "movement system not yet implemented"]
fn attempt_move_into_solid() {
    let mut test = TestBed::new();
    test.register::<ActionPoints>();
    test.register::<IsAlive>();
    test.register::<WorldPosition>();
    test.register::<Solid>();

    let _wall = test.spawn((
        Solid,
        WorldPosition(WorldPos::new(
            1,
            0,
            cdda_components::core::coords::ZLevel::new(0),
        )),
    ));
    let mover = test.spawn((
        ActionPoints {
            current: 100,
            speed: 100,
        },
        IsAlive,
        WorldPosition(WorldPos::new(
            0,
            0,
            cdda_components::core::coords::ZLevel::new(0),
        )),
    ));
    test.run_system(movement_phase);

    let ap = test.get::<ActionPoints>(mover).unwrap();
    assert_eq!(
        ap.current, 100,
        "moving into solid should be blocked, AP unspent"
    );
}
