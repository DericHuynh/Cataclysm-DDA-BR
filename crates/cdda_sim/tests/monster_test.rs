//! Tests for monster definition components — MonsterStats, MonsterMelee,
//! MonsterVision, MonsterArmour, MonsterFlags, MonsterSpecies, and related.
//!
//! These are pure component construction/inspection tests — no systems
//! required. They verify that def-level components store and retrieve
//! the CDDA-derived monster fields correctly.

use cdda_sim::test_utils::TestBed;

// Use def component types
use cdda_sim::def_components::*;

// ---------------------------------------------------------------------------
// MonsterName / MonsterDescription
// ---------------------------------------------------------------------------

#[test]
fn monster_name_stored() {
    let mut test = TestBed::new();
    test.register::<MonsterName>();
    let e = test.spawn((MonsterName("zombie".to_string()),));
    assert_eq!(test.get::<MonsterName>(e).unwrap().0, "zombie");
}

#[test]
fn monster_description_stored() {
    let mut test = TestBed::new();
    test.register::<MonsterDescription>();
    let e = test.spawn((MonsterDescription("A shambling undead".to_string()),));
    assert_eq!(
        test.get::<MonsterDescription>(e).unwrap().0,
        "A shambling undead"
    );
}

// ---------------------------------------------------------------------------
// MonsterStats
// ---------------------------------------------------------------------------

#[test]
fn monster_stats_all_fields() {
    let mut test = TestBed::new();
    test.register::<MonsterStats>();
    let e = test.spawn((MonsterStats {
        hp: 60,
        speed: 70,
        attack_cost: 100,
        dodge: 2,
        morale: 30,
        aggression: 100,
        melee_skill: 4,
        melee_dice: 2,
        melee_dice_sides: 6,
        grab_strength: 0,
        bleed_rate: 20,
        diff: 2,
    },));
    let s = test.get::<MonsterStats>(e).unwrap();
    assert_eq!(s.hp, 60);
    assert_eq!(s.speed, 70);
    assert_eq!(s.attack_cost, 100);
    assert_eq!(s.dodge, 2);
    assert_eq!(s.morale, 30);
    assert_eq!(s.aggression, 100);
    assert_eq!(s.melee_skill, 4);
    assert_eq!(s.melee_dice, 2);
    assert_eq!(s.melee_dice_sides, 6);
}

#[test]
fn monster_stats_zero_values() {
    let mut test = TestBed::new();
    test.register::<MonsterStats>();
    let e = test.spawn((MonsterStats {
        hp: 0,
        speed: 0,
        attack_cost: 0,
        dodge: 0,
        morale: 0,
        aggression: 0,
        melee_skill: 0,
        melee_dice: 0,
        melee_dice_sides: 0,
        grab_strength: 0,
        bleed_rate: 0,
        diff: 0,
    },));
    let s = test.get::<MonsterStats>(e).unwrap();
    assert_eq!(s.hp, 0);
}

#[test]
fn monster_stats_high_values() {
    let mut test = TestBed::new();
    test.register::<MonsterStats>();
    let e = test.spawn((MonsterStats {
        hp: 500,
        speed: 200,
        attack_cost: 50,
        dodge: 20,
        morale: 100,
        aggression: 20,
        melee_skill: 20,
        melee_dice: 10,
        melee_dice_sides: 12,
        grab_strength: 5,
        bleed_rate: 0,
        diff: 15,
    },));
    let s = test.get::<MonsterStats>(e).unwrap();
    assert_eq!(s.hp, 500);
    assert_eq!(s.melee_dice, 10);
    assert_eq!(s.melee_dice_sides, 12);
}

// ---------------------------------------------------------------------------
// MonsterMelee
// ---------------------------------------------------------------------------

