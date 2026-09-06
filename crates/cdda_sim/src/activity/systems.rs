//! Activity systems — one regular system per activity type.
//!
//! Each system drives a specific activity's lifecycle (start → tick → finish)
//! and uses typed queries instead of `&mut World`.  This lets Bevy parallelize
//! activity processing across different characters and alongside other simulation
//! work.
//!
//! ## Per-system flow
//!
//! 1. **Pending** — set up `moves_total`/`moves_left`, transition to `Active`.
//! 2. **Active** — decrement progress, deduct resources, check completion.
//! 3. **Done** — emit completion events, remove the activity components.
//!
//! Pending → Active transition and the first tick run in the same frame
//! (single-pass iteration checks both phases sequentially).

use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::item::InProgressCraft;
use cdda_components::messages::CraftCompleted;
use cdda_components::schedule::ActingEntity;

use crate::actor::turn::AP_COST_CRAFT_TICK;
use cdda_components::activity::{
    ActivityPhase, ActivityProgress, ActivityTracker, Aiming, Crafting, Interacting, Reading,
    Reloading, Waiting, BRISK_EXERCISE, LIGHT_EXERCISE, MODERATE_EXERCISE, NO_EXERCISE,
};

// ===========================================================================
// Crafting
// ===========================================================================

/// Tick all crafting activities: start, progress, finish.
pub fn tick_crafting(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    mut query: Query<
        (
            Entity,
            &mut ActivityProgress,
            &Crafting,
            &mut ActionPoints,
            Option<&mut ActivityTracker>,
        ),
        With<IsAlive>,
    >,
    mut craft_query: Query<&mut InProgressCraft>,
    mut craft_done: MessageWriter<CraftCompleted>,
) {
    let entities = match acting.as_ref() {
        Some(acting) => vec![acting.0],
        None => query.iter().map(|row| row.0).collect(),
    };
    let mut selected = query.iter_many_mut(entities);
    while let Some((entity, mut progress, crafting, mut ap, tracker)) = selected.fetch_next() {
        // ── Pending → Active transition ────────────────────────────
        if progress.phase == ActivityPhase::Pending {
            let ap_total = craft_query
                .get(crafting.craft_entity)
                .map(|c| c.ap_total)
                .unwrap_or(100);
            progress.moves_total = ap_total;
            progress.moves_left = ap_total;
            progress.phase = ActivityPhase::Active;
        }

        // ── Active: tick ───────────────────────────────────────────
        if progress.phase == ActivityPhase::Active {
            let Ok(mut craft) = craft_query.get_mut(crafting.craft_entity) else {
                progress.phase = ActivityPhase::Done;
                progress.moves_left = 0;
                continue;
            };
            // Saved craft work is authoritative, including after interruption.
            progress.moves_left = craft.ap_total.saturating_sub(craft.ap_spent).max(0);
            // Master craft_activity_actor::do_turn spends the whole available
            // budget, including overshoot on completion. Neutral speed only;
            // crafting/exertion modifiers are a separate compatibility slice.
            let budget = ap.current.max(0);
            let work = budget.min(progress.moves_left);
            ap.spend(budget);
            if work == 0 && progress.moves_left > 0 {
                continue;
            }
            craft.ap_spent = craft.ap_spent.saturating_add(work).min(craft.ap_total);
            if let Some(mut tracker) = tracker {
                tracker.log_activity(BRISK_EXERCISE);
            }

            progress.moves_left -= work;
            if progress.moves_left <= 0 {
                progress.phase = ActivityPhase::Done;
            }
        }

        // ── Done: emit completion message, remove components ───────
        if progress.phase == ActivityPhase::Done {
            craft_done.write(CraftCompleted {
                crafter: entity,
                craft_entity: crafting.craft_entity,
            });
            commands.entity(entity).remove::<Crafting>();
            commands.entity(entity).remove::<ActivityProgress>();
        }
    }
}

// ===========================================================================
// Aiming
// ===========================================================================

