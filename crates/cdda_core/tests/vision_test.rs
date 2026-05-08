//! Vision tests — translated from CDDA's vision testing patterns.
//!
//! Tests vision components and CDDA-derived vision range formulas.

use cdda_core::core::components::actor::{IsAlive, Vision};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Pure helper functions — CDDA-derived vision formulas
// ---------------------------------------------------------------------------

/// CDDA-style vision range calculation.
/// At daytime, returns day_range. At nighttime, returns night_range.
/// Dusk/dawn returns the average.
fn effective_vision_range(vision: &Vision, time_of_day: &str) -> i32 {
    match time_of_day {
        "day" => vision.day_range,
        "night" => vision.night_range,
        "dusk" | "dawn" => (vision.day_range + vision.night_range) / 2,
        _ => vision.day_range, // default to day
    }
}

/// Sight distance penalty per point of light level below threshold.
/// Below 10 light level, each missing point reduces range by 2.
fn light_level_penalty(light_level: u32) -> i32 {
    if light_level >= 10 {
        0
    } else {
        (10 - light_level) as i32 * 2
    }
}

/// Final vision range after applying light penalty.
fn final_vision_range(vision: &Vision, time_of_day: &str, light_level: u32) -> i32 {
    let base = effective_vision_range(vision, time_of_day);
    let penalty = light_level_penalty(light_level);
    (base - penalty).max(0)
}

// ---------------------------------------------------------------------------
// Basic component tests
// ---------------------------------------------------------------------------

#[test]
fn vision_normal() {
    let mut test = TestBed::new();
    test.register::<Vision>();
    test.register::<IsAlive>();

    let e = test.spawn((
        Vision {
            day_range: 40,
            night_range: 5,
        },
        IsAlive,
    ));
    let vision = test.get::<Vision>(e).unwrap();
    assert_eq!(vision.day_range, 40);
    assert_eq!(vision.night_range, 5);
}

#[test]
fn vision_no_night_vision() {
    let mut test = TestBed::new();
    test.register::<Vision>();
    test.register::<IsAlive>();

    let e = test.spawn((
        Vision {
            day_range: 40,
            night_range: 0,
        },
        IsAlive,
    ));
    let vision = test.get::<Vision>(e).unwrap();
    assert_eq!(vision.day_range, 40);
    assert_eq!(vision.night_range, 0);
}

#[test]
fn vision_both_zero() {
    let mut test = TestBed::new();
    test.register::<Vision>();
    test.register::<IsAlive>();

    let e = test.spawn((
        Vision {
            day_range: 0,
            night_range: 0,
        },
        IsAlive,
    ));
    let vision = test.get::<Vision>(e).unwrap();
    assert_eq!(vision.day_range, 0);
    assert_eq!(vision.night_range, 0);
}

#[test]
fn vision_high_day() {
    let mut test = TestBed::new();
    test.register::<Vision>();
    test.register::<IsAlive>();

    let e = test.spawn((
        Vision {
            day_range: 60,
            night_range: 10,
        },
        IsAlive,
    ));
    let vision = test.get::<Vision>(e).unwrap();
    assert_eq!(vision.day_range, 60);
    assert_eq!(vision.night_range, 10);
}

#[test]
fn vision_excellent_night() {
    let mut test = TestBed::new();
    test.register::<Vision>();
    test.register::<IsAlive>();

    let e = test.spawn((
        Vision {
            day_range: 50,
            night_range: 40,
        },
        IsAlive,
    ));
    let vision = test.get::<Vision>(e).unwrap();
    assert_eq!(vision.day_range, 50);
    assert_eq!(vision.night_range, 40);
}

#[test]
fn vision_negative() {
    let mut test = TestBed::new();
    test.register::<Vision>();
    test.register::<IsAlive>();

    let e = test.spawn((
        Vision {
            day_range: -5,
            night_range: -10,
        },
        IsAlive,
    ));
    let vision = test.get::<Vision>(e).unwrap();
    assert_eq!(vision.day_range, -5);
    assert_eq!(vision.night_range, -10);
}

// ---------------------------------------------------------------------------
// Pure function formula tests
// ---------------------------------------------------------------------------

#[test]
fn effective_range_day() {
    let vision = Vision {
        day_range: 40,
        night_range: 5,
    };
    assert_eq!(effective_vision_range(&vision, "day"), 40);
}

#[test]
fn effective_range_night() {
    let vision = Vision {
        day_range: 40,
        night_range: 5,
    };
    assert_eq!(effective_vision_range(&vision, "night"), 5);
}

#[test]
fn effective_range_dusk() {
    let vision = Vision {
        day_range: 40,
        night_range: 10,
    };
    assert_eq!(effective_vision_range(&vision, "dusk"), 25);
    assert_eq!(effective_vision_range(&vision, "dawn"), 25);
}

#[test]
fn light_penalty_none() {
    assert_eq!(light_level_penalty(10), 0);
    assert_eq!(light_level_penalty(15), 0);
    assert_eq!(light_level_penalty(100), 0);
}

#[test]
fn light_penalty_dark() {
    assert_eq!(light_level_penalty(0), 20);
    assert_eq!(light_level_penalty(5), 10);
    assert_eq!(light_level_penalty(9), 2);
}

#[test]
fn final_range_clamped() {
    let vision = Vision {
        day_range: 10,
        night_range: 2,
    };
    // Very dark room at night: base=2, penalty=20 → -18, clamped to 0
    assert_eq!(final_vision_range(&vision, "night", 0), 0);
    // Very dark room at day: base=10, penalty=20 → -10, clamped to 0
    assert_eq!(final_vision_range(&vision, "day", 0), 0);
    // Dimly lit room at day: base=10, penalty=2 → 8
    assert_eq!(final_vision_range(&vision, "day", 9), 8);
}
