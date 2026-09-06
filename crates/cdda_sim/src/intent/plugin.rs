//! Intent systems run inside `SimulationAction`: one selected actor acts per
//! schedule run, repeated while that actor still has budget.
use super::systems::{collect_intents, resolve_intents};
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::intent::{ActionRequestCounter, IntentQueue};
use cdda_components::schedule::{SimSet, SimulationAction};

pub struct IntentPlugin;

impl Plugin for IntentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionRequestCounter>();
        app.init_resource::<IntentQueue>();
        app.configure_sets(
            SimulationAction,
            (SimSet::IntentDeclare, SimSet::IntentResolve).chain(),
        );
        app.add_systems(
            SimulationAction,
            collect_intents.in_set(SimSet::IntentDeclare),
        );
        app.add_systems(
            SimulationAction,
            resolve_intents
                .in_set(SimSet::IntentResolve)
                .after(collect_intents),
        );
    }
}