/// Tick all aiming activities: start, progress, finish.
pub fn tick_aiming(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    mut query: Query<
        (
            Entity,
            &mut ActivityProgress,
            &mut Aiming,
            &mut ActionPoints,
            Option<&mut ActivityTracker>,
        ),
        With<IsAlive>,
    >,
) {
    let entities = match acting.as_ref() {
        Some(acting) => vec![acting.0],
        None => query.iter().map(|row| row.0).collect(),
    };
    let mut selected = query.iter_many_mut(entities);
    while let Some((entity, mut progress, mut aiming, mut ap, tracker)) = selected.fetch_next() {
        if progress.phase == ActivityPhase::Pending {
            // Native approximation: 20 AP per percentage point of aim.
            progress.moves_total =
                i32::try_from(aiming.target_aim_percent.saturating_sub(aiming.cur_aim))
                    .unwrap_or(i32::MAX)
                    .saturating_mul(20);
            progress.moves_left = progress.moves_total;
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(LIGHT_EXERCISE);
            }
            let work = spend_work(&mut ap, progress.moves_left);
            if work == 0 && progress.moves_left > 0 {
                continue;
            }
            progress.moves_left -= work;
            aiming.cur_aim = aiming
                .target_aim_percent
                .saturating_sub(((i64::from(progress.moves_left) + 19) / 20) as u32);
            if aiming.cur_aim >= aiming.target_aim_percent {
                progress.phase = ActivityPhase::Done;
            }
        }

        if progress.phase == ActivityPhase::Done {
            commands
                .entity(entity)
                .remove::<(Aiming, ActivityProgress)>();
        }
    }
}

// ===========================================================================
// Reading
// ===========================================================================

/// Tick all reading activities: start, progress, finish.
pub fn tick_reading(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    mut query: Query<
        (
            Entity,
            &mut ActivityProgress,
            &mut Reading,
            &mut ActionPoints,
            Option<&mut ActivityTracker>,
        ),
        With<IsAlive>,
    >,
) {
    let entities = match acting.as_ref() {
        Some(acting) => vec![acting.0],
        None => query.iter().map(|row| row.0).collect(),
    };
    let mut selected = query.iter_many_mut(entities);
    while let Some((entity, mut progress, mut reading, mut ap, tracker)) = selected.fetch_next() {
        if progress.phase == ActivityPhase::Pending {
            progress.moves_total = reading.turns_total.max(0).saturating_mul(100);
            progress.moves_left = reading.turns_total.max(0).saturating_mul(100);
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(NO_EXERCISE);
            }
            let work = spend_time(&mut ap, progress.moves_left);
            if work == 0 && progress.moves_left > 0 {
                continue;
            }
            progress.moves_left -= work;
            reading.turns_read = (progress.moves_total - progress.moves_left) / 100;
            if progress.moves_left <= 0 {
                progress.phase = ActivityPhase::Done;
            }
        }

        if progress.phase == ActivityPhase::Done {
            commands
                .entity(entity)
                .remove::<(Reading, ActivityProgress)>();
        }
    }
}

// ===========================================================================
// Waiting
// ===========================================================================

/// Tick all waiting activities: start, progress, finish.
pub fn tick_waiting(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    mut query: Query<
        (
            Entity,
            &mut ActivityProgress,
            &Waiting,
            &mut ActionPoints,
            Option<&mut ActivityTracker>,
        ),
        With<IsAlive>,
    >,
) {
    let entities = match acting.as_ref() {
        Some(acting) => vec![acting.0],
        None => query.iter().map(|row| row.0).collect(),
    };
    let mut selected = query.iter_many_mut(entities);
    while let Some((entity, mut progress, waiting, mut ap, tracker)) = selected.fetch_next() {
        if progress.phase == ActivityPhase::Pending {
            progress.moves_total = waiting.turns.max(0).saturating_mul(100);
            progress.moves_left = waiting.turns.max(0).saturating_mul(100);
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(NO_EXERCISE);
            }
            let work = spend_time(&mut ap, progress.moves_left);
            if work == 0 && progress.moves_left > 0 {
                continue;
            }
            progress.moves_left -= work;
            if progress.moves_left <= 0 {
                progress.phase = ActivityPhase::Done;
            }
        }

        if progress.phase == ActivityPhase::Done {
            commands
                .entity(entity)
                .remove::<(Waiting, ActivityProgress)>();
        }
    }
}

// ===========================================================================
// Reloading
// ===========================================================================

