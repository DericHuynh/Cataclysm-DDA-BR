//! Melee weapon and combat tests — translated from CDDA's melee_test.cpp.
//!
//! Tests WeaponData component creation and pure-function formulas for
//! melee damage, to-hit, moves, reach, and crit chance.

use cdda_core::core::components::def::{
    ItemCategory, ItemMaterials, ItemName, ItemPrice, ItemStackSize, ItemVolume, ItemWeight,
    WeaponData,
};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helper: pure formula functions (as declared in the test plan)
// ---------------------------------------------------------------------------

/// Average melee damage roll (dice * average of each die).
fn average_melee_damage(weapon: &WeaponData) -> f64 {
    let avg_die_roll = (weapon.dice_sides as f64 + 1.0) / 2.0;
    weapon.dice as f64 * avg_die_roll
}

/// Total damage combining the damage die roll and fixed damage bonuses.
fn total_melee_damage(weapon: &WeaponData, skill_bonus: i32, stat_bonus: i32) -> i32 {
    let base = average_melee_damage(weapon).round() as i32;
    let fixed = weapon.damage_bash + weapon.damage_cut + weapon.damage_stab;
    base + fixed + skill_bonus + stat_bonus
}

/// To-hit modifier combining weapon and skill.
fn total_to_hit(weapon_to_hit: i32, melee_skill: i32) -> i32 {
    weapon_to_hit + (melee_skill / 3)
}

/// Moves per attack with speed modifier.
/// A creature with 100 speed takes the listed moves_per_attack.
/// A creature with 150 speed takes 2/3 the time.
fn effective_moves(weapon_moves: i32, creature_speed: i32) -> i32 {
    let modifier = 100.0 / creature_speed as f64;
    (weapon_moves as f64 * modifier).round() as i32
}

/// Reach attack available check.
fn can_reach_attack(weapon: &WeaponData, target_distance: u8) -> bool {
    weapon.reach >= target_distance
}

/// Crit chance based on to-hit bonus. Higher to-hit = higher crit chance.
fn crit_chance(weapon_to_hit: i32, melee_skill: i32) -> f64 {
    let total = total_to_hit(weapon_to_hit, melee_skill);
    (total as f64 * 0.02).clamp(0.01, 0.30)
}

// ---------------------------------------------------------------------------
// WeaponData component tests
// ---------------------------------------------------------------------------

#[test]
fn weapon_combat_knife() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("combat_knife".to_string()),
        ItemWeight(300),
        ItemVolume(500),
        ItemPrice {
            price: 12000,
            price_postapoc: 12000,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 6,
            damage_cut: 14,
            damage_stab: 0,
            to_hit: 2,
            moves_per_attack: 85,
            reach: 1,
            techniques: vec!["RAPID".to_string(), "BLOCK".to_string()],
            dice: 1,
            dice_sides: 4,
            skill: "cutting".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.damage_bash, 6);
    assert_eq!(weapon.damage_cut, 14);
    assert_eq!(weapon.damage_stab, 0);
    assert_eq!(weapon.to_hit, 2);
    assert_eq!(weapon.moves_per_attack, 85);
    assert_eq!(weapon.reach, 1);
    assert_eq!(weapon.techniques, vec!["RAPID", "BLOCK"]);
    assert_eq!(weapon.dice, 1);
    assert_eq!(weapon.dice_sides, 4);
    assert_eq!(weapon.skill, "cutting");
}

#[test]
fn weapon_heavy_hammer() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("heavy_hammer".to_string()),
        ItemWeight(4000),
        ItemVolume(6000),
        ItemPrice {
            price: 25000,
            price_postapoc: 25000,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 30,
            damage_cut: 0,
            damage_stab: 0,
            to_hit: -1,
            moves_per_attack: 200,
            reach: 1,
            techniques: vec!["SWEEP".to_string(), "BLOCK".to_string()],
            dice: 4,
            dice_sides: 6,
            skill: "bashing".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.damage_bash, 30);
    assert_eq!(weapon.damage_cut, 0);
    assert_eq!(weapon.damage_stab, 0);
    assert_eq!(weapon.to_hit, -1);
    assert_eq!(weapon.moves_per_attack, 200);
    assert_eq!(weapon.reach, 1);
    assert_eq!(weapon.techniques, vec!["SWEEP", "BLOCK"]);
    assert_eq!(weapon.dice, 4);
    assert_eq!(weapon.dice_sides, 6);
    assert_eq!(weapon.skill, "bashing");
}

