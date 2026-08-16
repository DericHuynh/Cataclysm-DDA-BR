//! Crafting execution — the use-case executor for starting a craft.
//!
//! `process_pending_craft` drains the `PendingCraft` queue and invokes
//! `start_craft`. The keyboard navigation of the crafting menu has moved up
//! to `cdda_render::render::input::crafting_menu_input` (the presenter layer);
//! this module keeps only the simulation-side craft execution.

use bevy_ecs::prelude::*;

use super::systems::{build_craft_state, find_dev_player, start_craft, CraftState, PendingCraft};

/// Drain `PendingCraft`, execute the craft, and rebuild `CraftState`.
pub fn process_pending_craft(world: &mut World) {
    let recipe_entity = {
        let mut pending = world.resource_mut::<PendingCraft>();
        pending.0.take()
    };
    let Some(recipe_entity) = recipe_entity else {
        return;
    };

    let Some(player) = find_dev_player(world) else {
        return;
    };

    match start_craft(world, player, recipe_entity) {
        Ok(craft_e) => {
            let result_name = world
                .get::<cdda_components::item::InProgressCraft>(craft_e)
                .map(|c| c.result_name.clone())
                .unwrap_or_else(|| "item".to_string());
            tracing::info!("Started crafting: {}", result_name);
            if let Some(mut state) = world.get_resource_mut::<CraftState>() {
                state.last_message = Some(format!("Crafting: {}", result_name));
            }
        }
        Err(e) => {
            tracing::warn!("Craft failed: {}", e);
            if let Some(mut state) = world.get_resource_mut::<CraftState>() {
                state.last_message = Some(format!("Failed: {}", e));
            }
        }
    }

    build_craft_state(world);
}
