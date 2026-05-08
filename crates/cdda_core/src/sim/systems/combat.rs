//! Combat phase — resolve melee and ranged combat actions.
//!
//! Emits DamageEvent, DeathEvent, and SoundEvent.

use crate::actor::components::*;
use crate::coords::WorldPos;
use crate::sim::def_components::{AmmoData, GunData, WeaponData};
use crate::{Damage, Stats};
use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The result of a single combat action (melee or ranged).
#[derive(Debug, Clone)]
pub struct CombatResult {
    /// Whether the attack connected.
    pub hit: bool,
    /// Damage dealt after armour mitigation.
    pub damage: Damage,
    /// Which body part was hit (if applicable).
    pub body_part_hit: Option<Entity>,
    /// Whether the hit was a critical.
    pub critical: bool,
}

/// An intent to perform a melee attack.
#[derive(Debug, Clone)]
pub struct MeleeIntent {
    pub attacker: Entity,
    pub defender: Entity,
    pub weapon: Option<Entity>,
}

// ---------------------------------------------------------------------------
// Formulas
// ---------------------------------------------------------------------------

/// Calculate melee hit chance using attacker skill, weapon to-hit, and
/// defender dodge.
///
/// Formula (derived from CDDA `hit_roll()`):
/// ```ignore
/// hit = 0.5 + (skill * 0.04) + (weapon_to_hit * 0.01) - (dodge * 0.03)
/// ```
/// Clamped to [0.05, 0.95] to leave room for luck/training.
pub fn calculate_melee_hit_chance(
    attacker_stats: &CombatStats,
    weapon_to_hit: i32,
    defender_dodge: i32,
) -> f32 {
    let base = 0.5;
    let skill_bonus = attacker_stats.melee_skill as f32 * 0.04;
    let weapon_bonus = weapon_to_hit as f32 * 0.01;
    let dodge_penalty = defender_dodge as f32 * 0.03;
    (base + skill_bonus + weapon_bonus - dodge_penalty).clamp(0.05, 0.95)
}

/// Calculate raw melee damage from weapon stats and creature strength.
///
/// Formula (derived from CDDA `deal_melee_attack`):
/// - Sum weapon's fixed damage (bash + cut + stab)
/// - Add dice-based damage: dice * (dice_sides + 1) / 2 (average roll)
/// - Add stat bonus: strength * 0.5 as bash
/// - Add skill bonus: skill_level * 0.25 as bash
pub fn calculate_melee_damage(weapon: &WeaponData, stats: &Stats, skill_level: u32) -> Damage {
    use crate::id::{DamageTypeId, DefIdx};

    let bash_type = DamageTypeId(DefIdx(0));
    let cut_type = DamageTypeId(DefIdx(1));
    let stab_type = DamageTypeId(DefIdx(2));

    let mut dmg = Damage::ZERO;

    // Fixed damage from weapon
    if weapon.damage_bash > 0 {
        dmg.add(bash_type, weapon.damage_bash as u32);
    }
    if weapon.damage_cut > 0 {
        dmg.add(cut_type, weapon.damage_cut as u32);
    }
    if weapon.damage_stab > 0 {
        dmg.add(stab_type, weapon.damage_stab as u32);
    }

    // Dice-based damage (average roll, added as bash)
    let avg_die = (weapon.dice_sides as f32 + 1.0) / 2.0;
    let dice_damage = (weapon.dice as f32 * avg_die).round() as u32;
    if dice_damage > 0 {
        dmg.add(bash_type, dice_damage);
    }

    // Stat bonus (strength → bash)
    let stat_bonus = (stats.strength as f32 * 0.5).round() as u32;
    if stat_bonus > 0 {
        dmg.add(bash_type, stat_bonus);
    }

    // Skill bonus
    let skill_bonus = (skill_level as f32 * 0.25).round() as u32;
    if skill_bonus > 0 {
        dmg.add(bash_type, skill_bonus);
    }

    dmg
}

