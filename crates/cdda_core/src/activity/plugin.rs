//! Bevy plugin registering all activity system resources and systems.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;

use crate::activity::systems::{cleanup_done_activities, tick_activities};
use crate::activity::tracker::ActivityTracker;
use crate::schedule::SimSet;

pub struct ActivityPlugin;

impl Plugin for ActivityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ActivityTracker>();

        // Activity tick runs in SimSet::Effects (after movement/combat).
        // start_pending_activities and tick_activities use exclusive world access,
        // so they are registered as exclusive systems via app.add_systems with
        // the appropriate label.
        app.add_systems(
            Update,
            cleanup_done_activities
                .in_set(SimSet::Effects)
                .after(tick_activities),
        );
    }
}
