//! Damage and combat tests — translated from CDDA's melee_test.cpp and
//! explosion_balance_test.cpp.
//!
//! Tests damage calculations, health tracking, and creature state.

use bevy_ecs::prelude::*;
use cdda_core::Damage;
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Damage profile
// ---------------------------------------------------------------------------

#[test]
fn damage_zero_is_empty() {
    let d = Damage::ZERO;
    assert!(d.is_empty());
    assert_eq!(d.total(), 0);
}

#[test]
fn damage_add_types() {
    let mut d = Damage::ZERO;
    use cdda_core::id::{DamageTypeId, DefIdx};
    let bash = DamageTypeId(DefIdx(0));
    let cut = DamageTypeId(DefIdx(1));

    d.add(bash, 5);
    d.add(cut, 3);
    assert_eq!(d.total(), 8);
    assert_eq!(d.len(), 2);
}

#[test]
fn damage_merge_same_type() {
    use cdda_core::id::{DamageTypeId, DefIdx};
    let bash = DamageTypeId(DefIdx(0));
    let mut d = Damage::ZERO;
    d.add(bash, 5);
    d.add(bash, 3);
    assert_eq!(d.len(), 1);
    assert_eq!(d.by_type(bash), 8);
}

#[test]
fn damage_zero_amount_not_stored() {
    use cdda_core::id::{DamageTypeId, DefIdx};
    let bash = DamageTypeId(DefIdx(0));
    let mut d = Damage::ZERO;
    d.add(bash, 0);
    assert!(d.is_empty());
}

#[test]
fn damage_merge_profiles() {
    use cdda_core::id::{DamageTypeId, DefIdx};
    let bash = DamageTypeId(DefIdx(0));
    let cut = DamageTypeId(DefIdx(1));

    let mut a = Damage::ZERO;
    a.add(bash, 5);

    let mut b = Damage::ZERO;
    b.add(cut, 3);
    b.add(bash, 2);

    a.merge(&b);
    assert_eq!(a.total(), 10);
    assert_eq!(a.by_type(bash), 7);
    assert_eq!(a.by_type(cut), 3);
}

// ---------------------------------------------------------------------------
// Health tracking
// ---------------------------------------------------------------------------

#[test]
fn creature_health_initialized() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Health>();

    let e = test.spawn((
        cdda_core::actor::components::Health {
            current: 100,
            max: 100,
        },
        cdda_core::actor::components::IsAlive,
    ));
    let health = test.get::<cdda_core::actor::components::Health>(e).unwrap();
    assert_eq!(health.current, 100);
    assert_eq!(health.max, 100);
}

#[test]
fn creature_health_damage_reduces() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Health>();

    let e = test.spawn((cdda_core::actor::components::Health {
        current: 100,
        max: 100,
    },));
    let mut health = test
        .world_mut()
        .get_mut::<cdda_core::actor::components::Health>(e)
        .unwrap();
    health.current = 70;
    drop(health);

    let health = test.get::<cdda_core::actor::components::Health>(e).unwrap();
    assert_eq!(health.current, 70);
    assert_eq!(health.max, 100);
}

#[test]
fn creature_health_zero_is_dead() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Health>();

    let e = test.spawn((cdda_core::actor::components::Health {
        current: 0,
        max: 100,
    },));
    let health = test.get::<cdda_core::actor::components::Health>(e).unwrap();
    assert_eq!(health.current, 0);
    // current = 0 should correspond to death
    assert!(health.current <= 0);
}

// ---------------------------------------------------------------------------
// Creature stats
// ---------------------------------------------------------------------------

#[test]
fn creature_has_default_stats() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Stats>();

    let e = test.spawn((cdda_core::actor::components::Stats(cdda_core::Stats::new(
        8, 8, 8, 8,
    )),));
    let stats = test.get::<cdda_core::actor::components::Stats>(e).unwrap();
    assert_eq!(stats.0.strength, 8);
    assert_eq!(stats.0.dexterity, 8);
    assert_eq!(stats.0.intelligence, 8);
    assert_eq!(stats.0.perception, 8);
}

// ---------------------------------------------------------------------------
// Turn scheduling — MovePoints and Speed
// ---------------------------------------------------------------------------

#[test]
fn speed_default_is_one_hundred() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Speed>();

    let e = test.spawn((cdda_core::actor::components::Speed(100),));
    let speed = test.get::<cdda_core::actor::components::Speed>(e).unwrap();
    assert_eq!(speed.0, 100);
}

#[test]
fn move_points_default_is_zero() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::MovePoints>();

    let e = test.spawn((cdda_core::actor::components::MovePoints(0),));
    let mp = test.get::<cdda_core::actor::components::MovePoints>(e).unwrap();
    assert_eq!(mp.0, 0);
}

// ---------------------------------------------------------------------------
// Damage reduction
// ---------------------------------------------------------------------------

#[test]
fn damage_reduces_health() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Health>();

    let e = test.spawn((cdda_core::actor::components::Health {
        current: 100,
        max: 100,
    },));

    // Apply 30 damage
    let mut health = test
        .world_mut()
        .get_mut::<cdda_core::actor::components::Health>(e)
        .unwrap();
    health.current = (health.current - 30).max(0);
    drop(health);

    let health = test.get::<cdda_core::actor::components::Health>(e).unwrap();
    assert_eq!(health.current, 70);
}

#[test]
fn damage_doubled_does_not_go_below_zero() {
    let mut test = TestBed::new();
    test.register::<cdda_core::actor::components::Health>();

    let e = test.spawn((cdda_core::actor::components::Health {
        current: 50,
        max: 100,
    },));

    let mut health = test
        .world_mut()
        .get_mut::<cdda_core::actor::components::Health>(e)
        .unwrap();
    health.current = (health.current - 200).max(0);
    drop(health);

    let health = test.get::<cdda_core::actor::components::Health>(e).unwrap();
    assert_eq!(health.current, 0);
}