#[test]
fn weapon_spear() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("spear".to_string()),
        ItemWeight(1000),
        ItemVolume(3000),
        ItemPrice {
            price: 15000,
            price_postapoc: 15000,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 5,
            damage_cut: 0,
            damage_stab: 20,
            to_hit: 1,
            moves_per_attack: 100,
            reach: 2,
            techniques: vec!["IMPALE".to_string()],
            dice: 2,
            dice_sides: 6,
            skill: "stabbing".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.reach, 2);
    assert_eq!(weapon.damage_stab, 20);
    assert_eq!(weapon.damage_bash, 5);
    assert_eq!(weapon.to_hit, 1);
}

#[test]
fn weapon_no_techniques() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("simple_club".to_string()),
        ItemWeight(800),
        ItemVolume(1500),
        ItemPrice {
            price: 200,
            price_postapoc: 200,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 10,
            damage_cut: 0,
            damage_stab: 0,
            to_hit: 0,
            moves_per_attack: 120,
            reach: 1,
            techniques: vec![],
            dice: 2,
            dice_sides: 4,
            skill: "bashing".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert!(weapon.techniques.is_empty());
}

#[test]
fn weapon_negative_to_hit() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("unwieldy_axe".to_string()),
        ItemWeight(5000),
        ItemVolume(8000),
        ItemPrice {
            price: 8000,
            price_postapoc: 8000,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 20,
            damage_cut: 10,
            damage_stab: 0,
            to_hit: -3,
            moves_per_attack: 180,
            reach: 1,
            techniques: vec!["SWEEP".to_string()],
            dice: 3,
            dice_sides: 6,
            skill: "bashing".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.to_hit, -3);
}

#[test]
fn weapon_zero_damage() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("improvised_weapon".to_string()),
        ItemWeight(200),
        ItemVolume(400),
        ItemPrice {
            price: 50,
            price_postapoc: 50,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 0,
            damage_cut: 0,
            damage_stab: 0,
            to_hit: -2,
            moves_per_attack: 150,
            reach: 1,
            techniques: vec![],
            dice: 1,
            dice_sides: 2,
            skill: "bashing".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.damage_bash, 0);
    assert_eq!(weapon.damage_cut, 0);
    assert_eq!(weapon.damage_stab, 0);
    assert_eq!(weapon.dice, 1);
    assert_eq!(weapon.dice_sides, 2);
}

#[test]
fn weapon_high_dice() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("greatsword".to_string()),
        ItemWeight(3500),
        ItemVolume(6000),
        ItemPrice {
            price: 35000,
            price_postapoc: 35000,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 8,
            damage_cut: 25,
            damage_stab: 0,
            to_hit: 1,
            moves_per_attack: 150,
            reach: 1,
            techniques: vec!["RAPID".to_string(), "BLOCK".to_string(), "WIDE".to_string()],
            dice: 5,
            dice_sides: 10,
            skill: "cutting".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.dice, 5);
    assert_eq!(weapon.dice_sides, 10);
}

#[test]
fn weapon_many_techniques() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemStackSize>();
    test.register::<ItemCategory>();
    test.register::<cdda_core::data::flags::ItemFlagList>();
    test.register::<ItemMaterials>();

    let e = test.spawn((
        ItemName("masterwork_blade".to_string()),
        ItemWeight(2500),
        ItemVolume(5000),
        ItemPrice {
            price: 50000,
            price_postapoc: 50000,
        },
        ItemStackSize(1),
        ItemCategory("weapons".to_string()),
        WeaponData {
            damage_bash: 10,
            damage_cut: 20,
            damage_stab: 5,
            to_hit: 3,
            moves_per_attack: 120,
            reach: 1,
            techniques: vec![
                "RAPID".to_string(),
                "SWEEP".to_string(),
                "PRECISE".to_string(),
                "BLOCK".to_string(),
                "WIDE".to_string(),
            ],
            dice: 3,
            dice_sides: 8,
            skill: "cutting".to_string(),
        },
    ));

    let weapon = test.get::<WeaponData>(e).unwrap();
    assert_eq!(weapon.techniques.len(), 5);
    assert_eq!(weapon.techniques[0], "RAPID");
    assert_eq!(weapon.techniques[1], "SWEEP");
    assert_eq!(weapon.techniques[2], "PRECISE");
    assert_eq!(weapon.techniques[3], "BLOCK");
    assert_eq!(weapon.techniques[4], "WIDE");
}

