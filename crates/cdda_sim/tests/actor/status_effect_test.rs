use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;
use cdda_components::Time;
use cdda_components::actor::{
    Bleeding, EffectOn, IsAlive, Morale, MoraleBonus, OnFire, StatusEffect, Stunned,
};
use cdda_components::EffectId;
use cdda_sim::runtime::test_utils::TestBed;

fn empty_entity(test: &mut TestBed) -> Entity {
    test.world_mut().spawn_empty().id()
}

// Effect basics
#[test]
fn effect_has_id_and_duration() {
    let mut test = TestBed::new();
    test.register::<StatusEffect>();

    let creature = empty_entity(&mut test);
    let e = test.spawn((
        EffectOn(creature),
        StatusEffect {
            effect_id: EffectId::from(0u32),
            intensity: 1,
            remaining: Time::from_turns(100),
        },
    ));
    let eff = test.get::<StatusEffect>(e).unwrap();
    assert_eq!(eff.intensity, 1);
    assert!(eff.remaining.as_turns() > 0);
}

#[test]
fn effect_intensity_increases() {
    let mut test = TestBed::new();
    test.register::<StatusEffect>();

    let creature = empty_entity(&mut test);
    let e = test.spawn((
        EffectOn(creature),
        StatusEffect {
            effect_id: EffectId::from(0u32),
            intensity: 2,
            remaining: Time::from_turns(100),
        },
    ));
    assert_eq!(
        test.get::<StatusEffect>(e)
            .unwrap()
            .intensity,
        2
    );
}

#[test]
fn effect_duration_decays() {
    let mut test = TestBed::new();
    test.register::<StatusEffect>();

    let creature = empty_entity(&mut test);
    let e = test.spawn((
        EffectOn(creature),
        StatusEffect {
            effect_id: EffectId::from(0u32),
            intensity: 1,
            remaining: Time::from_turns(50),
        },
    ));
    {
        let mut eff = test
            .world_mut()
            .get_mut::<StatusEffect>(e)
            .unwrap();
        eff.remaining = eff.remaining - Time::from_turns(10);
    }
    assert_eq!(
        test.get::<StatusEffect>(e)
            .unwrap()
            .remaining
            .as_turns(),
        40
    );
}

#[test]
fn effect_expired_at_zero() {
    let eff = StatusEffect {
        effect_id: EffectId::from(0u32),
        intensity: 1,
        remaining: Time::from_turns(0),
    };
    assert_eq!(eff.remaining.as_turns(), 0);
}

// Status markers
#[test]
fn bleeding_marker_present() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<Bleeding>();

    let e = test.spawn((
        IsAlive,
        Bleeding,
    ));
    assert!(test
        .world()
        .entity(e)
        .contains::<Bleeding>());
}

#[test]
fn stunned_marker_present() {
    let mut test = TestBed::new();
    test.register::<Stunned>();
    let e = test.spawn((Stunned,));
    assert!(test
        .world()
        .entity(e)
        .contains::<Stunned>());
}

#[test]
fn on_fire_marker_present() {
    let mut test = TestBed::new();
    test.register::<OnFire>();
    let e = test.spawn((OnFire,));
    assert!(test
        .world()
        .entity(e)
        .contains::<OnFire>());
}

// Effect relationships
#[test]
fn effect_points_to_creature() {
    let mut test = TestBed::new();
    let creature = test.spawn((IsAlive,));
    let effect = test.spawn((
        EffectOn(creature),
        StatusEffect {
            effect_id: EffectId::from(0u32),
            intensity: 1,
            remaining: Time::from_turns(100),
        },
    ));
    assert_eq!(
        test.get::<EffectOn>(effect)
            .unwrap()
            .0,
        creature
    );
}

// Morale
#[test]
fn morale_default_is_zero() {
    let mut test = TestBed::new();
    test.register::<Morale>();
    let e = test.spawn((Morale(0),));
    assert_eq!(test.get::<Morale>(e).unwrap().0, 0);
}

#[test]
fn morale_increases_with_bonus() {
    let mut test = TestBed::new();
    test.register::<Morale>();
    let e = test.spawn((Morale(10),));
    assert!(test.get::<Morale>(e).unwrap().0 > 0);
}

#[test]
fn morale_negative_from_bad_events() {
    let mut test = TestBed::new();
    test.register::<Morale>();
    let e = test.spawn((Morale(-10),));
    assert!(test.get::<Morale>(e).unwrap().0 < 0);
}

#[test]
fn morale_bonus_has_reason_and_duration() {
    let mut test = TestBed::new();
    test.register::<MoraleBonus>();
    let e = test.spawn((MoraleBonus {
        reason: "ate_ice_cream".to_string(),
        amount: 15,
        remaining: Time::from_turns(200),
    },));
    let bonus = test.get::<MoraleBonus>(e).unwrap();
    assert_eq!(bonus.reason, "ate_ice_cream");
    assert_eq!(bonus.amount, 15);
    assert!(bonus.remaining.as_turns() > 0);
}

// Creature status combinations
#[test]
fn creature_can_have_multiple_statuses() {
    let mut test = TestBed::new();
    test.register::<IsAlive>();
    test.register::<Stunned>();
    test.register::<Bleeding>();

    let e = test.spawn((
        IsAlive,
        Stunned,
        Bleeding,
    ));
    assert!(test
        .world()
        .entity(e)
        .contains::<IsAlive>());
    assert!(test
        .world()
        .entity(e)
        .contains::<Stunned>());
    assert!(test
        .world()
        .entity(e)
        .contains::<Bleeding>());
}

#[test]
fn status_removed_after_despawn() {
    let mut test = TestBed::new();
    test.register::<Stunned>();

    let e = test.spawn((Stunned,));
    test.world_mut().despawn(e);

    let mut q = test.world_mut().query::<&Stunned>();
    assert!(q.iter(test.world()).next().is_none());
}
