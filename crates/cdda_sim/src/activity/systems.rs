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
use cdda_components::actor::ActionPoints;
use cdda_components::item::InProgressCraft;
use cdda_components::messages::CraftCompleted;

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
    mut query: Query<(
        Entity,
        &mut ActivityProgress,
        &Crafting,
        Option<&mut ActionPoints>,
        Option<&mut ActivityTracker>,
    )>,
    mut craft_query: Query<&mut InProgressCraft>,
    mut craft_done: MessageWriter<CraftCompleted>,
) {
    for (entity, mut progress, crafting, ap, tracker) in &mut query {
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
            // Guard: craft entity may have been despawned externally.
            if craft_query.get(crafting.craft_entity).is_err() {
                progress.phase = ActivityPhase::Done;
                progress.moves_left = 0;
                continue;
            }

            // Spend AP and advance craft progress.
            if let Some(mut ap) = ap {
                ap.spend(AP_COST_CRAFT_TICK);
            }
            if let Some(mut tracker) = tracker {
                tracker.log_activity(BRISK_EXERCISE);
            }

            if let Ok(mut craft) = craft_query.get_mut(crafting.craft_entity) {
                craft.ap_spent += AP_COST_CRAFT_TICK;
            }
            progress.moves_left -= AP_COST_CRAFT_TICK;
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
    mut query: Query<(
        Entity,
        &mut ActivityProgress,
        &mut Aiming,
        Option<&mut ActivityTracker>,
    )>,
) {
    for (entity, mut progress, mut aiming, tracker) in &mut query {
        if progress.phase == ActivityPhase::Pending {
            // Aim is NEITHER-based; tick drives it.
            progress.moves_total = -1;
            progress.moves_left = 1;
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(LIGHT_EXERCISE);
            }
            aiming.cur_aim = (aiming.cur_aim + 5).min(aiming.target_aim_percent);
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
    mut query: Query<(
        Entity,
        &mut ActivityProgress,
        &mut Reading,
        Option<&mut ActivityTracker>,
    )>,
) {
    for (entity, mut progress, mut reading, tracker) in &mut query {
        if progress.phase == ActivityPhase::Pending {
            progress.moves_total = reading.turns_total * 100;
            progress.moves_left = reading.turns_total * 100;
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(NO_EXERCISE);
            }
            reading.turns_read += 1;
            progress.moves_left -= 100;
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
    mut query: Query<(
        Entity,
        &mut ActivityProgress,
        &Waiting,
        Option<&mut ActivityTracker>,
    )>,
) {
    for (entity, mut progress, waiting, tracker) in &mut query {
        if progress.phase == ActivityPhase::Pending {
            progress.moves_total = waiting.turns * 100;
            progress.moves_left = waiting.turns * 100;
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(NO_EXERCISE);
            }
            progress.moves_left -= 100;
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
    mut query: Query<(
        Entity,
        &mut ActivityProgress,
        &Reloading,
        Option<&mut ActivityTracker>,
    )>,
) {
    for (entity, mut progress, reloading, tracker) in &mut query {
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
            progress.moves_left -= 100;
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
    mut query: Query<(
        Entity,
        &mut ActivityProgress,
        &Interacting,
        Option<&mut ActivityTracker>,
    )>,
) {
    for (entity, mut progress, interacting, tracker) in &mut query {
        if progress.phase == ActivityPhase::Pending {
            progress.moves_total = interacting.duration * 100;
            progress.moves_left = interacting.duration * 100;
            progress.phase = ActivityPhase::Active;
        }

        if progress.phase == ActivityPhase::Active {
            if let Some(mut t) = tracker {
                t.log_activity(MODERATE_EXERCISE);
            }
            progress.moves_left -= 100;
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
    q_progress: Query<(Entity, &ActivityProgress)>,
) {
    for (entity, progress) in &q_progress {
        if progress.phase == ActivityPhase::Done {
            // Remove the progress component; the type component stays
            // and will be cleaned up by the next tick cycle or this one.
            commands.entity(entity).remove::<ActivityProgress>();
        }
    }
}
