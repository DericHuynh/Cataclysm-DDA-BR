//! Morale system — temporary morale bonuses and their effects on stats.
//!
//! Morale bonuses are individual entities related to creatures via
//! `MoraleBonusOf`/`MoraleBonuses`. Each bonus has a reason, amount,
//! and remaining duration. Bonuses decay each turn and are removed
//! when expired. The net morale value affects combat stats (damage,
//! accuracy) and other gameplay systems.

use bevy_ecs::prelude::*;
use cdda_components::Time;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Add a temporary morale bonus to a creature.
///
/// Creates a new `MoraleBonus` entity with `MoraleBonusOf(creature)`
/// relationship. Returns the bonus entity ID.
pub fn add_morale_bonus(
    world: &mut World,
    creature: Entity,
    reason: String,
    amount: i32,
    duration: Time,
) -> Entity {
    let _ = (world, creature, reason, amount, duration);
    todo!("create MoraleBonus entity + MoraleBonusOf relationship")
}

/// Calculate the net morale value for a creature.
///
/// Sums the base `Morale` value and all active `MoraleBonus.amount`
/// values from the `MoraleBonuses` relationship.
pub fn calculate_morale(world: &World, entity: Entity) -> i32 {
    let _ = (world, entity);
    todo!("sum morale base + bonuses: iterate MoraleBonuses relationship")
}

/// Decay all morale bonuses and remove expired ones.
///
/// Decrements `MoraleBonus.remaining` by one turn. Bonuses with
/// `remaining <= 0` are despawned. Runs each turn after effects
/// and before bionics.
pub fn tick_morale_decay(world: &mut World) {
    let _ = world;
}

/// Apply morale modifiers to a creature's stats.
///
/// High morale grants combat bonuses (e.g. +damage, +accuracy).
/// Low morale (especially < -50) applies penalties.
/// Modifiers are applied to `CombatStats` directly.
pub fn apply_morale_effects(world: &mut World, entity: Entity) {
    let _ = (world, entity);
    todo!("apply morale modifiers to CombatStats: damage/accuracy multipliers")
}
