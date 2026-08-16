//! Bevy plugin registering all activity system resources and systems.
//!
//! Each activity type has its own regular system.  Systems run in
//! `SimSet::Activity` (after `TurnTick`, before `Ai`).  `cleanup_done_activities`
//! runs after all tick systems as a safety net.
//!
//! ## Multi-activity future
//!
//! All systems query for different activity component types (`Crafting`,
//! `Aiming`, etc.), so a character can hold multiple activities simultaneously
//! without system conflict.  When that feature is enabled, all tick systems
//! process their respective activities independently.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::activity::ActivityTracker;
use cdda_components::schedule::SimSet;

use super::systems::{
    cleanup_done_activities, tick_aiming, tick_crafting, tick_interacting, tick_reading,
    tick_reloading, tick_waiting,
};

pub struct ActivityPlugin;

impl Plugin for ActivityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ActivityTracker>();

        // ── Per-activity tick systems ─────────────────────────────
        // Each system only touches entities with its specific activity
        // component type, so they can run in parallel.
        app.add_systems(
            Update,
            (
                tick_crafting,
                tick_aiming,
                tick_reading,
                tick_waiting,
                tick_reloading,
                tick_interacting,
            )
                .in_set(SimSet::Activity),
        );

        // Safety net: runs after all tick systems to catch stale Done-phase
        // activities that weren't cleaned up inline.
        app.add_systems(
            Update,
            cleanup_done_activities
                .in_set(SimSet::Activity)
                .after(tick_crafting)
                .after(tick_aiming)
                .after(tick_reading)
                .after(tick_waiting)
                .after(tick_reloading)
                .after(tick_interacting),
        );
    }
}