// ---------------------------------------------------------------------------
// Pure function formula tests
// ---------------------------------------------------------------------------

#[test]
fn average_melee_damage_combat_knife() {
    let weapon = WeaponData {
        damage_bash: 6,
        damage_cut: 14,
        damage_stab: 0,
        to_hit: 2,
        moves_per_attack: 85,
        reach: 1,
        techniques: vec!["RAPID".to_string(), "BLOCK".to_string()],
        dice: 1,
        dice_sides: 4,
        skill: "cutting".to_string(),
    };
    // avg = 1 * (4+1)/2 = 2.5
    let result = average_melee_damage(&weapon);
    assert!((result - 2.5).abs() < f64::EPSILON);
}

#[test]
fn average_melee_damage_hammer() {
    let weapon = WeaponData {
        damage_bash: 30,
        damage_cut: 0,
        damage_stab: 0,
        to_hit: -1,
        moves_per_attack: 200,
        reach: 1,
        techniques: vec!["SWEEP".to_string(), "BLOCK".to_string()],
        dice: 4,
        dice_sides: 6,
        skill: "bashing".to_string(),
    };
    // avg = 4 * (6+1)/2 = 4 * 3.5 = 14.0
    let result = average_melee_damage(&weapon);
    assert!((result - 14.0).abs() < f64::EPSILON);
}

#[test]
fn total_melee_damage_test() {
    let weapon = WeaponData {
        damage_bash: 6,
        damage_cut: 14,
        damage_stab: 0,
        to_hit: 2,
        moves_per_attack: 85,
        reach: 1,
        techniques: vec!["RAPID".to_string(), "BLOCK".to_string()],
        dice: 1,
        dice_sides: 4,
        skill: "cutting".to_string(),
    };
    // base = round(2.5) = 3, fixed = 6+14+0 = 20, skill=5, stat=3 => 3+20+5+3 = 31
    let result = total_melee_damage(&weapon, 5, 3);
    assert_eq!(result, 31);
}

#[test]
fn total_to_hit_test() {
    let result = total_to_hit(2, 6);
    // 2 + (6/3) = 2 + 2 = 4
    assert_eq!(result, 4);
}

#[test]
fn effective_moves_test() {
    // 85 moves, speed 100 => 85 * 100/100 = 85
    let result_normal = effective_moves(85, 100);
    assert_eq!(result_normal, 85);

    // 85 moves, speed 150 => 85 * 100/150 = 56.666... -> rounds to 57
    let result_fast = effective_moves(85, 150);
    assert_eq!(result_fast, 57);
}

#[test]
fn crit_chance_test() {
    // to_hit=2, skill=9 => total=2+9/3=5 => 5*0.02 = 0.10
    let result = crit_chance(2, 9);
    assert!((result - 0.10).abs() < f64::EPSILON);
}

#[test]
fn reach_attack_test() {
    let short_weapon = WeaponData {
        damage_bash: 5,
        damage_cut: 0,
        damage_stab: 0,
        to_hit: 0,
        moves_per_attack: 100,
        reach: 1,
        techniques: vec![],
        dice: 1,
        dice_sides: 4,
        skill: "bashing".to_string(),
    };
    let long_weapon = WeaponData {
        damage_bash: 5,
        damage_cut: 0,
        damage_stab: 20,
        to_hit: 1,
        moves_per_attack: 100,
        reach: 2,
        techniques: vec!["IMPALE".to_string()],
        dice: 2,
        dice_sides: 6,
        skill: "stabbing".to_string(),
    };
    // Short weapon can reach distance 1 but not 2
    assert!(can_reach_attack(&short_weapon, 1));
    assert!(!can_reach_attack(&short_weapon, 2));
    // Long weapon can reach distance 2
    assert!(can_reach_attack(&long_weapon, 2));
    assert!(can_reach_attack(&long_weapon, 1));
}
