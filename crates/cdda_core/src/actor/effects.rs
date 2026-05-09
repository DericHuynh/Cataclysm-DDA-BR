//! Effects phase — apply status effects, decay, removal.
//!
//! Effects are individual entities related to creatures via
//! `EffectOn`/`ActiveEffects` (a Bevy relationship). Each effect has
//! a type (`EffectId`), intensity, and remaining duration. Decay and
//! removal happen here; actual effect logic (damage, stat mods) is
//! applied by other systems reading the `StatusEffect` components.

use bevy_ecs::prelude::*;
use crate::{EffectId, Time};
use crate::core::components::actor::*;


// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Apply a status effect to a target entity.
///
/// Creates a new `StatusEffect` entity with the given `effect_id`,
/// `intensity`, and `duration`, then inserts an `EffectOn(target)`
/// relationship component. Bevy hooks auto-populate `ActiveEffects`
/// on the target.
///
/// If the target already has an effect with the same `EffectId`, the
/// existing effect's intensity is increased and duration is refreshed
/// instead of creating a duplicate.
pub fn apply_effect(
    world: &mut World,
    target: Entity,
    effect_id: EffectId,
    intensity: u32,
    duration: Time,
) {
    // Check if target already has this effect — if so, upgrade it.
    // Collect entity IDs first so the immutable borrow on world is
    // released before we call world.get_mut below.
    let effect_entities: Vec<Entity> = if let Some(active) = world.get::<ActiveEffects>(target) {
        active.iter().collect()
    } else {
        Vec::new()
    };

    for effect_entity in effect_entities {
        if let Some(mut se) = world.get_mut::<StatusEffect>(effect_entity) {
            if se.effect_id == effect_id {
                se.intensity = se.intensity.saturating_add(intensity);
                se.remaining = se.remaining.max(duration);
                return;
            }
        }
    }

    // Spawn a new effect entity with EffectOn(target)
    // Bevy hooks auto-populate ActiveEffects on the target
    world.spawn((
        EffectOn(target),
        StatusEffect {
            effect_id,
            intensity,
            remaining: duration,
        },
    ));
}

/// Remove all instances of a specific effect from a target.
///
/// Despawns all `StatusEffect` entities on the target that match
/// the given `effect_id`. Bevy `linked_spawn` hooks auto-remove
/// the entity from `ActiveEffects` on despawn.
pub fn remove_effect(world: &mut World, target: Entity, effect_id: EffectId) {
    // Find all effect entities on target with matching effect_id
    let to_remove: Vec<Entity> = if let Some(active) = world.get::<ActiveEffects>(target) {
        active
            .iter()
            .filter(|&e| {
                world
                    .get::<StatusEffect>(e)
                    .map(|se| se.effect_id == effect_id)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        return;
    };

    for e in to_remove {
        world.despawn(e);
        // Bevy hooks auto-remove from ActiveEffects via linked_spawn
    }
}

/// Check whether a target has a specific effect active.
pub fn has_effect(world: &World, entity: Entity, effect_id: EffectId) -> bool {
    if let Some(active) = world.get::<ActiveEffects>(entity) {
        active.iter().any(|e| {
            world
                .get::<StatusEffect>(e)
                .map(|se| se.effect_id == effect_id)
                .unwrap_or(false)
        })
    } else {
        false
    }
}

/// Get the total intensity of a specific effect on a target.
///
/// If multiple instances of the same effect exist, their intensities
/// are summed.
pub fn get_effect_intensity(world: &World, entity: Entity, effect_id: EffectId) -> u32 {
    if let Some(active) = world.get::<ActiveEffects>(entity) {
        active
            .iter()
            .filter_map(|e| world.get::<StatusEffect>(e))
            .filter(|se| se.effect_id == effect_id)
            .map(|se| se.intensity)
            .sum()
    } else {
        0
    }
}

/// Decay all status effects by one tick and remove expired ones.
///
/// Decrements `StatusEffect.remaining` by 1 turn. Effects with
/// `remaining <= 0` are despawned. Uses a two-phase approach:
/// collect expired entities during iteration, then despawn after
/// the query borrow is released.
pub fn tick_effects(world: &mut World) {
    // Collect expired effects to remove (two-phase: read then mutate)
    let mut expired: Vec<Entity> = Vec::new();

    // Iterate all StatusEffect entities directly.
    // Using iter_mut avoids the borrow conflict between a query iter
    // (which borrows world) and world.get_mut (which also borrows world).
    let mut q = world.query::<(Entity, &mut StatusEffect)>();
    for (entity, mut se) in q.iter_mut(world) {
        se.remaining = se.remaining - Time::from_turns(1);
        if se.remaining <= Time::ZERO {
            expired.push(entity);
        }
    }

    // Mutate phase: despawn expired (query borrow is released)
    for e in expired {
        world.despawn(e);
    }
}

// ---------------------------------------------------------------------------
// Phase orchestrator
// ---------------------------------------------------------------------------

/// Run one tick of the effects system.
pub fn effects_phase(world: &mut World) {
    tick_effects(world);
}
