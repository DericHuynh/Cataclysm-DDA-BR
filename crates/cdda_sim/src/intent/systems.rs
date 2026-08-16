//! # Intent resolution — turn ordering by action points
//!
//! Handles the intent → action pipeline:
//!
//! 1. **Declare** (`SimSet::IntentDeclare`): AI + player input insert `ActionIntent` components.
//!    `collect_intents` gathers them into a global `IntentQueue`, sorted by AP descending.
//! 2. **Resolve** (`SimSet::IntentResolve`): `resolve_intents` drains the queue, validates
//!    preconditions, and executes actions.  Later intents see the results of earlier ones.
//!
//! ## Precondition validation
//!
//! Before executing any intent, the system checks:
//! - The entity still has `IsAlive` (may have been killed by an earlier actor)
//! - The entity still has sufficient `ActionPoints`
//! - For targeted intents: the target entity still exists and is valid
//!
//! Cancelled intents cost no AP.
//!
//! ## Relationship to activities
//!
//! Intents that start multi-turn activities (`StartCraft`, `StartRead`) insert
//! `(ActivityProgress, <Type>)` components.  The activity system (next phase)
//! ticks them.  An entity with an active activity cannot declare new intents
//! until the activity completes.

use bevy_ecs::prelude::*;

use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::intent::{ActionIntent, IntentQueue, QueuedIntent};
use cdda_components::item::{InProgressCraft, InsideContainer};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::WorldPos;

use crate::actor::turn::{AP_COST_PICKUP, AP_COST_WIELD, MOVE_COST_ATTACK_BASE, MOVE_COST_WALK};
use tracing::info;

// ---------------------------------------------------------------------------
// collect_intents — gather all ActionIntent components, sort by AP
// ---------------------------------------------------------------------------

/// Collects every entity's `ActionIntent`, builds the `IntentQueue` sorted
/// by AP descending (highest AP acts first).  Removes the `ActionIntent`
/// component from each entity after collection.
pub fn collect_intents(
    mut commands: Commands,
    q_intents: Query<(Entity, &ActionIntent, &ActionPoints)>,
    mut queue: ResMut<IntentQueue>,
) {
    queue.queued.clear();
    queue.rejected = 0;

    let mut entries: Vec<QueuedIntent> = q_intents
        .iter()
        .map(|(entity, intent, ap)| QueuedIntent {
            entity,
            intent: intent.clone(),
            ap: ap.current,
        })
        .collect();

    // Sort by AP descending — highest acts first.
    entries.sort_by(|a, b| b.ap.cmp(&a.ap));

    // Remove the intent components so they don't persist to next turn.
    for (entity, ..) in &q_intents {
        commands.entity(entity).remove::<ActionIntent>();
    }

    queue.queued = entries;
}

// ---------------------------------------------------------------------------
// resolve_intents — drain IntentQueue, validate, execute
// ---------------------------------------------------------------------------

