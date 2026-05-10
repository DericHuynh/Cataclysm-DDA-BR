//! Bevy plugin registering all activity system resources and systems.
//!
//! Schedules `start_pending_activities` before `tick_activities` so that
//! new activities are initialised before the per-turn tick runs, and
//! `cleanup_done_activities` after both to remove completed activities.
//!
//! All three run in `SimSet::Activity` (after `TurnTick`, before `Ai`).

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;

use crate::activity::systems::{
    cleanup_done_activities, start_pending_activities, tick_activities,
};
use crate::activity::tracker::ActivityTracker;
use crate::schedule::SimSet;

pub struct ActivityPlugin;

impl Plugin for ActivityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ActivityTracker>();

        // start_pending_activities: transitions Pending → Active and calls actor.start().
        app.add_systems(Update, start_pending_activities.in_set(SimSet::Activity));

        // tick_activities: advances every active PlayerActivity by one tick.
        app.add_systems(
            Update,
            tick_activities
                .in_set(SimSet::Activity)
                .after(start_pending_activities),
        );

        // cleanup_done_activities: removes PlayerActivity components stuck in Done phase.
        app.add_systems(
            Update,
            cleanup_done_activities
                .in_set(SimSet::Activity)
                .after(tick_activities),
        );
    }
}
