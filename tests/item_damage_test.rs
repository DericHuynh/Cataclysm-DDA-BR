//! Item damage tests — damage level effects on item performance, repair, and
//! combat effectiveness.

use cdda_core::core::components::item::ItemDamage;
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Damage-level → effectiveness formula
// ---------------------------------------------------------------------------

/// Damage level to effectiveness multiplier.
/// Level 0 = 100% (no loss), level 5 = 20% (nearly useless).
fn damage_effectiveness(damage_level: u32) -> f64 {
    if damage_level >= 5 {
        0.2
    } else {
        1.0 - damage_level as f64 * 0.16
    }
}

/// Melee damage penalty from item damage.
fn melee_damage_penalty(base_damage: i32, damage_level: u32) -> i32 {
    let effectiveness = damage_effectiveness(damage_level);
    (base_damage as f64 * effectiveness).round() as i32
}

/// Armor effectiveness penalty from damage.
fn armor_effectiveness(base_coverage: u8, damage_level: u32) -> u8 {
    if damage_level == 0 {
        base_coverage
    } else {
        let reduction = (damage_level as f64 * 5.0) as u8; // 5% per damage level
        base_coverage.saturating_sub(reduction)
    }
}

/// Repair difficulty increases with damage level.
fn repair_difficulty(base_difficulty: u32, damage_level: u32) -> u32 {
    base_difficulty + damage_level * 2
}

// ---------------------------------------------------------------------------
// Component tests
// ---------------------------------------------------------------------------

#[test]
fn item_damage_zero() {
    let mut test = TestBed::new();
    test.register::<ItemDamage>();

    let e = test.spawn((ItemDamage(0),));
    let dmg = test.get::<ItemDamage>(e).unwrap();
    assert_eq!(dmg.0, 0);
}

#[test]
fn item_damage_moderate() {
    let mut test = TestBed::new();
    test.register::<ItemDamage>();

    let e = test.spawn((ItemDamage(2),));
    let dmg = test.get::<ItemDamage>(e).unwrap();
    assert_eq!(dmg.0, 2);
}

#[test]
fn item_damage_max() {
    let mut test = TestBed::new();
    test.register::<ItemDamage>();

    let e = test.spawn((ItemDamage(5),));
    let dmg = test.get::<ItemDamage>(e).unwrap();
    assert_eq!(dmg.0, 5);
}

#[test]
fn item_damage_display() {
    let mut test = TestBed::new();
    test.register::<ItemDamage>();

    let e = test.spawn((ItemDamage(3),));
    let dmg = test.get::<ItemDamage>(e).unwrap();
    // Verify the inner u32 field is accessible and can be converted
    let val: u32 = dmg.0;
    assert_eq!(val, 3);
}

// ---------------------------------------------------------------------------
// Formula tests — damage_effectiveness
// ---------------------------------------------------------------------------

#[test]
fn damage_effectiveness_pristine() {
    let eff = damage_effectiveness(0);
    assert!((eff - 1.0).abs() < f64::EPSILON);
}

#[test]
fn damage_effectiveness_moderate() {
    let eff = damage_effectiveness(2);
    assert!((eff - 0.68).abs() < f64::EPSILON);
}

#[test]
fn damage_effectiveness_max() {
    let eff = damage_effectiveness(5);
    assert!((eff - 0.2).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// Formula tests — melee_damage_penalty
// ---------------------------------------------------------------------------

#[test]
fn melee_damage_penalty_test() {
    // base 20, level 0 → 20
    assert_eq!(melee_damage_penalty(20, 0), 20);
    // base 20, level 2 → 20 * 0.68 = 13.6 → 14
    assert_eq!(melee_damage_penalty(20, 2), 14);
    // base 20, level 5 → 20 * 0.2 = 4
    assert_eq!(melee_damage_penalty(20, 5), 4);
}

// ---------------------------------------------------------------------------
// Formula tests — armor_effectiveness
// ---------------------------------------------------------------------------

#[test]
fn armor_effectiveness_pristine() {
    assert_eq!(armor_effectiveness(90, 0), 90);
}

#[test]
fn armor_effectiveness_damaged() {
    assert_eq!(armor_effectiveness(90, 2), 80);
}

#[test]
fn armor_effectiveness_heavily_damaged() {
    assert_eq!(armor_effectiveness(20, 5), 0);
}

// ---------------------------------------------------------------------------
// Formula tests — repair_difficulty
// ---------------------------------------------------------------------------

#[test]
fn repair_difficulty_test() {
    assert_eq!(repair_difficulty(5, 0), 5);
    assert_eq!(repair_difficulty(5, 3), 11);
}