#[test]
fn monster_melee_fields() {
    let mut test = TestBed::new();
    test.register::<MonsterMelee>();
    let e = test.spawn((MonsterMelee {
        dice: 2,
        dice_sides: 6,
        damage_bash: 6,
        damage_cut: 0,
        damage_stab: 0,
        to_hit: 1,
    },));
    let m = test.get::<MonsterMelee>(e).unwrap();
    assert_eq!(m.dice, 2);
    assert_eq!(m.dice_sides, 6);
    assert_eq!(m.damage_bash, 6);
    assert_eq!(m.to_hit, 1);
}

#[test]
fn monster_melee_all_damage_types() {
    let mut test = TestBed::new();
    test.register::<MonsterMelee>();
    let e = test.spawn((MonsterMelee {
        dice: 1,
        dice_sides: 4,
        damage_bash: 2,
        damage_cut: 8,
        damage_stab: 4,
        to_hit: 2,
    },));
    let m = test.get::<MonsterMelee>(e).unwrap();
    assert_eq!(m.damage_cut, 8);
    assert_eq!(m.damage_stab, 4);
}

// ---------------------------------------------------------------------------
// MonsterVision
// ---------------------------------------------------------------------------

#[test]
fn monster_vision_fields() {
    let mut test = TestBed::new();
    test.register::<MonsterVision>();
    let e = test.spawn((MonsterVision { day: 40, night: 5 },));
    let v = test.get::<MonsterVision>(e).unwrap();
    assert_eq!(v.day, 40);
    assert_eq!(v.night, 5);
}

#[test]
fn monster_vision_no_night() {
    let mut test = TestBed::new();
    test.register::<MonsterVision>();
    let e = test.spawn((MonsterVision { day: 40, night: 0 },));
    let v = test.get::<MonsterVision>(e).unwrap();
    assert_eq!(v.night, 0);
}

// ---------------------------------------------------------------------------
// MonsterArmour
// ---------------------------------------------------------------------------

#[test]
fn monster_armour_fields() {
    let mut test = TestBed::new();
    test.register::<MonsterArmour>();
    let e = test.spawn((MonsterArmour {
        bash: 3,
        cut: 5,
        bullet: 2,
        fire: 0,
        acid: 0,
        electric: 0,
        cold: 0,
        stab: 4,
    },));
    let a = test.get::<MonsterArmour>(e).unwrap();
    assert_eq!(a.bash, 3);
    assert_eq!(a.cut, 5);
    assert_eq!(a.bullet, 2);
    assert_eq!(a.stab, 4);
}

#[test]
fn monster_armour_all_types() {
    let mut test = TestBed::new();
    test.register::<MonsterArmour>();
    let e = test.spawn((MonsterArmour {
        bash: 2,
        cut: 3,
        bullet: 4,
        fire: 5,
        acid: 1,
        electric: 2,
        cold: 3,
        stab: 6,
    },));
    let a = test.get::<MonsterArmour>(e).unwrap();
    assert_eq!(a.fire, 5);
    assert_eq!(a.acid, 1);
    assert_eq!(a.electric, 2);
    assert_eq!(a.cold, 3);
}

// ---------------------------------------------------------------------------
// MonsterFlags
// ---------------------------------------------------------------------------

#[test]
fn monster_flags_vec() {
    let mut test = TestBed::new();
    test.register::<MonsterFlags>();
    let e = test.spawn((MonsterFlags(vec![
        "SEES".to_string(),
        "HEARS".to_string(),
        "POISON".to_string(),
    ]),));
    let f = test.get::<MonsterFlags>(e).unwrap();
    assert!(f.0.contains(&"SEES".to_string()));
    assert!(f.0.contains(&"POISON".to_string()));
}

// ---------------------------------------------------------------------------
// MonsterSpecies
// ---------------------------------------------------------------------------

#[test]
fn monster_species_single() {
    let mut test = TestBed::new();
    test.register::<MonsterSpecies>();
    let e = test.spawn((MonsterSpecies(vec!["ZOMBIE".to_string()]),));
    let s = test.get::<MonsterSpecies>(e).unwrap();
    assert_eq!(s.0, vec!["ZOMBIE"]);
}

