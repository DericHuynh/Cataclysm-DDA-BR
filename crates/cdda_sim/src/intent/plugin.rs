//! Bevy plugin for the intent resolution pipeline.
//!
//! Registers `collect_intents` (IntentDeclare) and `resolve_intents`
//! (IntentResolve) with proper ordering so intents are gathered before
//! they are resolved.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::schedule::SimSet;

use super::systems::{collect_intents, resolve_intents};

pub struct IntentPlugin;

impl Plugin for IntentPlugin {
    fn build(&self, app: &mut App) {
        // Collect intents from all entities (AI + player) into the IntentQueue.
        app.add_systems(Update, collect_intents.in_set(SimSet::IntentDeclare));

        // Resolve intents in AP order, with precondition validation.
        app.add_systems(
            Update,
            resolve_intents
                .in_set(SimSet::IntentResolve)
                .after(collect_intents),
        );
    }
}