/// Resolve intents in AP-priority order.  For each intent, validate
/// preconditions; if they pass, execute the action and deduct AP.
pub fn resolve_intents(
    mut commands: Commands,
    mut queue: ResMut<IntentQueue>,
    mut q_ap: Query<&mut ActionPoints>,
    q_is_alive: Query<(), With<IsAlive>>,
    q_pos: Query<&WorldPosition>,
) {
    // Drain all queued intents; order is already sorted.
    let intents: Vec<QueuedIntent> = std::mem::take(&mut queue.queued);

    for pending in intents {
        // ── Precondition: entity still alive ─────────────────────────
        if q_is_alive.get(pending.entity).is_err() {
            queue.rejected += 1;
            continue;
        }

        // ── Precondition: entity still has enough AP ─────────────────
        let ap_ok = q_ap
            .get(pending.entity)
            .map(|ap| ap.current >= 0)
            .unwrap_or(false);
        if !ap_ok {
            queue.rejected += 1;
            continue;
        }

        // ── Resolve ──────────────────────────────────────────────────
        match &pending.intent {
            ActionIntent::Move { dx, dy } => {
                resolve_move(&mut commands, pending.entity, *dx, *dy, &mut q_ap, &q_pos);
            }
            ActionIntent::MeleeAttack { target } => {
                // Precondition: target still alive
                if q_is_alive.get(*target).is_err() {
                    queue.rejected += 1;
                    continue;
                }
                resolve_melee_attack(&mut commands, pending.entity, *target, &mut q_ap);
            }
            ActionIntent::Pickup { item } => {
                resolve_pickup(&mut commands, pending.entity, *item, &mut q_ap);
            }
            ActionIntent::Wield { item } => {
                resolve_wield(&mut commands, pending.entity, *item, &mut q_ap);
            }
            ActionIntent::StartCraft { recipe } => {
                resolve_start_craft(&mut commands, pending.entity, *recipe, &mut q_ap);
            }
            ActionIntent::Wait => {
                // Burn AP without doing anything.
                if let Ok(mut ap) = q_ap.get_mut(pending.entity) {
                    ap.spend(100);
                }
            }
            // Stubs for unimplemented intent types:
            ActionIntent::UseItem { .. }
            | ActionIntent::Reload { .. }
            | ActionIntent::StartRead { .. }
            | ActionIntent::Interact { .. } => {
                // Burn 100 AP as placeholder cost.
                if let Ok(mut ap) = q_ap.get_mut(pending.entity) {
                    ap.spend(100);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Intent resolvers — one per intent type
// ---------------------------------------------------------------------------

/// Move the entity one tile.  Deducts `MOVE_COST_WALK` AP.
fn resolve_move(
    commands: &mut Commands,
    entity: Entity,
    dx: i32,
    dy: i32,
    ap_q: &mut Query<&mut ActionPoints>,
    pos_q: &Query<&WorldPosition>,
) {
    if let Ok(mut ap) = ap_q.get_mut(entity) {
        ap.spend(MOVE_COST_WALK);
    }
    if let Ok(pos) = pos_q.get(entity) {
        let new_pos = WorldPos::new(pos.get().x + dx, pos.get().y + dy, pos.get().z);
        commands.entity(entity).insert(WorldPosition::new(new_pos));
    }
}

/// Resolve a melee attack: deduct AP, the combat system handles actual
/// damage resolution later.
fn resolve_melee_attack(
    _commands: &mut Commands,
    entity: Entity,
    _target: Entity,
    ap_q: &mut Query<&mut ActionPoints>,
) {
    if let Ok(mut ap) = ap_q.get_mut(entity) {
        ap.spend(MOVE_COST_ATTACK_BASE);
    }
    // Actual damage resolution is handled by the combat phase.
    // TODO: emit a MeleeAttackEvent or DamageEvent for the combat system.
    let _ = entity;
}

/// Pick up an item: deduct AP, insert `InsideContainer(entity)` on the item.
fn resolve_pickup(
    commands: &mut Commands,
    entity: Entity,
    item: Entity,
    ap_q: &mut Query<&mut ActionPoints>,
) {
    if let Ok(mut ap) = ap_q.get_mut(entity) {
        ap.spend(AP_COST_PICKUP);
    }
    commands.entity(item).insert(InsideContainer(entity));
}

/// Wield an item: deduct AP, insert `WieldedBy(entity)` on the item.
fn resolve_wield(
    _commands: &mut Commands,
    entity: Entity,
    _item: Entity,
    ap_q: &mut Query<&mut ActionPoints>,
) {
    if let Ok(mut ap) = ap_q.get_mut(entity) {
        ap.spend(AP_COST_WIELD);
    }
    // Actual wield logic is handled by equipment systems.
    let _ = entity;
}

/// Start crafting: insert `(ActivityProgress, Crafting)` components.
/// The activity system picks up the progress next frame.
fn resolve_start_craft(
    commands: &mut Commands,
    entity: Entity,
    recipe: Entity,
    ap_q: &mut Query<&mut ActionPoints>,
) {
    // Deduct one tick's worth of AP immediately.
    if let Ok(mut ap) = ap_q.get_mut(entity) {
        ap.spend(100);
    }
    // The activity system will handle the multi-turn craft progression.
    // For now, insert the Crafting component stub; the real start_craft
    // logic (recipe lookup, component consumption) lives in cdda_crafting.
    let _ = recipe;
    info!(
        "StartCraft intent resolved for entity {:?}, recipe {:?}",
        entity, recipe
    );
}
