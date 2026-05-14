//! Combat mechanics tests — damage calculations, hit/miss probability, armor mitigation.
//!
//! Tests `CombatStats`, `DamageReduction`, `Vision`, and `Creature` components,
//! along with combat formula helpers derived from CDDA rules.

use cdda_components::Damage;
use cdda_core_types::core::damage::DamageTypeDef;
use cdda_core_types::core::DefId;
use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Combat formula helpers (pure functions, not in the library)
// ---------------------------------------------------------------------------

/// Calculate melee hit probability from `CombatStats`.
///
/// Formula: `(melee_skill * 0.1) + (melee_dice * melee_dice_sides * 0.01)`
fn melee_to_hit(stats: &cdda_components::actor::CombatStats) -> f32 {
    (stats.melee_skill as f32 * 0.1)
        + (stats.melee_dice as f32 * stats.melee_dice_sides as f32 * 0.01)
}

/// Calculate dodge chance from dodge skill.
///
/// Formula: `dodge * 0.05`
fn dodge_chance(dodge: i32) -> f32 {
    dodge as f32 * 0.05
}

/// Apply flat armor reduction to raw damage, clamping to zero.
fn apply_armor(raw_damage: u32, armor: u32) -> u32 {
    raw_damage.saturating_sub(armor)
}

// ---------------------------------------------------------------------------
// CombatStats
// ---------------------------------------------------------------------------

#[test]
fn combat_stats_initialized() {
    let mut test = TestBed::new();
    test.register::<cdda_components::actor::CombatStats>();

    let e = test.spawn((cdda_components::actor::CombatStats {
        melee_skill: 5,
        melee_dice: 2,
        melee_dice_sides: 6,
        dodge: 2,
        armor: cdda_components::actor::DamageReduction {
            bash: 0,
            cut: 0,
            pierce: 0,
            bullet: 0,
            fire: 0,
            acid: 0,
            electric: 0,
            cold: 0,
        },
    },));
    let stats = test.get::<cdda_components::actor::CombatStats>(e).unwrap();
    assert_eq!(stats.melee_skill, 5);
    assert_eq!(stats.melee_dice, 2);
    assert_eq!(stats.melee_dice_sides, 6);
    assert_eq!(stats.dodge, 2);
}

#[test]
fn combat_stats_zero_skill() {
    let mut test = TestBed::new();
    test.register::<cdda_components::actor::CombatStats>();

    let e = test.spawn((cdda_components::actor::CombatStats {
        melee_skill: 0,
        melee_dice: 1,
        melee_dice_sides: 1,
        dodge: 0,
        armor: cdda_components::actor::DamageReduction {
            bash: 0,
            cut: 0,
            pierce: 0,
            bullet: 0,
            fire: 0,
            acid: 0,
            electric: 0,
            cold: 0,
        },
    },));
    let stats = test.get::<cdda_components::actor::CombatStats>(e).unwrap();
    assert_eq!(stats.melee_skill, 0);
    assert_eq!(stats.dodge, 0);
}

// ---------------------------------------------------------------------------
// DamageReduction
// ---------------------------------------------------------------------------

#[test]
fn damage_reduction_bash() {
    let armor = cdda_components::actor::DamageReduction {
        bash: 5,
        cut: 0,
        pierce: 0,
        bullet: 0,
        fire: 0,
        acid: 0,
        electric: 0,
        cold: 0,
    };
    assert_eq!(armor.bash, 5);
    assert_eq!(armor.cut, 0);
}

#[test]
fn damage_reduction_multiple_types() {
    let armor = cdda_components::actor::DamageReduction {
        bash: 3,
        cut: 7,
        bullet: 12,
        pierce: 0,
        fire: 0,
        acid: 0,
        electric: 0,
        cold: 0,
    };
    assert_eq!(armor.bash, 3);
    assert_eq!(armor.cut, 7);
    assert_eq!(armor.bullet, 12);
    assert_eq!(armor.pierce, 0);
    assert_eq!(armor.fire, 0);
}

// ---------------------------------------------------------------------------
// Vision
// ---------------------------------------------------------------------------

#[test]
fn vision_range() {
    let mut test = TestBed::new();
    test.register::<cdda_components::actor::Vision>();

    let e = test.spawn((cdda_components::actor::Vision {
        day_range: 40,
        night_range: 5,
    },));
    let vision = test.get::<cdda_components::actor::Vision>(e).unwrap();
    assert_eq!(vision.day_range, 40);
    assert_eq!(vision.night_range, 5);
}

#[test]
fn vision_no_night() {
    let mut test = TestBed::new();
    test.register::<cdda_components::actor::Vision>();

    let e = test.spawn((cdda_components::actor::Vision {
        day_range: 40,
        night_range: 0,
    },));
    let vision = test.get::<cdda_components::actor::Vision>(e).unwrap();
    assert_eq!(vision.day_range, 40);
    assert_eq!(vision.night_range, 0);
}

