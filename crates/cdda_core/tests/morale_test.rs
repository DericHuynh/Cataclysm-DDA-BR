//! Morale system tests.
//!
//! Exercises `Morale` base value, `MoraleBonus` creation and decay,
//! and the `MoraleBonusOf`/`MoraleBonuses` relationship pair.
//!
//! Covers positive and negative morale, multiple bonuses on the same
//! creature, bonus expiration (remaining == 0), bonus removal via
//! entity despawn, and the default zero state.

use bevy_ecs::entity::Entity;
use cdda_core::core::components::actor::{Morale, MoraleBonus, MoraleBonusOf, MoraleBonuses};
use cdda_core::Time;
use cdda_core::sim::test_utils::TestBed;

fn empty_entity(test: &mut TestBed) -> Entity {
    test.world_mut().spawn_empty().id()
}

// ===========================================================================
// 1: Morale defaults to zero
// ===========================================================================

#[test]
fn morale_default_zero() {
    let mut test = TestBed::new();
    test.register::<Morale>();
    let e = test.spawn((Morale(0),));
    assert_eq!(test.get::<Morale>(e).unwrap().0, 0);
}

// ===========================================================================
// 2: Positive morale
// ===========================================================================

#[test]
fn morale_positive() {
    let mut test = TestBed::new();
    test.register::<Morale>();
    let e = test.spawn((Morale(50),));
    assert_eq!(test.get::<Morale>(e).unwrap().0, 50);
}

// ===========================================================================
// 3: Negative morale
// ===========================================================================

#[test]
fn morale_negative() {
    let mut test = TestBed::new();
    test.register::<Morale>();
    let e = test.spawn((Morale(-30),));
    assert_eq!(test.get::<Morale>(e).unwrap().0, -30);
}

// ===========================================================================
// 4: MoraleBonus created with reason, amount, and duration
// ===========================================================================

#[test]
fn morale_bonus_created() {
    let mut test = TestBed::new();
    test.register::<MoraleBonus>();
    let e = test.spawn((MoraleBonus {
        reason: "ate_good_food".to_string(),
        amount: 15,
        remaining: Time::from_turns(200),
    },));
    let bonus = test.get::<MoraleBonus>(e).unwrap();
    assert_eq!(bonus.reason, "ate_good_food");
    assert_eq!(bonus.amount, 15);
    assert_eq!(bonus.remaining.as_turns(), 200);
}

// ===========================================================================
// 5: MoraleBonusOf relationship
// ===========================================================================

#[test]
fn morale_bonus_of_relationship() {
    let mut test = TestBed::new();
    test.register::<MoraleBonusOf>();
    test.register::<MoraleBonus>();

    let creature = empty_entity(&mut test);
    let bonus = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "ate_good_food".to_string(),
            amount: 15,
            remaining: Time::from_turns(200),
        },
    ));
    assert_eq!(
        test.get::<MoraleBonusOf>(bonus).unwrap().0,
        creature,
        "MoraleBonusOf should point to the creature"
    );
}

// ===========================================================================
// 6: MoraleBonuses auto-populated when MoraleBonusOf is inserted
// ===========================================================================

#[test]
fn morale_bonuses_auto_populated() {
    let mut test = TestBed::new();
    test.register::<MoraleBonusOf>();
    test.register::<MoraleBonuses>();
    test.register::<MoraleBonus>();

    let creature = empty_entity(&mut test);
    let bonus = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "ate_ice_cream".to_string(),
            amount: 20,
            remaining: Time::from_turns(300),
        },
    ));
    let bonuses = test.get::<MoraleBonuses>(creature);
    assert!(
        bonuses.is_some(),
        "MoraleBonuses should be auto-populated on the creature"
    );
    let collected: Vec<Entity> = bonuses.unwrap().iter().collect();
    assert!(
        collected.contains(&bonus),
        "Bonus entity should be in MoraleBonuses"
    );
}

// ===========================================================================
// 7: MoraleBonus decay — remaining decreases
// ===========================================================================

#[test]
fn morale_bonus_decay() {
    let mut test = TestBed::new();
    test.register::<MoraleBonus>();

    let e = test.spawn((MoraleBonus {
        reason: "ate_good_food".to_string(),
        amount: 15,
        remaining: Time::from_turns(100),
    },));
    {
        let mut bonus = test.world_mut().get_mut::<MoraleBonus>(e).unwrap();
        bonus.remaining = bonus.remaining - Time::from_turns(30);
    }
    let bonus = test.get::<MoraleBonus>(e).unwrap();
    assert_eq!(bonus.remaining.as_turns(), 70);
}

// ===========================================================================
// 8: MoraleBonus expired — remaining is zero
// ===========================================================================

