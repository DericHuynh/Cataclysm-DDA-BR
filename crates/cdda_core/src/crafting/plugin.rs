use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_state::prelude::OnEnter;

use crate::context::ctx::Ctx;
use crate::crafting::systems::{
    build_craft_state, complete_craft, CategoryIndex, CraftState, PendingCraft, RecipeIndex,
};
use crate::crafting::input::{crafting_menu_input, process_pending_craft};
use cdda_components::schedule::SimSet;

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        // Register the craft completion hook so cdda_activity can complete
        // crafts without a circular dependency on cdda_crafting.
        cdda_activity::CRAFT_COMPLETE_HOOK
            .set(complete_craft)
            .ok();

        app.init_resource::<CraftState>();
        app.init_resource::<PendingCraft>();
        app.init_resource::<RecipeIndex>();
        app.init_resource::<CategoryIndex>();

        // OnEnter: build the recipe/category index when crafting menu opens.
        app.add_systems(OnEnter(Ctx::CraftingMenu), build_craft_state);

        // Input: runs whenever the crafting menu is open.
        app.add_systems(
            Update,
            crafting_menu_input.run_if(bevy_state::condition::in_state(Ctx::CraftingMenu)),
        );

        // Simulation: execute pending craft in the Activity phase so it
        // participates in the AP-driven activity system.
        app.add_systems(
            Update,
            process_pending_craft
                .in_set(SimSet::Activity)
                .run_if(bevy_state::condition::in_state(Ctx::CraftingMenu)),
        );
    }
}
