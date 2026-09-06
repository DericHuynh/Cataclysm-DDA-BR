use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::schedule::{SimSet, SimulationAction};

use super::systems::{
    drive_behaviour_tree, drive_goap, drive_none, has_behaviour_tree_agents, has_goap_agents,
    has_htn_agents,
};
use crate::ai::htn::exec::drive_htn_system;
use crate::intent::systems::collect_intents;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // Planner systems run inside SimulationAction for the ACTING entity
        // selected by the budget scheduler (ActingEntity resource). Direct
        // test calls without that resource keep the previous all-agents
        // behavior. Producers are anchored before the collector: sharing
        // SimSet::IntentDeclare alone does NOT order them.
        app.add_systems(
            SimulationAction,
            (
                drive_behaviour_tree.run_if(has_behaviour_tree_agents),
                drive_goap.run_if(has_goap_agents),
                drive_htn_system.run_if(has_htn_agents),
                drive_none,
            )
                .in_set(SimSet::IntentDeclare)
                .before(collect_intents),
        );
    }
}
