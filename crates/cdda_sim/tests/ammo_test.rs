//! Ammo, gun, and magazine component tests — covering ammunition properties,
//! ranged weapon stats, magazine capacities, and combat formula helpers.
//!
//! Translated from CDDA's ranged balance and ammo type tests.

use cdda_components::def::{AmmoData, GunData, MagazineData};
use cdda_components::{AmmoTypeId, SkillId};
use cdda_data::interner::AmmoTypeRegistry;
use cdda_data::interner::*;
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// AmmoData component tests
// ---------------------------------------------------------------------------

#[test]
fn ammo_data_fields() {
    let mut test = TestBed::new();
    test.register::<AmmoData>();
    let e = test.spawn((AmmoData {
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        damage: 18,
        pierce: 0,
        range: 14,
        dispersion: 180,
        recoil: 18,
        count: 50,
        casing: Some("9mm_casing".to_string()),
        effects: vec!["NEVER_MISFIRES".to_string()],
        stack_size: 50,
    },));
    let ammo = test.get::<AmmoData>(e).unwrap();
    assert_eq!(ammo.ammo_type, AmmoTypeRegistry::default().intern("9mm"));
    assert_eq!(ammo.damage, 18);
    assert_eq!(ammo.pierce, 0);
    assert_eq!(ammo.range, 14);
    assert_eq!(ammo.dispersion, 180);
    assert_eq!(ammo.recoil, 18);
    assert_eq!(ammo.count, 50);
    assert_eq!(ammo.casing, Some("9mm_casing".to_string()));
    assert_eq!(ammo.effects, vec!["NEVER_MISFIRES"]);
    assert_eq!(ammo.stack_size, 50);
}

#[test]
fn ammo_no_casing() {
    let mut test = TestBed::new();
    test.register::<AmmoData>();
    let e = test.spawn((AmmoData {
        ammo_type: AmmoTypeRegistry::default().intern("shot"),
        damage: 70,
        pierce: 0,
        range: 6,
        dispersion: 375,
        recoil: 55,
        count: 25,
        casing: None,
        effects: vec!["SHOT".to_string(), "BOUNCE".to_string()],
        stack_size: 20,
    },));
    let ammo = test.get::<AmmoData>(e).unwrap();
    assert!(ammo.casing.is_none());
}

#[test]
fn ammo_shotgun() {
    let mut test = TestBed::new();
    test.register::<AmmoData>();
    let e = test.spawn((AmmoData {
        ammo_type: AmmoTypeRegistry::default().intern("shot"),
        damage: 70,
        pierce: 0,
        range: 6,
        dispersion: 375,
        recoil: 55,
        count: 25,
        casing: None,
        effects: vec!["SHOT".to_string(), "BOUNCE".to_string()],
        stack_size: 20,
    },));
    let ammo = test.get::<AmmoData>(e).unwrap();
    assert_eq!(ammo.ammo_type, AmmoTypeRegistry::default().intern("shot"));
    assert_eq!(ammo.damage, 70);
    assert_eq!(ammo.pierce, 0);
    assert_eq!(ammo.range, 6);
    assert_eq!(ammo.dispersion, 375);
    assert_eq!(ammo.effects, vec!["SHOT", "BOUNCE"]);
}

#[test]
fn ammo_high_pierce() {
    let mut test = TestBed::new();
    test.register::<AmmoData>();
    let e = test.spawn((AmmoData {
        ammo_type: AmmoTypeRegistry::default().intern("rifle"),
        damage: 40,
        pierce: 15,
        range: 60,
        dispersion: 50,
        recoil: 30,
        count: 20,
        casing: Some("rifle_casing".to_string()),
        effects: vec![],
        stack_size: 20,
    },));
    let ammo = test.get::<AmmoData>(e).unwrap();
    assert_eq!(ammo.pierce, 15);
    assert_eq!(ammo.range, 60);
}

#[test]
fn ammo_effects_empty() {
    let mut test = TestBed::new();
    test.register::<AmmoData>();
    let e = test.spawn((AmmoData {
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        damage: 18,
        pierce: 0,
        range: 14,
        dispersion: 180,
        recoil: 18,
        count: 50,
        casing: Some("9mm_casing".to_string()),
        effects: vec![],
        stack_size: 50,
    },));
    let ammo = test.get::<AmmoData>(e).unwrap();
    assert!(ammo.effects.is_empty());
}

