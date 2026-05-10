use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_state::prelude::OnEnter;

use crate::context::ctx::Ctx;
use crate::crafting::systems::{
    build_craft_state, CategoryIndex, CraftState, PendingCraft, RecipeIndex,
};
use crate::input::crafting::{crafting_menu_input, process_pending_craft};
use crate::render::crafting::{spawn_crafting_ui, update_crafting_ui};

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftState>();
        app.init_resource::<PendingCraft>();
        app.init_resource::<RecipeIndex>();
        app.init_resource::<CategoryIndex>();

        // OnEnter: exclusive state builder first, then spawn the root shell.
        app.add_systems(OnEnter(Ctx::CraftingMenu), build_craft_state);
        app.add_systems(
            OnEnter(Ctx::CraftingMenu),
            spawn_crafting_ui.after(build_craft_state),
        );

        app.add_systems(
            Update,
            (
                crafting_menu_input,
                update_crafting_ui.after(crafting_menu_input),
                process_pending_craft.after(crafting_menu_input),
            )
                .run_if(bevy_state::condition::in_state(Ctx::CraftingMenu)),
        );
    }
}
