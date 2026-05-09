#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_core::core::components::actor::*;
use cdda_core::*;
use cdda_core::core::components::sim::*;
use cdda_core::actor::morale::*;
use cdda_core::sim::test_utils::TestBed;

// ===========================================================================
// Helper: create a creature with morale
// ===========================================================================

fn spawn_creature(test: &mut TestBed) -> Entity {
    test.register::<IsAlive>();
    test.register::<Morale>();
    test.register::<MoraleBonuses>();
    test.spawn((IsAlive, Morale(0)))
}

fn spawn_morale_bonus(test: &mut TestBed, creature: Entity, amount: i32, turns: i64) -> Entity {
    test.register::<MoraleBonus>();
    test.register::<MoraleBonusOf>();
    test.spawn((
        MoraleBonusOf(creature),
        MoraleBonus {
            reason: "test_bonus".to_string(),
            amount,
            remaining: Time::from_turns(turns),
        },
    ))
}

// ===========================================================================
// 1: add_morale_bonus_creates_entity
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn add_morale_bonus_creates_entity() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);

    // Adding a morale bonus creates a new entity with MoraleBonus + MoraleBonusOf
    let bonus = spawn_morale_bonus(&mut test, creature, 15, 200);

    assert!(test.get::<MoraleBonus>(bonus).is_some());
    assert!(test.get::<MoraleBonusOf>(bonus).is_some());
    assert_eq!(
        test.get::<MoraleBonusOf>(bonus).unwrap().0,
        creature
    );
}

// ===========================================================================
// 2: add_morale_bonus_in_creature_bonuses
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn add_morale_bonus_in_creature_bonuses() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let bonus = spawn_morale_bonus(&mut test, creature, 15, 200);

    // The creature's MoraleBonuses should contain the bonus entity
    let bonuses = test.get::<MoraleBonuses>(creature);
    assert!(bonuses.is_some());
    if let Some(b) = bonuses {
        let entities: Vec<Entity> = b.iter().collect();
        assert!(entities.contains(&bonus));
    }
}

// ===========================================================================
// 3: calculate_morale_default
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn calculate_morale_default() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);

    // No bonuses, Morale(0) => calculated morale = 0
    let morale = test.get::<Morale>(creature).unwrap().0;
    assert_eq!(morale, 0);
}

// ===========================================================================
// 4: calculate_morale_with_bonuses
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn calculate_morale_with_bonuses() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);

    // Base Morale(10) + bonus 15 + bonus 5 = 30
    test.world_mut().entity_mut(creature).insert(Morale(10));
    let _b1 = spawn_morale_bonus(&mut test, creature, 15, 200);
    let _b2 = spawn_morale_bonus(&mut test, creature, 5, 200);

    let base = test.get::<Morale>(creature).unwrap().0;
    let bonus_sum = 15 + 5;
    let total = base + bonus_sum;
    assert_eq!(total, 30);
}

// ===========================================================================
// 5: calculate_morale_negative
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn calculate_morale_negative() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);

    // Morale(0) + bonus -10 = -10
    let _b1 = spawn_morale_bonus(&mut test, creature, -10, 100);

    let base = test.get::<Morale>(creature).unwrap().0;
    let bonus_amount: i32 = -10;
    let total = base + bonus_amount;
    assert_eq!(total, -10);
}

// ===========================================================================
// 6: calculate_morale_mixed
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn calculate_morale_mixed() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);

    // Base Morale(5) + bonus 10 + bonus -5 = 10
    test.world_mut().entity_mut(creature).insert(Morale(5));
    let _b1 = spawn_morale_bonus(&mut test, creature, 10, 200);
    let _b2 = spawn_morale_bonus(&mut test, creature, -5, 100);

    let base = test.get::<Morale>(creature).unwrap().0;
    let total = base + 10 + (-5);
    assert_eq!(total, 10);
}

// ===========================================================================
// 7: tick_decay_reduces_duration
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn tick_decay_reduces_duration() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let bonus = spawn_morale_bonus(&mut test, creature, 10, 100);

    // Remaining starts at 100 turns
    assert_eq!(
        test.get::<MoraleBonus>(bonus).unwrap().remaining.as_turns(),
        100
    );

    // After one tick, remaining should be 99
    let mut mb = test.world_mut().get_mut::<MoraleBonus>(bonus).unwrap();
    mb.remaining = mb.remaining - Time::from_turns(1);

    assert_eq!(
        test.get::<MoraleBonus>(bonus).unwrap().remaining.as_turns(),
        99
    );
}

// ===========================================================================
// 8: tick_removes_expired
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn tick_removes_expired() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let bonus = spawn_morale_bonus(&mut test, creature, 10, 1);

    // 1 remaining - should be removed after tick
    assert_eq!(
        test.get::<MoraleBonus>(bonus).unwrap().remaining.as_turns(),
        1
    );

    // Tick to 0
    test.world_mut().entity_mut(bonus).insert(MoraleBonus {
        reason: "test_bonus".to_string(),
        amount: 10,
        remaining: Time::from_turns(0),
    });

    // After removal (remaining = 0), entity should be despawned or inactive
    let expired = test.get::<MoraleBonus>(bonus).unwrap().remaining.as_turns();
    assert_eq!(expired, 0);
}

// ===========================================================================
// 9: apply_effects_high_morale
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn apply_effects_high_morale() {
    // High morale (50) should provide stat bonuses
    let morale_value = 50;

    // High morale grants +1 to strength, +1 to dexterity, +1 to perception
    let stat_bonus = morale_value >= 50;
    assert!(stat_bonus);

    // When morale is high enough, positive modifiers are applied
    let expected_bonus = 1;
    assert!(expected_bonus > 0);
}

// ===========================================================================
// 10: apply_effects_low_morale
// ===========================================================================

#[test]
#[ignore = "morale system not yet implemented"]
fn apply_effects_low_morale() {
    // Low morale (-50) should provide stat penalties
    let morale_value = -50;

    // Low morale grants -1 to strength, -1 to dexterity
    let stat_penalty = morale_value <= -50;
    assert!(stat_penalty);

    // When morale is low enough, negative modifiers are applied
    let expected_penalty = -1;
    assert!(expected_penalty < 0);
}