/// Apply damage to a target, reducing by armour, distributing across
/// body parts. Returns the actual damage dealt after mitigation.
pub fn apply_damage_to_target(
    world: &mut World,
    target: Entity,
    damage: &Damage,
    armor: &DamageReduction,
) -> Damage {
    let _ = (world, target, damage, armor);
    todo!("damage application with armour: subtract armour per type, distribute to BodyPartHp")
}

/// Check if an entity's HP has reached 0. If so, remove `IsAlive`,
/// emit `DeathEvent`, and return `true`.
pub fn check_and_handle_death(world: &mut World, entity: Entity) -> bool {
    let _ = (world, entity);
    todo!("death check: query Health, if <= 0 remove IsAlive, emit DeathEvent")
}

// ---------------------------------------------------------------------------
// Attack resolution
// ---------------------------------------------------------------------------

/// Resolve a full melee attack: hit check, damage calc, armour,
/// death check.
pub fn resolve_melee_attack(world: &mut World, attacker: Entity, defender: Entity) -> CombatResult {
    let _ = (world, attacker, defender);
    todo!("full melee attack resolution: hit chance → damage → armour → death")
}

/// Calculate ranged hit chance factoring gun accuracy, ammo,
/// distance, and shooter skill.
///
/// Formula (derived from CDDA `ranged_attack`):
/// - Base accuracy = 1.0
/// - Gun dispersion reduces accuracy (10 dispersion = 0.99)
/// - Ammo dispersion also penalizes
/// - Distance penalty: accuracy drops linearly beyond ideal range
/// - Skill bonus: each skill point adds 0.02 to hit
/// Returns value in [0.05, 0.98].
pub fn calculate_ranged_hit_chance(
    gun: &GunData,
    ammo: &AmmoData,
    distance: f64,
    shooter_skill: i32,
) -> f32 {
    // Gun accuracy: 1.0 - dispersion / 1000
    let gun_factor = (1.0 - gun.dispersion as f64 / 1000.0).max(0.1);
    // Ammo factor: 1.0 - dispersion / 1000
    let ammo_factor = (1.0 - ammo.dispersion as f64 / 1000.0).max(0.1);
    // Distance penalty: at ammo.range, accuracy starts dropping
    let range_factor = if distance <= ammo.range as f64 {
        1.0
    } else {
        let over = distance - ammo.range as f64;
        (1.0 - over / ammo.range as f64).max(0.05)
    };
    // Skill bonus
    let skill_factor = 1.0 + shooter_skill as f64 * 0.02;

    let hit = gun_factor * ammo_factor * range_factor * skill_factor;
    (hit as f32).clamp(0.05, 0.98)
}

/// Resolve a full ranged attack: trajectory, hit check, damage
/// application, death check.
pub fn resolve_ranged_attack(
    world: &mut World,
    attacker: Entity,
    target_pos: WorldPos,
    weapon: Entity,
    ammo: Entity,
) -> CombatResult {
    let _ = (world, attacker, target_pos, weapon, ammo);
    todo!("full ranged attack resolution: projectile travel → hit → damage → death")
}

// ---------------------------------------------------------------------------
// Phase orchestrators
// ---------------------------------------------------------------------------

/// Process all melee attack intents for this tick.
///
/// Iterates queued `MeleeIntent`s, calls `resolve_melee_attack` for
/// each, deducts action costs.
pub fn melee_combat_phase(world: &mut World) {
    let _ = world;
    todo!("melee combat phase not yet implemented")
}

/// Process all ranged attack intents for this tick.
pub fn ranged_combat_phase(world: &mut World) {
    let _ = world;
    todo!("ranged combat phase not yet implemented")
}

/// Run both melee and ranged combat sub-phases.
pub fn combat_phase(world: &mut World) {
    melee_combat_phase(world);
    ranged_combat_phase(world);
}
