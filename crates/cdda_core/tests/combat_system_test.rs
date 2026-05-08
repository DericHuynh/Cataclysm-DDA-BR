//! Combat system tests — integration tests for the combat phase.
//!
//! Each test calls `combat_phase` and asserts post-conditions that the
//! stub implementation does not satisfy, causing deliberate failure.
//!
//! All tests are `#[ignore = "combat system not yet implemented"]`.

#![allow(unused_imports)]

use bevy_ecs::prelude::*;
use cdda_core::core::components::actor::*;
use cdda_core::core::components::sim::*;
use cdda_core::core::components::def::*;
use cdda_core::sim::systems::combat::*;
use cdda_core::sim::test_utils::TestBed;
use cdda_core::{Damage, DamageTypeId, DefIdx};

// ---------------------------------------------------------------------------
// Hit chance tests
// ---------------------------------------------------------------------------

#[test]
#[ignore = "combat system not yet implemented"]
fn hit_chance_unskilled_vs_unarmored() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let attacker = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let defender = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Stub does not compute hit chance — the factory-default hit probability
    // should be 0.5 when skill and dodge are both zero, but the phase does
    // nothing, so this assertion fails.
    let health = test.get::<Health>(defender).unwrap();
    assert!(
        health.current < 100,
        "unskilled attacker should land some hits at point blank"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn hit_chance_skilled_vs_unskilled() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let attacker = test.spawn((
        CombatStats {
            melee_skill: 10,
            melee_dice: 3,
            melee_dice_sides: 6,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let defender = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Skilled attacker (10) vs zero-dodge defender → ~0.95 hit chance → damage likely.
    // Stub applies no damage, so this fails.
    let health = test.get::<Health>(defender).unwrap();
    assert!(
        health.current < 95,
        "highly skilled attacker should hit nearly every swing"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn hit_chance_dodge_reduces() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let attacker = test.spawn((
        CombatStats {
            melee_skill: 5,
            melee_dice: 2,
            melee_dice_sides: 6,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let defender = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 5,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Equal skill (5) vs dodge (5) → ~0.5 hit chance → roughly half the
    // damage of a zero-dodge target.  Stub does nothing → fails.
    let health = test.get::<Health>(defender).unwrap();
    assert!(
        health.current < 100,
        "high dodge should reduce incoming damage compared to no dodge"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn hit_chance_weapon_bonus() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<WeaponData>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let weapon = test.spawn(WeaponData {
        damage_bash: 5,
        damage_cut: 0,
        damage_stab: 0,
        to_hit: 3,
        moves_per_attack: 100,
        reach: 1,
        techniques: Vec::new(),
        dice: 1,
        dice_sides: 6,
        skill: "bashing".to_string(),
    });
    let attacker = test.spawn((
        CombatStats {
            melee_skill: 5,
            melee_dice: 2,
            melee_dice_sides: 6,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let defender = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 5,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Weapon with +3 to-hit should improve hit chance over bare hands.
    // Stub does nothing — fails.
    let health = test.get::<Health>(defender).unwrap();
    assert!(
        health.current < 100,
        "weapon to-hit bonus should improve hit chance against a dodging target"
    );
}

// ---------------------------------------------------------------------------
// Melee damage calculation
// ---------------------------------------------------------------------------

#[test]
#[ignore = "combat system not yet implemented"]
fn melee_damage_bare_hands() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<CombatStats>();

    let attacker = test.spawn((
        Stats::new(8, 8, 8, 8),
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let defender = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Bare-hand damage with 8 STR should produce some calculated damage.
    // Stub does nothing — fails.
    let health = test.get::<Health>(defender).unwrap();
    assert!(
        health.current < 100,
        "bare hands with 8 STR should deal at least minimal damage"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn melee_damage_with_weapon() {
    let mut test = TestBed::new();
    test.register::<WeaponData>();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<CombatStats>();

    let weapon = test.spawn(WeaponData {
        damage_bash: 10,
        damage_cut: 5,
        damage_stab: 0,
        to_hit: 1,
        moves_per_attack: 100,
        reach: 1,
        techniques: Vec::new(),
        dice: 1,
        dice_sides: 6,
        skill: "bashing".to_string(),
    });
    let attacker = test.spawn((
        Stats::new(10, 8, 8, 8),
        CombatStats {
            melee_skill: 5,
            melee_dice: 2,
            melee_dice_sides: 6,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let defender = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Weapon with bash=10 + cut=5 should contribute both damage types.
    // Stub does nothing — fails.
    let health = test.get::<Health>(defender).unwrap();
    assert!(
        health.current < 100,
        "weapon damage should be reflected in total damage dealt"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn melee_damage_skill_bonus() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<CombatStats>();

    let high_skill = test.spawn((
        Stats::new(8, 8, 8, 8),
        CombatStats {
            melee_skill: 10,
            melee_dice: 3,
            melee_dice_sides: 8,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let low_skill = test.spawn((
        Stats::new(8, 8, 8, 8),
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let dummy = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Higher skill → more damage. Stub does nothing → both still at 100.
    let skill_health = test.get::<Health>(low_skill).unwrap();
    assert!(
        skill_health.current < 100,
        "higher melee skill should increase damage output"
    );
}

// ---------------------------------------------------------------------------
// Damage application
// ---------------------------------------------------------------------------

#[test]
#[ignore = "combat system not yet implemented"]
fn apply_damage_reduces_health() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let victim = test.spawn((
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // The combat phase should apply 30 damage to the victim → health = 70.
    // Stub does nothing → stays at 100 → fails.
    let health = test.get::<Health>(victim).unwrap();
    assert_eq!(
        health.current, 70,
        "applying 30 damage to a 100 HP creature should leave 70 HP"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn apply_damage_armor_reduces() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let armored = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 5,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let attacker = test.spawn((
        CombatStats {
            melee_skill: 5,
            melee_dice: 2,
            melee_dice_sides: 6,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // 30 bash damage - 5 armor = 25 net → health = 75.
    // Stub does nothing → stays at 100 → fails.
    let health = test.get::<Health>(armored).unwrap();
    assert!(health.current > 70, "armor should reduce incoming damage");
}

// ---------------------------------------------------------------------------
// Death
// ---------------------------------------------------------------------------

#[test]
#[ignore = "combat system not yet implemented"]
fn death_at_zero_health() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let victim = test.spawn((
        Health {
            current: 0,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Creature with 0 HP should be flagged as dead — IsAlive removed.
    // Stub does nothing → IsAlive still present → fails.
    let alive = test.world().entity(victim).contains::<IsAlive>();
    assert!(!alive, "creature with 0 HP should have IsAlive removed");
}

#[test]
#[ignore = "combat system not yet implemented"]
fn death_removes_IsAlive() {
    let mut test = TestBed::new();
    test.register::<Health>();
    test.register::<IsAlive>();

    let victim = test.spawn((
        Health {
            current: 0,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // After death processing, IsAlive should be gone.
    // Stub does nothing → IsAlive is still present → fails.
    assert!(
        !test.world().entity(victim).contains::<IsAlive>(),
        "IsAlive must be removed when health reaches zero"
    );
}

// ---------------------------------------------------------------------------
// Ranged hit
// ---------------------------------------------------------------------------

#[test]
#[ignore = "combat system not yet implemented"]
fn ranged_hit_short_range() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<GunData>();
    test.register::<AmmoData>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let gun = test.spawn(GunData {
        skill: "rifle".to_string(),
        ammo_type: "762".to_string(),
        dispersion: 0,
        recoil: 30,
        reload_time: 100,
        clip_size: 10,
        burst: 1,
        ammo_effects: Vec::new(),
    });
    let ammo = test.spawn(AmmoData {
        ammo_type: "762".to_string(),
        damage: 20,
        pierce: 5,
        range: 30,
        dispersion: 0,
        recoil: 10,
        count: 30,
        casing: None,
        effects: Vec::new(),
        stack_size: 30,
    });
    let shooter = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let target = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Point-blank shot should hit nearly 100% of the time.
    // Stub does nothing — target still at 100 HP → fails.
    let health = test.get::<Health>(target).unwrap();
    assert!(
        health.current < 100,
        "point-blank ranged shot should almost always hit"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn ranged_hit_long_range() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<GunData>();
    test.register::<AmmoData>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let gun = test.spawn(GunData {
        skill: "rifle".to_string(),
        ammo_type: "223".to_string(),
        dispersion: 10,
        recoil: 15,
        reload_time: 100,
        clip_size: 30,
        burst: 1,
        ammo_effects: Vec::new(),
    });
    let ammo = test.spawn(AmmoData {
        ammo_type: "223".to_string(),
        damage: 15,
        pierce: 2,
        range: 5,
        dispersion: 5,
        recoil: 5,
        count: 30,
        casing: None,
        effects: Vec::new(),
        stack_size: 30,
    });
    let shooter = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let target = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // Beyond effective range → low hit probability → less damage.
    // Stub does nothing → still at 100 → fails.
    let health = test.get::<Health>(target).unwrap();
    assert!(
        health.current < 100,
        "long-range shot should have reduced hit probability"
    );
}

#[test]
#[ignore = "combat system not yet implemented"]
fn ranged_hit_high_dispersion() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<GunData>();
    test.register::<AmmoData>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let gun = test.spawn(GunData {
        skill: "pistol".to_string(),
        ammo_type: "9mm".to_string(),
        dispersion: 50,
        recoil: 40,
        reload_time: 50,
        clip_size: 15,
        burst: 1,
        ammo_effects: Vec::new(),
    });
    let ammo = test.spawn(AmmoData {
        ammo_type: "9mm".to_string(),
        damage: 12,
        pierce: 0,
        range: 12,
        dispersion: 10,
        recoil: 5,
        count: 15,
        casing: None,
        effects: Vec::new(),
        stack_size: 15,
    });
    let shooter = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));
    let target = test.spawn((
        CombatStats {
            melee_skill: 0,
            melee_dice: 1,
            melee_dice_sides: 1,
            dodge: 0,
            armor: DamageReduction {
                bash: 0,
                cut: 0,
                pierce: 0,
                bullet: 0,
                fire: 0,
                acid: 0,
                electric: 0,
                cold: 0,
            },
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ));

    test.run_system(combat_phase);

    // High dispersion gun → lower hit chance.
    // Stub does nothing → fails.
    let health = test.get::<Health>(target).unwrap();
    assert!(
        health.current < 100,
        "high dispersion should reduce accuracy"
    );
}

// ---------------------------------------------------------------------------
// Combat phase processes all intents
// ---------------------------------------------------------------------------

#[test]
#[ignore = "combat system not yet implemented"]
fn melee_combat_phase_processes_all() {
    let mut test = TestBed::new();
    test.register::<CombatStats>();
    test.register::<Health>();
    test.register::<IsAlive>();

    let mut entities = Vec::new();
    for _ in 0..5 {
        let e = test.spawn((
            CombatStats {
                melee_skill: 5,
                melee_dice: 2,
                melee_dice_sides: 6,
                dodge: 2,
                armor: DamageReduction {
                    bash: 0,
                    cut: 0,
                    pierce: 0,
                    bullet: 0,
                    fire: 0,
                    acid: 0,
                    electric: 0,
                    cold: 0,
                },
            },
            Health {
                current: 100,
                max: 100,
            },
            IsAlive,
        ));
        entities.push(e);
    }

    test.run_system(combat_phase);

    // After phase processes all intents, at least some creatures should
    // have taken damage.  Stub does nothing → all still at 100 → fails.
    let any_damaged = entities
        .iter()
        .any(|e| test.get::<Health>(*e).map_or(false, |h| h.current < 100));
    assert!(
        any_damaged,
        "combat phase should process all attackers and deal damage"
    );
}