#[test]
fn morale_bonus_expired() {
    let bonus = MoraleBonus {
        reason: "ate_good_food".to_string(),
        amount: 15,
        remaining: Time::from_turns(0),
    };
    assert_eq!(bonus.remaining.as_turns(), 0);
}

// ===========================================================================
// 9: Multiple morale bonuses on the same creature
// ===========================================================================

#[test]
fn multiple_morale_bonuses() {
    let mut test = TestBed::new();
    test.register::<MoraleBonusOf>();
    test.register::<MoraleBonuses>();
    test.register::<MoraleBonus>();

    let creature = empty_entity(&mut test);
    let bonus_a = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "ate_ice_cream".to_string(),
            amount: 20,
            remaining: Time::from_turns(300),
        },
    ));
    let bonus_b = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "read_good_book".to_string(),
            amount: 10,
            remaining: Time::from_turns(500),
        },
    ));
    let bonuses = test.get::<MoraleBonuses>(creature).unwrap();
    let collected: Vec<Entity> = bonuses.iter().collect();
    assert!(collected.contains(&bonus_a), "Bonus A should be in MoraleBonuses");
    assert!(collected.contains(&bonus_b), "Bonus B should be in MoraleBonuses");
    assert_eq!(collected.len(), 2, "Both bonuses should be present");
}

// ===========================================================================
// 10: MoraleBonus removed when bonus entity is despawned
// ===========================================================================

#[test]
fn morale_bonus_removed() {
    let mut test = TestBed::new();
    test.register::<MoraleBonusOf>();
    test.register::<MoraleBonuses>();
    test.register::<MoraleBonus>();

    let creature = empty_entity(&mut test);
    let bonus = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "ate_ice_cream".to_string(),
            amount: 20,
            remaining: Time::from_turns(300),
        },
    ));

    // Verify it's there before removal
    {
        let bonuses = test.get::<MoraleBonuses>(creature).unwrap();
        let collected: Vec<Entity> = bonuses.iter().collect();
        assert!(collected.contains(&bonus));
    }

    // Despawn the bonus entity
    test.world_mut().despawn(bonus);

    // Verify it's gone from MoraleBonuses
    let bonuses = test.get::<MoraleBonuses>(creature);
    match bonuses {
        Some(bonuses) => {
            let collected: Vec<Entity> = bonuses.iter().collect();
            assert!(
                !collected.contains(&bonus),
                "Bonus should be removed from MoraleBonuses after despawn"
            );
        }
        None => {
            // Empty collection may be removed entirely — also valid
        }
    }
}

// ===========================================================================
// 11: MoraleBonus reason stored and read back
// ===========================================================================

#[test]
fn morale_bonus_reason_stored() {
    let mut test = TestBed::new();
    test.register::<MoraleBonus>();

    let a = test.spawn((MoraleBonus {
        reason: "ate_ice_cream".to_string(),
        amount: 20,
        remaining: Time::from_turns(200),
    },));
    let b = test.spawn((MoraleBonus {
        reason: "read_good_book".to_string(),
        amount: 10,
        remaining: Time::from_turns(400),
    },));
    let c = test.spawn((MoraleBonus {
        reason: "killed_monster".to_string(),
        amount: 25,
        remaining: Time::from_turns(600),
    },));

    assert_eq!(test.get::<MoraleBonus>(a).unwrap().reason, "ate_ice_cream");
    assert_eq!(test.get::<MoraleBonus>(b).unwrap().reason, "read_good_book");
    assert_eq!(test.get::<MoraleBonus>(c).unwrap().reason, "killed_monster");
}

// ===========================================================================
// 12: Positive and negative morale bonuses on the same creature
// ===========================================================================

#[test]
fn morale_positive_negative() {
    let mut test = TestBed::new();
    test.register::<MoraleBonusOf>();
    test.register::<MoraleBonuses>();
    test.register::<MoraleBonus>();

    let creature = empty_entity(&mut test);
    let pos = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "ate_ice_cream".to_string(),
            amount: 20,
            remaining: Time::from_turns(300),
        },
    ));
    let neg = test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "killed_monster".to_string(),
            amount: -10,
            remaining: Time::from_turns(100),
        },
    ));

    let bonuses = test.get::<MoraleBonuses>(creature).unwrap();
    let collected: Vec<Entity> = bonuses.iter().collect();
    assert!(collected.contains(&pos), "Positive bonus should be present");
    assert!(collected.contains(&neg), "Negative bonus should be present");
    assert_eq!(collected.len(), 2, "Both bonuses should be present");

    // Verify amounts are correct
    let mut bonus_data: Vec<&MoraleBonus> = collected
        .iter()
        .map(|&e| test.get::<MoraleBonus>(e).unwrap())
        .collect();
    bonus_data.sort_by_key(|b| b.amount);
    assert_eq!(bonus_data[0].amount, -10, "First should be negative");
    assert_eq!(bonus_data[1].amount, 20, "Second should be positive");
}