#[test]
fn monster_species_multiple() {
    let mut test = TestBed::new();
    test.register::<MonsterSpecies>();
    let e = test.spawn((MonsterSpecies(vec![
        "MAMMAL".to_string(),
        "PREDATOR".to_string(),
    ]),));
    let s = test.get::<MonsterSpecies>(e).unwrap();
    assert_eq!(s.0.len(), 2);
}

// ---------------------------------------------------------------------------
// MonsterDefaultFaction
// ---------------------------------------------------------------------------

#[test]
fn monster_default_faction() {
    let mut test = TestBed::new();
    test.register::<MonsterDefaultFaction>();
    let e = test.spawn((MonsterDefaultFaction("zombie".to_string()),));
    assert_eq!(test.get::<MonsterDefaultFaction>(e).unwrap().0, "zombie");
}

// ---------------------------------------------------------------------------
// MonsterBodyType
// ---------------------------------------------------------------------------

#[test]
fn monster_body_type() {
    let mut test = TestBed::new();
    test.register::<MonsterBodyType>();
    let e = test.spawn((MonsterBodyType("human".to_string()),));
    assert_eq!(test.get::<MonsterBodyType>(e).unwrap().0, "human");
}

// ---------------------------------------------------------------------------
// MonsterUpgrade
// ---------------------------------------------------------------------------

#[test]
fn monster_upgrade_fields() {
    let mut test = TestBed::new();
    test.register::<MonsterUpgrade>();
    let e = test.spawn((MonsterUpgrade {
        into: Some("mon_zombie_tough".to_string()),
        into_group: None,
        into_time: 30,
        into_pct: 0.1,
    },));
    let u = test.get::<MonsterUpgrade>(e).unwrap();
    assert_eq!(u.into, Some("mon_zombie_tough".to_string()));
    assert!(u.into_group.is_none());
    assert_eq!(u.into_time, 30);
    assert!((u.into_pct - 0.1).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// MonsterHarvest
// ---------------------------------------------------------------------------

#[test]
fn monster_harvest() {
    let mut test = TestBed::new();
    test.register::<MonsterHarvest>();
    let e = test.spawn((MonsterHarvest("zombie".to_string()),));
    assert_eq!(test.get::<MonsterHarvest>(e).unwrap().0, "zombie");
}

// ---------------------------------------------------------------------------
// MonsterDeathFunction / MonsterDeathDrops
// ---------------------------------------------------------------------------

#[test]
fn monster_death_function() {
    let mut test = TestBed::new();
    test.register::<MonsterDeathFunction>();
    let e = test.spawn((MonsterDeathFunction("DISAPPEAR".to_string()),));
    assert_eq!(test.get::<MonsterDeathFunction>(e).unwrap().0, "DISAPPEAR");
}

#[test]
fn monster_death_drops() {
    let mut test = TestBed::new();
    test.register::<MonsterDeathDrops>();
    let e = test.spawn((MonsterDeathDrops("mon_zombie_death_drops".to_string()),));
    assert_eq!(
        test.get::<MonsterDeathDrops>(e).unwrap().0,
        "mon_zombie_death_drops"
    );
}

// ---------------------------------------------------------------------------
// MonsterSpecialAttacks
// ---------------------------------------------------------------------------

#[test]
fn monster_special_attacks() {
    let mut test = TestBed::new();
    test.register::<MonsterSpecialAttacks>();
    let e = test.spawn((MonsterSpecialAttacks(vec![
        "GRAB".to_string(),
        "LEAP".to_string(),
    ]),));
    let s = test.get::<MonsterSpecialAttacks>(e).unwrap();
    assert!(s.0.contains(&"GRAB".to_string()));
    assert_eq!(s.0.len(), 2);
}