/// Tick all reloading activities: start, progress, finish.
pub fn tick_reloading(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    mut query: Query<
        (
            Entity,
            &mut ActivityProgress,
            &Reloading,
            &mut ActionPoints,
            Option<&mut ActivityTracker>,
        ),
        With<IsAlive>,
    >,
) {
    let entities = match acting.as_ref() {
        Some(acting) => vec![acting.0],
        None => query.iter().map(|row| row.0).collect(),
    };
    let mut selected = query.iter_many_mut(entities);
    while let Some((entity, mut progress, reloading, mut ap, tracker)) = selected.fetch_next() {
        if progress.phase == ActivityPhase::Pending {
            let base =
                (reloading.quantity as f32 * 100.0 / reloading.speed_factor.max(0.01)) as i32;
            progress.moves_total = base;
            progress.moves_left = base;
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(LIGHT_EXERCISE);
            }
            let work = spend_work(&mut ap, progress.moves_left);
            if work == 0 && progress.moves_left > 0 {
                continue;
            }
            progress.moves_left -= work;
            if progress.moves_left <= 0 {
                progress.phase = ActivityPhase::Done;
            }
        }

        if progress.phase == ActivityPhase::Done {
            commands
                .entity(entity)
                .remove::<(Reloading, ActivityProgress)>();
        }
    }
}

// ===========================================================================
// Interacting
// ===========================================================================

/// Tick all generic interaction activities: start, progress, finish.
pub fn tick_interacting(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    mut query: Query<
        (
            Entity,
            &mut ActivityProgress,
            &Interacting,
            &mut ActionPoints,
            Option<&mut ActivityTracker>,
        ),
        With<IsAlive>,
    >,
) {
    let entities = match acting.as_ref() {
        Some(acting) => vec![acting.0],
        None => query.iter().map(|row| row.0).collect(),
    };
    let mut selected = query.iter_many_mut(entities);
    while let Some((entity, mut progress, interacting, mut ap, tracker)) = selected.fetch_next() {
        if progress.phase == ActivityPhase::Pending {
            progress.moves_total = interacting.duration.max(0).saturating_mul(100);
            progress.moves_left = interacting.duration.max(0).saturating_mul(100);
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(MODERATE_EXERCISE);
            }
            let work = spend_time(&mut ap, progress.moves_left);
            if work == 0 && progress.moves_left > 0 {
                continue;
            }
            progress.moves_left -= work;
            if progress.moves_left <= 0 {
                progress.phase = ActivityPhase::Done;
            }
        }

        if progress.phase == ActivityPhase::Done {
            commands
                .entity(entity)
                .remove::<(Interacting, ActivityProgress)>();
        }
    }
}

// ===========================================================================
// cleanup_done_activities — safety net for stale Done-phase activities
// ===========================================================================

/// Remove any activity in `Done` phase that wasn't cleaned up by its tick system.
///
/// This catches edge cases like deserialized activities, externally despawned
/// craft entities, or activities that missed their finish step.
pub fn cleanup_done_activities(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    q_progress: Query<(Entity, &ActivityProgress)>,
) {
    for (entity, progress) in &q_progress {
        if acting.as_ref().is_none_or(|a| a.0 == entity) && progress.phase == ActivityPhase::Done {
            commands.entity(entity).remove::<(
                ActivityProgress,
                Crafting,
                Aiming,
                Reading,
                Waiting,
                Reloading,
                Interacting,
            )>();
        }
    }
}

/// Bounded speed-based work for aim and reload. No activity debt.
fn spend_work(ap: &mut ActionPoints, remaining: i32) -> i32 {
    let work = ap
        .current
        .max(0)
        .min(remaining.max(0))
        .min(AP_COST_CRAFT_TICK);
    if work > 0 {
        ap.spend(work);
    }
    work
}

/// Elapsed-time work advances at most one second, consuming this turn's budget.
/// A partial final second leaves the proportional budget available for actions.
fn spend_time(ap: &mut ActionPoints, remaining: i32) -> i32 {
    if ap.current <= 0 {
        return 0;
    }
    let work = remaining.clamp(0, 100);
    // Match player_activity's TIME branch: integer truncation, including zero
    // cost for a sufficiently short final fragment. Completion is still work.
    let cost = (i64::from(ap.current) * i64::from(work) / 100) as i32;
    if cost > 0 {
        ap.spend(cost);
    }
    work
}