#[test]
fn ammo_incendiary() {
    let mut test = TestBed::new();
    test.register::<AmmoData>();
    let e = test.spawn((AmmoData {
        ammo_type: AmmoTypeRegistry::default().intern("rifle"),
        damage: 35,
        pierce: 5,
        range: 40,
        dispersion: 100,
        recoil: 25,
        count: 30,
        casing: Some("rifle_casing".to_string()),
        effects: vec!["INCENDIARY".to_string()],
        stack_size: 30,
    },));
    let ammo = test.get::<AmmoData>(e).unwrap();
    assert!(ammo.effects.contains(&"INCENDIARY".to_string()));
}

// ---------------------------------------------------------------------------
// GunData component tests
// ---------------------------------------------------------------------------

#[test]
fn gun_data_fields() {
    let mut test = TestBed::new();
    test.register::<GunData>();
    let e = test.spawn((GunData {
        skill: SkillId(0),
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        dispersion: 400,
        recoil: 45,
        reload_time: 100,
        clip_size: 15,
        burst: 1,
        ammo_effects: vec![],
    },));
    let gun = test.get::<GunData>(e).unwrap();
    assert_eq!(gun.skill, SkillId(0));
    assert_eq!(gun.ammo_type, AmmoTypeRegistry::default().intern("9mm"));
    assert_eq!(gun.dispersion, 400);
    assert_eq!(gun.recoil, 45);
    assert_eq!(gun.reload_time, 100);
    assert_eq!(gun.clip_size, 15);
    assert_eq!(gun.burst, 1);
    assert!(gun.ammo_effects.is_empty());
}

#[test]
fn gun_shotgun() {
    let mut test = TestBed::new();
    test.register::<GunData>();
    let e = test.spawn((GunData {
        skill: SkillId(0),
        ammo_type: AmmoTypeRegistry::default().intern("shot"),
        dispersion: 525,
        recoil: 60,
        reload_time: 150,
        clip_size: 6,
        burst: 1,
        ammo_effects: vec![],
    },));
    let gun = test.get::<GunData>(e).unwrap();
    assert_eq!(gun.skill, SkillId(0));
    assert_eq!(gun.ammo_type, AmmoTypeRegistry::default().intern("shot"));
    assert_eq!(gun.dispersion, 525);
    assert_eq!(gun.clip_size, 6);
    assert_eq!(gun.burst, 1);
}

#[test]
fn gun_burst_fire() {
    let mut test = TestBed::new();
    test.register::<GunData>();
    let e = test.spawn((GunData {
        skill: SkillId(0),
        ammo_type: AmmoTypeRegistry::default().intern("rifle"),
        dispersion: 200,
        recoil: 35,
        reload_time: 150,
        clip_size: 30,
        burst: 3,
        ammo_effects: vec![],
    },));
    let gun = test.get::<GunData>(e).unwrap();
    assert_eq!(gun.burst, 3);
}

#[test]
fn gun_no_ammo_effects() {
    let mut test = TestBed::new();
    test.register::<GunData>();
    let e = test.spawn((GunData {
        skill: SkillId(0),
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        dispersion: 400,
        recoil: 45,
        reload_time: 100,
        clip_size: 15,
        burst: 1,
        ammo_effects: vec![],
    },));
    let gun = test.get::<GunData>(e).unwrap();
    assert!(gun.ammo_effects.is_empty());
}

#[test]
fn gun_with_ammo_effects() {
    let mut test = TestBed::new();
    test.register::<GunData>();
    let e = test.spawn((GunData {
        skill: SkillId(0),
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        dispersion: 400,
        recoil: 45,
        reload_time: 100,
        clip_size: 15,
        burst: 1,
        ammo_effects: vec!["NEVER_MISFIRES".to_string()],
    },));
    let gun = test.get::<GunData>(e).unwrap();
    assert_eq!(gun.ammo_effects, vec!["NEVER_MISFIRES"]);
}

#[test]
fn gun_zero_clip() {
    let mut test = TestBed::new();
    test.register::<GunData>();
    let e = test.spawn((GunData {
        skill: SkillId(0),
        ammo_type: AmmoTypeRegistry::default().intern("rifle"),
        dispersion: 150,
        recoil: 30,
        reload_time: 200,
        clip_size: 0,
        burst: 1,
        ammo_effects: vec![],
    },));
    let gun = test.get::<GunData>(e).unwrap();
    assert_eq!(gun.clip_size, 0);
}

// ---------------------------------------------------------------------------
// MagazineData component tests
// ---------------------------------------------------------------------------

#[test]
fn magazine_fields() {
    let mut test = TestBed::new();
    test.register::<MagazineData>();
    let e = test.spawn((MagazineData {
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        capacity: 15,
        reload_time: 100,
        linkage: None,
        default_ammo: "9mm".to_string(),
    },));
    let mag = test.get::<MagazineData>(e).unwrap();
    assert_eq!(mag.ammo_type, AmmoTypeRegistry::default().intern("9mm"));
    assert_eq!(mag.capacity, 15);
    assert_eq!(mag.reload_time, 100);
    assert!(mag.linkage.is_none());
    assert_eq!(mag.default_ammo, "9mm");
}

