//! Bevy plugin registering all activity system resources and systems.
//!
//! Each activity type has its own regular system inside `SimulationTurn`'s
//! `SimSet::Activity`, after intent resolution. `cleanup_done_activities` runs
//! after tick systems. The outer simulation driver owns time and pause gating.
//! Activity types share `ActivityProgress`; this does not permit concurrent
//! activities on one actor. Shared AP arbitration remains a pending extension.

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::activity::ActivityTracker;
use cdda_components::schedule::{SimSet, SimulationTurn};

use super::systems::{
    cleanup_done_activities, tick_aiming, tick_crafting, tick_interacting, tick_reading,
    tick_reloading, tick_waiting,
};

pub struct ActivityPlugin;

impl Plugin for ActivityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ActivityTracker>();

        // ── Per-activity tick systems ─────────────────────────────
        // Bevy schedules these according to their actual component access;
        // different activity tags alone do not establish query disjointness.
        app.add_systems(
            SimulationTurn,
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
            SimulationTurn,
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
