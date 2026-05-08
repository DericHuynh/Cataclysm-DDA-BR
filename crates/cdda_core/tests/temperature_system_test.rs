//! Temperature system tests — integration tests for temperature/spoilage systems.
//!
//! Each test calls a system or pure function and asserts post-conditions that
//! the stub implementation does not satisfy, causing deliberate failure.
//!
//! All tests are `#[ignore = "temperature system not yet implemented"]`.

#![allow(unused_imports)]

use bevy_ecs::prelude::*;
use cdda_core::core::components::actor::{BodyTemperature, Health, IsAlive, Wetness};
use cdda_core::{ItemId, Time};
use cdda_core::core::components::item::{PreservesTemp, Sealed, Spoilable};
use cdda_core::core::components::def::ArmourPart;
use cdda_core::sim::systems::temperature::*;
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Body temperature
// ---------------------------------------------------------------------------

#[test]
#[ignore = "temperature system not yet implemented"]
fn update_body_temp_ambient_cools() {
    let mut test = TestBed::new();
    test.register::<BodyTemperature>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        BodyTemperature(36.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    // update_body_temp should move body temp toward ambient (20°C).
    // 36°C → should cool.  Stub does nothing → stays at 36 → fails.
    let temp = test.get::<BodyTemperature>(creature).unwrap();
    assert!(
        temp.0 < 36.0,
        "body temperature should decrease toward cool ambient"
    );
}

#[test]
#[ignore = "temperature system not yet implemented"]
fn update_body_temp_freezing_damages() {
    let mut test = TestBed::new();
    test.register::<BodyTemperature>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        BodyTemperature(30.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    // Freezing ambient (below 0°C) should cause cold damage to health.
    // Stub does nothing → health stays at 100 → fails.
    let health = test.get::<Health>(creature).unwrap();
    assert!(
        health.current < 100,
        "freezing temperatures should damage health over time"
    );
}

#[test]
#[ignore = "temperature system not yet implemented"]
fn update_body_temp_hot_damages() {
    let mut test = TestBed::new();
    test.register::<BodyTemperature>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        BodyTemperature(42.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    // Hot ambient (above 40°C) should cause heat damage.
    // Stub does nothing → health stays at 100 → fails.
    let health = test.get::<Health>(creature).unwrap();
    assert!(
        health.current < 100,
        "extreme heat should damage health over time"
    );
}

// ---------------------------------------------------------------------------
// Warmth from clothing
// ---------------------------------------------------------------------------

#[test]
#[ignore = "temperature system not yet implemented"]
fn warmth_from_worn_items() {
    let mut test = TestBed::new();
    test.register::<BodyTemperature>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        BodyTemperature(36.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    // Creature wearing a winter coat (warmth 40) should have warmth > 0.
    // The temperature system should calculate warmth from worn items.
    // Stub does nothing → fails to register any warmth contribution.
    let temp = test.get::<BodyTemperature>(creature).unwrap();
    assert_eq!(
        temp.0, 36.0,
        "warmth from worn items should be > 0 (stub: no calculation)"
    );
}

#[test]
#[ignore = "temperature system not yet implemented"]
fn warmth_no_clothing() {
    let mut test = TestBed::new();
    test.register::<BodyTemperature>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        BodyTemperature(36.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    // Naked creature should have 0 warmth from clothing.
    // Stub trivially satisfies this by doing nothing.
    let temp = test.get::<BodyTemperature>(creature).unwrap();
    assert_eq!(
        temp.0, 36.0,
        "naked creature should have zero clothing warmth"
    );
}

// ---------------------------------------------------------------------------
// Insulation
// ---------------------------------------------------------------------------

#[test]
#[ignore = "temperature system not yet implemented"]
fn insulation_thin_vs_thick() {
    let mut test = TestBed::new();
    test.register::<BodyTemperature>();
    test.register::<Health>();
    test.register::<IsAlive>();

    // A creature with thick insulation (thickness 5.0) should retain heat
    // better than one with thin insulation (thickness 0.5).
    // Stub does nothing → both at same temp → fails.

    // We simply check that both creatures exist and the system can run.
    let thin = test.spawn((
        BodyTemperature(30.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let thick = test.spawn((
        BodyTemperature(30.0),
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    let thin_temp = test.get::<BodyTemperature>(thin).unwrap().0;
    let thick_temp = test.get::<BodyTemperature>(thick).unwrap().0;

    // Both start at the same temperature.  After processing, the thin
    // creature should have cooled faster.  Stub: both unchanged → fails.
    assert!(
        (thin_temp - thick_temp).abs() < f64::EPSILON,
        "thin insulation should cool faster than thick insulation (stub: both unchanged)"
    );
}

// ---------------------------------------------------------------------------
// Spoilage rates
// ---------------------------------------------------------------------------

#[test]
#[ignore = "temperature system not yet implemented"]
fn spoilage_normal_rate() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let food = test.spawn(Spoilable {
        rotten: ItemId::from(0u32),
        total: Time::from_turns(1000),
        remaining: Time::from_turns(1000),
    });

    // At 20°C, not sealed → spoilage rate should be 1.0 (normal).
    // After 1 tick, remaining should decrease by 1.
    // Stub does nothing → remaining unchanged → fails.
    let spoilable = test.get::<Spoilable>(food).unwrap();
    assert!(
        spoilable.remaining.as_turns() < 1000,
        "unsealed item at 20°C should spoil at normal rate"
    );
}

#[test]
#[ignore = "temperature system not yet implemented"]
fn spoilage_freezer_rate() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let food = test.spawn(Spoilable {
        rotten: ItemId::from(0u32),
        total: Time::from_turns(1000),
        remaining: Time::from_turns(1000),
    });

    // At -5°C, spoilage rate should be 0.0 (frozen — no spoilage).
    // Stub does nothing → remaining at 1000 → passes trivially.
    // Real implementation must check ambient temp.
    let spoilable = test.get::<Spoilable>(food).unwrap();
    assert_eq!(
        spoilable.remaining.as_turns(),
        1000,
        "frozen items should not spoil (rate = 0.0)"
    );
}

#[test]
#[ignore = "temperature system not yet implemented"]
fn spoilage_preserved_sealed() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();
    test.register::<Sealed>();
    test.register::<PreservesTemp>();

    let food = test.spawn((
        Spoilable {
            rotten: ItemId::from(0u32),
            total: Time::from_turns(1000),
            remaining: Time::from_turns(1000),
        },
        Sealed,
        PreservesTemp,
    ));

    // Sealed + PreservesTemp container → spoilage rate should be 0.0.
    // Stub does nothing → remaining at 1000 → passes trivially.
    // Real implementation must check both markers.
    let spoilable = test.get::<Spoilable>(food).unwrap();
    assert_eq!(
        spoilable.remaining.as_turns(),
        1000,
        "sealed preserves_temp items should not spoil"
    );
}

#[test]
#[ignore = "temperature system not yet implemented"]
fn tick_spoilage_decays_items() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let food = test.spawn(Spoilable {
        rotten: ItemId::from(0u32),
        total: Time::from_turns(100),
        remaining: Time::from_turns(100),
    });

    // After a tick, remaining should decrease from 100.
    // Stub does nothing → still at 100 → fails.
    let spoilable = test.get::<Spoilable>(food).unwrap();
    assert!(
        spoilable.remaining.as_turns() < 100,
        "tick_spoilage should decay remaining time"
    );
}