#[test]
fn magazine_belt_linkage() {
    let mut test = TestBed::new();
    test.register::<MagazineData>();
    let e = test.spawn((MagazineData {
        ammo_type: AmmoTypeRegistry::default().intern("rifle"),
        capacity: 200,
        reload_time: 300,
        linkage: Some("belt_link".to_string()),
        default_ammo: "rifle".to_string(),
    },));
    let mag = test.get::<MagazineData>(e).unwrap();
    assert_eq!(mag.linkage, Some("belt_link".to_string()));
}

#[test]
fn magazine_high_capacity() {
    let mut test = TestBed::new();
    test.register::<MagazineData>();
    let e = test.spawn((MagazineData {
        ammo_type: AmmoTypeRegistry::default().intern("9mm"),
        capacity: 50,
        reload_time: 200,
        linkage: None,
        default_ammo: "9mm".to_string(),
    },));
    let mag = test.get::<MagazineData>(e).unwrap();
    assert_eq!(mag.capacity, 50);
}

// ---------------------------------------------------------------------------
// Pure function formula tests
// ---------------------------------------------------------------------------

/// CDDA-style effective range based on weapon dispersion and ammo range.
fn effective_range(ammo_range: i32, gun_dispersion: i32) -> f64 {
    let accuracy_factor = (100.0 - (gun_dispersion as f64 * 0.1)).max(0.0) / 100.0;
    ammo_range as f64 * accuracy_factor
}

/// Damage after range falloff. Damage drops linearly from full at
/// 0 to half at effective range, to 1/4 at 2× effective range.
fn range_damage_falloff(base_damage: i32, distance: i32, effective_range: f64) -> i32 {
    if distance as f64 <= effective_range {
        base_damage
    } else if distance as f64 <= effective_range * 2.0 {
        (base_damage as f64 * 0.5).round() as i32
    } else {
        (base_damage as f64 * 0.25).round() as i32
    }
}

/// Armor penetration effectiveness.
/// If pierce >= armor, full damage passes. Otherwise, damage is reduced.
fn armor_penetration(base_damage: i32, pierce: i32, armor: i32) -> i32 {
    if pierce >= armor {
        base_damage
    } else {
        (base_damage - (armor - pierce)).max(0)
    }
}

/// Recoil accumulates per shot fired, reducing accuracy for follow-up shots.
fn recoil_accumulation(base_recoil: i32, shots_fired: u32) -> i32 {
    (base_recoil as f64 * (shots_fired as f64 * 0.5)).round() as i32
}

/// Magazine reload time depends on current ammo count vs capacity.
fn reload_time_percent(remaining: i32, capacity: i32, base_time: i32) -> i32 {
    let empty_pct = (capacity - remaining) as f64 / capacity as f64;
    (base_time as f64 * empty_pct).round() as i32
}

#[test]
fn effective_range_normal() {
    // range=14, dispersion=200 → 14 * (100 - 20) / 100 = 11.2
    let result = effective_range(14, 200);
    assert!((result - 11.2).abs() < 0.001);
}

#[test]
fn range_damage_falloff_test() {
    // effective range ≈ 11.2 with ammo range=14, dispersion=200
    let effective = effective_range(14, 200);
    // distance 0 (within effective) → full damage 40
    assert_eq!(range_damage_falloff(40, 0, effective), 40);
    // distance 12 (between 1× and 2× effective range) → half damage 20
    assert_eq!(range_damage_falloff(40, 12, effective), 20);
    // distance 25 (beyond 2× effective) → quarter damage 10
    assert_eq!(range_damage_falloff(40, 25, effective), 10);
}

#[test]
fn armor_penetration_test() {
    // 30 damage, pierce=5, armor=10 → 30 - (10 - 5) = 25
    assert_eq!(armor_penetration(30, 5, 10), 25);
}

#[test]
fn recoil_accumulation_test() {
    // 45 base, 3 shots → 45 × (3 × 0.5) = 45 × 1.5 = 67.5 → rounds to 68
    assert_eq!(recoil_accumulation(45, 3), 68);
}

#[test]
fn reload_time_percent_test() {
    // 15 capacity, 5 remaining → 10 empty → 10/15 = 0.666…
    // base_time 100 → 100 × 0.666… = 66.666… → rounds to 67
    assert_eq!(reload_time_percent(5, 15, 100), 67);
}
