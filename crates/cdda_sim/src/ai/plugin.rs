use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::schedule::SimSet;

use super::systems::{
    drive_behaviour_tree, drive_goap, drive_htn, drive_none, has_behaviour_tree_agents,
    has_goap_agents, has_htn_agents,
};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // Each planner system runs *only* when at least one entity carries its
        // marker (the `has_*_agents` run conditions), and they all run in the
        // declare phase so their produced intents join the same AP-sorted
        // `IntentQueue` as the player's.  No planner has priority — the
        // highest-AP intent (player or mob) resolves first.
        app.add_systems(
            Update,
            (
                drive_behaviour_tree.run_if(has_behaviour_tree_agents),
                drive_goap.run_if(has_goap_agents),
                drive_htn.run_if(has_htn_agents),
                drive_none, // inert — PlannerNone entities never act
            )
                .in_set(SimSet::IntentDeclare),
        );
    }
}
