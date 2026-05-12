//! Crafting menu — input handling.
//!
//! Moved here from `input/crafting.rs` during crate extraction.
//! - `crafting_menu_input` — keyboard navigation and filter text entry
//! - `process_pending_craft` — drains the PendingCraft queue and executes craft

use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input::ButtonState;

use crate::crafting::systems::{
    build_craft_state, find_dev_player, start_craft, CategoryIndex, CraftEntry, CraftState,
    PendingCraft,
};
use cdda_components::input::{GameAction, InputAction, InputContextId, InputContextStack};

// ---------------------------------------------------------------------------
// crafting_menu_input — regular per-frame system
// ---------------------------------------------------------------------------

/// Handles keyboard navigation for the crafting menu.
pub fn crafting_menu_input(
    mut reader: MessageReader<InputAction>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut craft_state: ResMut<CraftState>,
    mut cat_index: ResMut<CategoryIndex>,
    mut pending: ResMut<PendingCraft>,
    mut input_ctx: ResMut<InputContextStack>,
) {
    // ── Filter mode ──────────────────────────────────────────────────────
    if craft_state.filtering {
        for _ in reader.read() {}

        for ev in keyboard.read() {
            if ev.state == ButtonState::Released || ev.repeat {
                continue;
            }
            match &ev.logical_key {
                Key::Character(ch) if !ch.chars().any(|c| c.is_control()) => {
                    if ch == "/" && craft_state.filter.is_empty() {
                        continue;
                    }
                    craft_state.filter.push_str(ch.as_str());
                    craft_state.focus = 0;
                }
                Key::Space => {
                    craft_state.filter.push(' ');
                    craft_state.focus = 0;
                }
                Key::Backspace => {
                    craft_state.filter.pop();
                    craft_state.focus = 0;
                }
                Key::Enter => {
                    craft_state.filtering = false;
                    input_ctx.pop();
                }
                Key::Escape => {
                    craft_state.filtering = false;
                    craft_state.filter.clear();
                    craft_state.focus = 0;
                    input_ctx.pop();
                }
                _ => {}
            }
        }
        return;
    }

    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    for action in actions {
        match action {
            GameAction::Filter => {
                if !craft_state.filtering {
                    craft_state.filtering = true;
                    input_ctx.push(InputContextId::TextInput);
                }
            }
            GameAction::NavigateNextTab => {
                cat_index.focus_zone = (cat_index.focus_zone + 1) % 3;
                craft_state.focus = 0;
            }
            GameAction::NavigatePrevTab => {
                cat_index.focus_zone = if cat_index.focus_zone == 0 {
                    2
                } else {
                    cat_index.focus_zone - 1
                };
                craft_state.focus = 0;
            }
            GameAction::NavigateLeft => {
                if cat_index.focus_zone == 1 {
                    let n = cat_index.top_categories.len();
                    if n > 0 {
                        cat_index.selected_top = if cat_index.selected_top == 0 {
                            n - 1
                        } else {
                            cat_index.selected_top - 1
                        };
                        cat_index.selected_sub = 0;
                        craft_state.focus = 0;
                    }
                } else if cat_index.focus_zone == 2 {
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    if !subcats.is_empty() && cat_index.selected_sub > 0 {
                        cat_index.selected_sub -= 1;
                        craft_state.focus = 0;
                    }
                }
            }
            GameAction::NavigateRight => {
                if cat_index.focus_zone == 1 {
                    let n = cat_index.top_categories.len();
                    if n > 0 {
                        cat_index.selected_top =
                            (cat_index.selected_top + 1).min(n.saturating_sub(1));
                        cat_index.selected_sub = 0;
                        craft_state.focus = 0;
                    }
                } else if cat_index.focus_zone == 2 {
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    if !subcats.is_empty()
                        && cat_index.selected_sub < subcats.len().saturating_sub(1)
                    {
                        cat_index.selected_sub += 1;
                        craft_state.focus = 0;
                    }
                }
            }
            GameAction::NavigateUp => {
                if cat_index.focus_zone == 0 {
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                    let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                    let key = (current_top, current_sub);
                    let cat_recipes: Vec<Entity> =
                        cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                    let cat_visible: Vec<&CraftEntry> = craft_state
                        .visible()
                        .filter(|e| cat_recipes.contains(&e.recipe_entity))
                        .collect();
                    let max = cat_visible.len().saturating_sub(1);
                    craft_state.focus = if craft_state.focus > max {
                        max
                    } else {
                        craft_state.focus.saturating_sub(1)
                    };
                } else if cat_index.focus_zone == 1 {
                    cat_index.focus_zone = 2;
                } else {
                    cat_index.focus_zone = 1;
                }
            }
            GameAction::NavigateDown => {
                if cat_index.focus_zone == 0 {
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                    let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                    let key = (current_top, current_sub);
                    let cat_recipes: Vec<Entity> =
                        cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                    let cat_visible: Vec<&CraftEntry> = craft_state
                        .visible()
                        .filter(|e| cat_recipes.contains(&e.recipe_entity))
                        .collect();
                    let max = cat_visible.len().saturating_sub(1);
                    craft_state.focus = (craft_state.focus + 1).min(max);
                } else if cat_index.focus_zone == 1 {
                    cat_index.focus_zone = 2;
                } else {
                    cat_index.focus_zone = 0;
                }
            }
            GameAction::NavigateHome => {
                craft_state.focus = 0;
            }
            GameAction::NavigateEnd => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                craft_state.focus = cat_visible.len().saturating_sub(1);
            }
            GameAction::NavigatePageUp => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                let max = cat_visible.len().saturating_sub(1);
                craft_state.focus = if craft_state.focus > 10 {
                    craft_state.focus.saturating_sub(10)
                } else {
                    0
                };
                if craft_state.focus > max {
                    craft_state.focus = max;
                }
            }
            GameAction::NavigatePageDown => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                let max = cat_visible.len().saturating_sub(1);
                craft_state.focus = (craft_state.focus + 10).min(max);
            }
            GameAction::Confirm => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                if let Some(entry) = cat_visible.get(craft_state.focus) {
                    if entry.craftable {
                        pending.0 = Some(entry.recipe_entity);
                    }
                }
            }
            GameAction::HotkeyPress('a') => {
                craft_state.show_all = !craft_state.show_all;
                let max = craft_state.visible_count().saturating_sub(1);
                if craft_state.focus > max {
                    craft_state.focus = max;
                }
            }
            GameAction::Cancel => {
                craft_state.last_message = None;
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// process_pending_craft — exclusive per-frame system
// ---------------------------------------------------------------------------

/// Drain `PendingCraft`, execute the craft, and rebuild `CraftState`.
pub fn process_pending_craft(world: &mut World) {
    use tracing;

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