// ---------------------------------------------------------------------------
// Creature identity
// ---------------------------------------------------------------------------

#[test]
fn creature_identity() {
    let mut test = TestBed::new();
    test.register::<cdda_components::actor::Creature>();

    let e = test.spawn((cdda_components::actor::Creature {
        def_id: "mon_zombie".to_string(),
        name: "zombie".to_string(),
        species: cdda_components::SpeciesId::from(0u32),
        symbol: 'Z',
    },));
    let creature = test.get::<cdda_components::actor::Creature>(e).unwrap();
    assert_eq!(creature.def_id, "mon_zombie");
    assert_eq!(creature.name, "zombie");
    assert_eq!(creature.symbol, 'Z');
}

// ---------------------------------------------------------------------------
// Melee hit calculation
// ---------------------------------------------------------------------------

#[test]
fn melee_to_hit_calculation() {
    // skill=0, dice=1, sides=6 => 0.06
    let stats = cdda_components::actor::CombatStats {
        melee_skill: 0,
        melee_dice: 1,
        melee_dice_sides: 6,
        dodge: 0,
        armor: cdda_components::actor::DamageReduction {
            bash: 0,
            cut: 0,
            pierce: 0,
            bullet: 0,
            fire: 0,
            acid: 0,
            electric: 0,
            cold: 0,
        },
    };
    let hit = melee_to_hit(&stats);
    assert!((hit - 0.06).abs() < f32::EPSILON);

    // skill=5, dice=2, sides=6 => 0.62
    let stats = cdda_components::actor::CombatStats {
        melee_skill: 5,
        melee_dice: 2,
        melee_dice_sides: 6,
        dodge: 0,
        armor: cdda_components::actor::DamageReduction {
            bash: 0,
            cut: 0,
            pierce: 0,
            bullet: 0,
            fire: 0,
            acid: 0,
            electric: 0,
            cold: 0,
        },
    };
    let hit = melee_to_hit(&stats);
    assert!((hit - 0.62).abs() < f32::EPSILON);

    // skill=10, dice=3, sides=8 => 1.24
    let stats = cdda_components::actor::CombatStats {
        melee_skill: 10,
        melee_dice: 3,
        melee_dice_sides: 8,
        dodge: 0,
        armor: cdda_components::actor::DamageReduction {
            bash: 0,
            cut: 0,
            pierce: 0,
            bullet: 0,
            fire: 0,
            acid: 0,
            electric: 0,
            cold: 0,
        },
    };
    let hit = melee_to_hit(&stats);
    assert!((hit - 1.24).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// Dodge calculation
// ---------------------------------------------------------------------------

#[test]
fn dodge_calculation() {
    // dodge=0 => 0.0
    assert!((dodge_chance(0) - 0.0).abs() < f32::EPSILON);
    // dodge=2 => 0.1
    assert!((dodge_chance(2) - 0.1).abs() < f32::EPSILON);
    // dodge=10 => 0.5
    assert!((dodge_chance(10) - 0.5).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// Armor mitigation
// ---------------------------------------------------------------------------

#[test]
fn armor_reduces_damage() {
    // 30 damage with 5 armor => 25 net
    assert_eq!(apply_armor(30, 5), 25);
    // 10 damage with 20 armor => 0 net (clamped)
    assert_eq!(apply_armor(10, 20), 0);
}

// ---------------------------------------------------------------------------
// Damage profile
// ---------------------------------------------------------------------------

#[test]
fn damage_profile_tracks_types() {
    let bash = DefId::<DamageTypeDef>::new("bash");
    let cut = DefId::<DamageTypeDef>::new("cut");
    let bullet = DefId::<DamageTypeDef>::new("bullet");

    let mut d = Damage::ZERO;
    d.add(bash, 10);
    d.add(cut, 5);
    d.add(bullet, 3);

    assert_eq!(d.len(), 3);
    assert_eq!(d.total(), 18);
    assert_eq!(d.by_type(bash), 10);
    assert_eq!(d.by_type(cut), 5);
    assert_eq!(d.by_type(bullet), 3);
}

#[test]
fn damage_merge_combat() {
    let bash = DefId::<DamageTypeDef>::new("bash");
    let cut = DefId::<DamageTypeDef>::new("cut");

    let mut a = Damage::ZERO;
    a.add(bash, 12);

    let mut b = Damage::ZERO;
    b.add(cut, 4);
    b.add(bash, 3);

    a.merge(&b);
    assert_eq!(a.total(), 19);
    assert_eq!(a.by_type(bash), 15);
    assert_eq!(a.by_type(cut), 4);
}
