//! Temperature and spoilage tests — body temperature, wetness, spoilage rates,
//! and temperature-related health effects.
//!
//! Tests `BodyTemperature`, `Wetness`, `Spoilable`, `Sealed`, `PreservesTemp`,
//! and `Fireproof` components, along with helper functions that encode
//! CDDA-derived spoilage and temperature-damage formulas.

use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Temperature/spoilage formula helpers (pure functions, not in the library)
// ---------------------------------------------------------------------------

/// Calculate spoilage rate multiplier from ambient temperature in Celsius.
///
/// * `temp >= 30` → 2.0 (accelerated)
/// * `temp >= 10` → 1.0 (normal)
/// * `temp >= 0`  → 0.5 (refrigerated)
/// * `temp < 0`   → 0.0 (frozen — no spoilage)
fn spoilage_rate(temp_celsius: f64) -> f64 {
    if temp_celsius >= 30.0 {
        2.0
    } else if temp_celsius >= 10.0 {
        1.0
    } else if temp_celsius >= 0.0 {
        0.5
    } else {
        0.0
    }
}

/// Calculate health effect per tick from ambient temperature and wetness.
///
/// * `temp <= 0`   → cold damage (negative output)
/// * `temp > 40`   → heat damage (positive output)
/// * `15 < temp <= 35` → no damage (0)
/// * `0 < temp <= 15` and `35 < temp <= 40` → caller-defined boundary
///
/// Wetness multiplies cold damage by `1 + wetness / 100`.
fn temperature_health_effect(temp_celsius: f64, wetness: u32) -> i32 {
    if temp_celsius <= 0.0 {
        let base = -2;
        base * (1 + wetness as i32 / 100)
    } else if temp_celsius > 40.0 {
        3
    } else if temp_celsius > 15.0 && temp_celsius <= 35.0 {
        0
    } else if temp_celsius <= 15.0 && temp_celsius > 0.0 {
        // cool but not freezing: mild effect with wetness penalty
        -1 * (1 + wetness as i32 / 100)
    } else {
        // 35 < temp <= 40: warm but not dangerous
        0
    }
}

// ---------------------------------------------------------------------------
// BodyTemperature
// ---------------------------------------------------------------------------

#[test]
fn body_temperature_normal() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyTemperature>();

    let e = test.spawn((cdda_actor::components::BodyTemperature(36.5),));
    let temp = test
        .get::<cdda_actor::components::BodyTemperature>(e)
        .unwrap();
    assert!((temp.0 - 36.5).abs() < f64::EPSILON);
}

#[test]
fn body_temperature_hypothermia() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyTemperature>();

    let e = test.spawn((cdda_actor::components::BodyTemperature(30.0),));
    let temp = test
        .get::<cdda_actor::components::BodyTemperature>(e)
        .unwrap();
    assert!((temp.0 - 30.0).abs() < f64::EPSILON);
}

#[test]
fn body_temperature_hyperthermia() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyTemperature>();

    let e = test.spawn((cdda_actor::components::BodyTemperature(40.0),));
    let temp = test
        .get::<cdda_actor::components::BodyTemperature>(e)
        .unwrap();
    assert!((temp.0 - 40.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// Wetness
// ---------------------------------------------------------------------------

#[test]
fn wetness_dry() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Wetness>();

    let e = test.spawn((cdda_actor::components::Wetness(0),));
    let wet = test.get::<cdda_actor::components::Wetness>(e).unwrap();
    assert_eq!(wet.0, 0);
}

#[test]
fn wetness_wet() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Wetness>();

    let e = test.spawn((cdda_actor::components::Wetness(100),));
    let wet = test.get::<cdda_actor::components::Wetness>(e).unwrap();
    assert_eq!(wet.0, 100);
}

#[test]
fn wetness_soaked() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Wetness>();

    let e = test.spawn((cdda_actor::components::Wetness(500),));
    let wet = test.get::<cdda_actor::components::Wetness>(e).unwrap();
    assert_eq!(wet.0, 500);
}

// ---------------------------------------------------------------------------
// Temperature and wetness coexistence
// ---------------------------------------------------------------------------

#[test]
fn temperature_and_wetness_independent() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyTemperature>();
    test.register::<cdda_actor::components::Wetness>();

    let e = test.spawn((
        cdda_actor::components::BodyTemperature(36.5),
        cdda_actor::components::Wetness(100),
    ));
    assert!(test.get::<cdda_actor::components::BodyTemperature>(e).is_some());
    assert!(test.get::<cdda_actor::components::Wetness>(e).is_some());
}

// ---------------------------------------------------------------------------
// Spoilage rate
// ---------------------------------------------------------------------------

#[test]
fn temperature_affects_spoilage_rate() {
    // Hot: >= 30°C => 2.0x (accelerated)
    assert!((spoilage_rate(30.0) - 2.0).abs() < f64::EPSILON);
    assert!((spoilage_rate(35.0) - 2.0).abs() < f64::EPSILON);

    // Normal: >= 10°C => 1.0x
    assert!((spoilage_rate(10.0) - 1.0).abs() < f64::EPSILON);
    assert!((spoilage_rate(20.0) - 1.0).abs() < f64::EPSILON);

    // Cold: >= 0°C => 0.5x (refrigerated)
    assert!((spoilage_rate(0.0) - 0.5).abs() < f64::EPSILON);
    assert!((spoilage_rate(4.0) - 0.5).abs() < f64::EPSILON);

    // Freezing: < 0°C => 0.0x (no spoilage)
    assert!((spoilage_rate(-5.0) - 0.0).abs() < f64::EPSILON);
    assert!((spoilage_rate(-10.0) - 0.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// PreservesTemp and Sealed markers
// ---------------------------------------------------------------------------

#[test]
fn preserves_temp_marker() {
    let mut test = TestBed::new();
    test.register::<cdda_item::components::PreservesTemp>();

    let e = test.spawn((cdda_item::components::PreservesTemp,));
    assert!(test
        .world()
        .entity(e)
        .contains::<cdda_item::components::PreservesTemp>());
}

#[test]
fn sealed_and_preserves_temp() {
    let mut test = TestBed::new();
    test.register::<cdda_item::components::Sealed>();
    test.register::<cdda_item::components::PreservesTemp>();

    let e = test.spawn((
        cdda_item::components::Sealed,
        cdda_item::components::PreservesTemp,
    ));
    assert!(test
        .world()
        .entity(e)
        .contains::<cdda_item::components::Sealed>());
    assert!(test
        .world()
        .entity(e)
        .contains::<cdda_item::components::PreservesTemp>());
}

// ---------------------------------------------------------------------------
// Temperature health effects
// ---------------------------------------------------------------------------

#[test]
fn extreme_temperature_health_effect() {
    // 0°C, dry => -2 (cold damage)
    assert_eq!(temperature_health_effect(0.0, 0), -2);

    // 0°C, wet(100) => -4 (wetness doubles cold damage)
    assert_eq!(temperature_health_effect(0.0, 100), -4);

    // 50°C, dry => 3 (heat damage)
    assert_eq!(temperature_health_effect(50.0, 0), 3);

    // 25°C, dry => 0 (no damage)
    assert_eq!(temperature_health_effect(25.0, 0), 0);
}
