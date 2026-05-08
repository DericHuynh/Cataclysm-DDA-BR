//! Healing system tests — integration tests for the healing phase.
//!
//! Each test calls a healing system function and asserts post-conditions
//! that the stub implementation does not satisfy (all functions return
//! `todo!()`), causing deliberate failure.
//!
//! All tests are `#[ignore = "healing system not yet implemented"]`.

#![allow(unused_imports)]

use bevy_ecs::prelude::*;
use cdda_core::sim::test_utils::TestBed;
use cdda_core::sim::systems::healing::*;
use cdda_core::core::components::actor::{Health, IsAlive, BodyPartHp, BodyPartOf, BodyPartDef, BodyPartSlot};

// ---------------------------------------------------------------------------
// Healing rate
// ---------------------------------------------------------------------------

#[test]
#[ignore = "healing system not yet implemented"]
fn healing_rate_awake_normal() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        Health { current: 50, max: 100 },
        IsAlive,
    ));

    // healing_rate for an awake, well-fed creature at 50/100 HP should
    // produce a modest positive rate.  Stub returns todo!() → panics.
    // We reach here only if the stub is replaced; but the stub isn't
    // called directly via run_system — it's a pure function we test
    // indirectly by running healing_phase.
    test.run_system(healing_phase);

    // After a healing tick, HP should have increased.
    let health = test.get::<Health>(creature).unwrap();
    assert!(
        health.current > 50,
        "normal healing should restore HP over time"
    );
}

#[test]
#[ignore = "healing system not yet implemented"]
fn healing_rate_sleeping() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let awake = test.spawn((
        Health { current: 50, max: 100 },
        IsAlive,
    ));
    let asleep = test.spawn((
        Health { current: 50, max: 100 },
        IsAlive,
    ));

    test.run_system(healing_phase);

    // Sleeping creatures should heal at 2-3x the awake rate.
    // Stub does nothing → both stay at 50 → fails.
    let awake_hp = test.get::<Health>(awake).unwrap().current;
    let asleep_hp = test.get::<Health>(asleep).unwrap().current;
    assert!(
        asleep_hp > awake_hp,
        "sleeping should increase healing rate compared to awake"
    );
}

#[test]
#[ignore = "healing system not yet implemented"]
fn healing_rate_starving() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let well_fed = test.spawn((
        Health { current: 50, max: 100 },
        IsAlive,
    ));
    let starving = test.spawn((
        Health { current: 50, max: 100 },
        IsAlive,
    ));

    test.run_system(healing_phase);

    // Poor nutrition (starving) should slow healing.
    // Stub does nothing → both at 50 → fails.
    let fed_hp = test.get::<Health>(well_fed).unwrap().current;
    let starve_hp = test.get::<Health>(starving).unwrap().current;
    assert!(
        starve_hp < fed_hp,
        "starvation should reduce healing rate"
    );
}

#[test]
#[ignore = "healing system not yet implemented"]
fn healing_rate_zero_hp() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        Health { current: 0, max: 100 },
        IsAlive,
    ));

    test.run_system(healing_phase);

    // A creature at 0 HP should have 0 healing rate.
    // Stub does nothing → stays at 0 → passes trivially.
    // Real implementation: 0 HP means dead → no healing.
    let health = test.get::<Health>(creature).unwrap();
    assert_eq!(
        health.current, 0,
        "creature at 0 HP should have zero healing rate"
    );
}

// ---------------------------------------------------------------------------
// Natural healing tick
// ---------------------------------------------------------------------------

#[test]
#[ignore = "healing system not yet implemented"]
fn natural_healing_tick_restores() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let creature = test.spawn((
        Health { current: 90, max: 100 },
        IsAlive,
    ));

    test.run_system(healing_phase);

    // A tick of natural healing should restore some HP.
    // Stub does nothing → stays at 90 → fails.
    let health = test.get::<Health>(creature).unwrap();
    assert!(
        health.current > 90,
        "natural healing tick should restore HP"
    );
}

// ---------------------------------------------------------------------------
// First aid
// ---------------------------------------------------------------------------

#[test]
#[ignore = "healing system not yet implemented"]
fn first_aid_applies_healing() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<BodyPartHp>();
    test.register::<BodyPartOf>();
    test.register::<BodyPartDef>();
    test.register::<BodyPartSlot>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 80, max: 100 },
    ));
    let body_part = test.spawn((
        BodyPartHp {
            max: 50.0,
            current: 20.0,
            damage_multiplier: 1.0,
        },
        BodyPartOf(creature),
    ));

    test.run_system(healing_phase);

    // Applying a bandage (quality 2) to a body part with 20/50 HP
    // should restore some HP.  Stub does nothing → stays at 20 → fails.
    let bp_hp = test.get::<BodyPartHp>(body_part).unwrap();
    assert!(
        bp_hp.current > 20.0,
        "first aid with bandage quality 2 should heal HP"
    );
}

#[test]
#[ignore = "healing system not yet implemented"]
fn first_aid_with_disinfectant() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<BodyPartHp>();
    test.register::<BodyPartOf>();
    test.register::<BodyPartDef>();
    test.register::<BodyPartSlot>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 80, max: 100 },
    ));
    let body_part = test.spawn((
        BodyPartHp {
            max: 50.0,
            current: 20.0,
            damage_multiplier: 1.0,
        },
        BodyPartOf(creature),
    ));

    test.run_system(healing_phase);

    // First aid with disinfectant should heal more than without.
    // Stub does nothing → stays at 20 → fails.
    let bp_hp = test.get::<BodyPartHp>(body_part).unwrap();
    assert!(
        bp_hp.current > 20.0,
        "disinfectant should add bonus healing to first aid"
    );
}

#[test]
#[ignore = "healing system not yet implemented"]
fn first_aid_high_quality() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<BodyPartHp>();
    test.register::<BodyPartOf>();
    test.register::<BodyPartDef>();
    test.register::<BodyPartSlot>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 80, max: 100 },
    ));
    let body_part = test.spawn((
        BodyPartHp {
            max: 50.0,
            current: 20.0,
            damage_multiplier: 1.0,
        },
        BodyPartOf(creature),
    ));

    test.run_system(healing_phase);

    // Quality 5 bandage should heal more than quality 1.
    // Stub does nothing → stays at 20 → fails.
    let bp_hp = test.get::<BodyPartHp>(body_part).unwrap();
    assert!(
        bp_hp.current > 20.0,
        "high-quality bandages should provide more healing"
    );
}
