use bevy_app::{App, Plugin, Update};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_state::prelude::OnEnter;

use super::input::process_pending_craft;
use super::systems::{
    build_craft_state, complete_craft, CategoryIndex, CraftState, PendingCraft, RecipeIndex,
};
use cdda_components::context::Ctx;
use cdda_components::messages::CraftCompleted;
use cdda_components::schedule::SimSet;

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        // Register CraftCompleted message — replaces the old CRAFT_COMPLETE_HOOK.
        // tick_crafting writes this message; we read it here.
        app.add_message::<CraftCompleted>();

        app.init_resource::<CraftState>();
        app.init_resource::<PendingCraft>();
        app.init_resource::<RecipeIndex>();
        app.init_resource::<CategoryIndex>();

        // OnEnter: build the recipe/category index when crafting menu opens.
        app.add_systems(OnEnter(Ctx::CraftingMenu), build_craft_state);

        // Simulation: execute pending craft in the Activity phase so it
        // participates in the AP-driven activity system.
        app.add_systems(
            Update,
            process_pending_craft
                .in_set(SimSet::Activity)
                .run_if(bevy_state::condition::in_state(Ctx::CraftingMenu)),
        );

        // Craft completion: reads CraftCompleted messages emitted by
        // tick_crafting and spawns the result item.
        app.add_systems(Update, process_craft_completions.in_set(SimSet::Activity));
    }
}

/// Read `CraftCompleted` messages and call `complete_craft` for each.
///
/// This is an exclusive system because `complete_craft` needs `&mut World` to
/// spawn result items and manipulate inventory.  Craft completions are rare
/// (not every frame), so the scheduling cost is negligible.
fn process_craft_completions(world: &mut World) {
    let completed: Vec<(Entity, Entity)> = {
        let mut messages = world.resource_mut::<bevy_ecs::message::Messages<CraftCompleted>>();
        messages.update();
        messages
            .drain()
            .map(|c| (c.crafter, c.craft_entity))
            .collect()
    };
    for (player, craft_e) in &completed {
        complete_craft(world, *player, *craft_e);
    }
}
