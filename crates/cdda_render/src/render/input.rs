//! # Screen input adapters ("presenter" systems)
//!
//! The UI-input systems for screen-keyboard navigation and item actions.
//! Relocated from `cdda_sim` so the simulation crate no longer matches the
//! display-UI `GameAction` enum. These systems translate `InputAction`
//! messages (the Bevy `Message` stream from `cdda_input`) into use-case
//! calls that live in `cdda_sim` (crafting, inventory) and navigation
//! transitions (`cdda_context`).
//!
//! This is the Bevy-idiomatic expression of the "presenter" role: a plain
//! `SystemParam` system that sits above the sim (`cdda_render` is a Layer 5
//! crate), renders nothing itself, and lets the sim stay a pure use-case
//! layer free of UI vocabulary.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_state::prelude::NextState;

use cdda_components::actor::{ActionPoints, HandCount};
use cdda_components::context::{push_ctx, ContextStack, Ctx, FocusedCommandIndex};
use cdda_components::def::ItemVolume;
use cdda_components::dev::{DevCamera, DevGroundItemName, DevPlayer};
use cdda_components::events::{ItemMoveEvent, MoveLocation};
use cdda_components::input::{GameAction, InputAction, InputContextId, InputContextStack};
use cdda_components::item::{
    ContainerContents, InsideContainer, InventoryFocus, Invlet, MountedPockets, WieldedBy,
    WieldedItems, FLOOR_CAP_ML,
};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::{WorldPos, ZLevel, TILES_PER_OMT};
use cdda_sim::actor::turn::{AP_COST_PICKUP, AP_COST_WIELD};
use cdda_sim::crafting::systems::{CategoryIndex, CraftEntry, CraftState, PendingCraft};
use cdda_sim::inventory::examine_resource::ExaminedItem;
use cdda_sim::inventory::pocket::get_body_pocket;
use cdda_sim::inventory::systems::all_items_for_creature_q;
use tracing::warn;

// ---------------------------------------------------------------------------
// Crafting menu input
// ---------------------------------------------------------------------------

/// Handle keyboard navigation and filter entry for the crafting menu.
///
/// Moved from `cdda_sim::crafting::input` — a UI adapter above the sim. The
/// sim's `process_pending_craft` (a use-case executor) remains in `cdda_sim`.
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
// Inventory screen input
// ---------------------------------------------------------------------------

/// Handle navigation and item actions while `Ctx::Inventory` is open.
///
/// - **j / k / arrows** — move focus up / down through item rows
/// - **Enter / e**       — drop the focused item at the camera's OMT tile
///
/// Gated by `run_if(in_state(Ctx::Inventory))` at registration in cdda_app.
/// `GameAction::Cancel` (Esc/q) is handled by `handle_navigation_input` which
/// pops the screen back to Gameplay.
///
/// Moved from `cdda_sim::inventory::systems`.
pub fn inventory_screen_input(
    mut reader: MessageReader<InputAction>,
    mut focus: ResMut<InventoryFocus>,
    player_query: Query<(Entity, &HandCount), With<DevPlayer>>,
    wielded_items_q: Query<&WieldedItems>,
    wielded_by_check: Query<Entity, With<WieldedBy>>,
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
    mut commands: Commands,
    mut stack: ResMut<ContextStack>,
    mut next_screen: ResMut<NextState<Ctx>>,
    mut focused_cmd: ResMut<FocusedCommandIndex>,
    contents_q: Query<&ContainerContents>,
    invlet_q: Query<&Invlet>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, hand_count)) = player_query.single() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    // Collect pocket item entities: in containers/pockets with an Invlet, not
    // currently wielded.
    let all_items = all_items_for_creature_q(
        player_entity,
        &contents_q,
        &mounted_pockets_q,
        &wielded_items_q,
    );
    let mut pocket_items: Vec<(char, Entity)> = Vec::new();
    for item in all_items {
        if wielded_by_check.get(item).is_ok() {
            continue;
        }
        if let Ok(invlet) = invlet_q.get(item) {
            pocket_items.push((invlet.0, item));
        }
    }
    pocket_items.sort_by_key(|(c, _)| *c);

    // Wielded items
    let wielded_list: Vec<Entity> = wielded_items_q
        .get(player_entity)
        .ok()
        .map(|wi| wi.iter().collect())
        .unwrap_or_default();

    let current_panel_len = if focus.panel == 0 {
        pocket_items.len()
    } else {
        wielded_list.len()
    };

    for action in actions {
        match action {
            GameAction::NavigateUp => {
                focus.index = focus.index.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                if current_panel_len > 0 {
                    focus.index = (focus.index + 1).min(current_panel_len - 1);
                }
            }
            GameAction::NavigateHome => {
                focus.index = 0;
            }
            GameAction::NavigateEnd => {
                focus.index = current_panel_len.saturating_sub(1);
            }
            // Tab / Shift-Tab: cycle between pocket panel and wielded panel.
            GameAction::NavigateNextTab | GameAction::NavigatePrevTab => {
                focus.panel = 1 - focus.panel.min(1);
                focus.index = 0;
            }

            // [Enter] — open item examine / action menu.
            GameAction::Confirm => {
                if focus.panel != 0 {
                    continue;
                }
                if let Some(&(_, item_entity)) = pocket_items.get(focus.index) {
                    commands.insert_resource(ExaminedItem(Some(item_entity)));
                    push_ctx(
                        Ctx::Inventory,
                        Ctx::ItemExamine,
                        &mut stack,
                        &mut next_screen,
                        &mut focused_cmd,
                    );
                }
            }

            // [w] — wield from pocket panel, or unwield from wielded panel.
            GameAction::UseItem => {
                if focus.panel == 0 {
                    // Wield: pocket → hand.
                    if let Some(&(_, item_entity)) = pocket_items.get(focus.index) {
                        let wielded_count = wielded_list.len();
                        if wielded_count < hand_limit {
                            commands
                                .entity(item_entity)
                                .remove::<InsideContainer>()
                                .insert(WieldedBy(player_entity));
                            if let Ok(mut ap) = ap_query.single_mut() {
                                ap.spend(AP_COST_WIELD);
                            }
                        } else {
                            warn!(
                                "Hands full ({}/{}) — cannot wield.",
                                wielded_count, hand_limit
                            );
                        }
                    }
                } else {
                    // Unwield: hand → body pocket.
                    if let Some(&item_entity) = wielded_list.get(focus.index) {
                        let body_pocket = get_body_pocket(player_entity, &mounted_pockets_q)
                            .unwrap_or(player_entity);
                        commands
                            .entity(item_entity)
                            .remove::<WieldedBy>()
                            .insert(InsideContainer(body_pocket));
                        if let Ok(mut ap) = ap_query.single_mut() {
                            ap.spend(AP_COST_WIELD);
                        }
                        let new_len = wielded_list.len().saturating_sub(1);
                        focus.index = focus.index.min(new_len);
                    }
                }
            }

            // [X / examine] — open item detail overlay (pocket panel only).
            GameAction::Examine => {
                if focus.panel == 0 {
                    if let Some(&(_, item_entity)) = pocket_items.get(focus.index) {
                        commands.insert_resource(ExaminedItem(Some(item_entity)));
                        push_ctx(
                            Ctx::Inventory,
                            Ctx::ItemExamine,
                            &mut stack,
                            &mut next_screen,
                            &mut focused_cmd,
                        );
                    }
                }
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Dev-world pickup / drop
// ---------------------------------------------------------------------------

/// Handles `Pickup` and `Drop` actions in the dev world.
///
/// - **g / Pickup** — picks up all items at the camera's current OMT tile.
/// - **d / Drop**   — drops the first invlet-assigned item at the camera's tile.
///
/// Emits `ItemMoveEvent` messages for each item moved. The
/// `process_item_move_events` system (which runs later in the same
/// `SimSet::Inventory` phase) applies the actual component changes.
///
/// Moved from `cdda_sim::inventory::systems`.
pub fn dev_pickup_drop_system(
    mut reader: MessageReader<InputAction>,
    camera: Res<DevCamera>,
    player_query: Query<(Entity, &HandCount), With<DevPlayer>>,
    ground_item_query: Query<
        (Entity, &WorldPosition, Option<&ItemVolume>),
        With<DevGroundItemName>,
    >,
    item_volumes: Query<Option<&ItemVolume>>,
    wielded_items_q: Query<&WieldedItems>,
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
    mut move_writer: MessageWriter<ItemMoveEvent>,
    contents_q: Query<&ContainerContents>,
    invlet_q: Query<&Invlet>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, hand_count)) = player_query.single() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    for action in actions {
        match action {
            GameAction::Pickup => {
                let to_pickup: Vec<(Entity, WorldPos)> = ground_item_query
                    .iter()
                    .filter(|(_, wp, _)| {
                        wp.0.x.div_euclid(TILES_PER_OMT) == camera.x
                            && wp.0.y.div_euclid(TILES_PER_OMT) == camera.y
                            && wp.0.z.0 as i32 == camera.z
                    })
                    .map(|(e, wp, _)| (e, wp.0))
                    .collect();

                for (item, pos) in to_pickup {
                    // Fill hand slots first (WieldedBy), then fall back to
                    // the body pocket.
                    let wielded_count = wielded_items_q
                        .get(player_entity)
                        .ok()
                        .map(|wi| wi.iter().count())
                        .unwrap_or(0);

                    if wielded_count < hand_limit {
                        move_writer.write(ItemMoveEvent {
                            item,
                            from: MoveLocation::Ground(pos),
                            to: MoveLocation::Wielded(player_entity),
                            count: 1,
                        });
                    } else {
                        let body_pocket = get_body_pocket(player_entity, &mounted_pockets_q)
                            .unwrap_or(player_entity);
                        move_writer.write(ItemMoveEvent {
                            item,
                            from: MoveLocation::Ground(pos),
                            to: MoveLocation::Container(body_pocket),
                            count: 1,
                        });
                    }
                    if let Ok(mut ap) = ap_query.single_mut() {
                        ap.spend(AP_COST_PICKUP);
                    }
                }
            }

            GameAction::Drop => {
                // Drop the first invlet-assigned item in the player's domain.
                let invlet_items: Vec<(char, Entity)> = all_items_for_creature_q(
                    player_entity,
                    &contents_q,
                    &mounted_pockets_q,
                    &wielded_items_q,
                )
                .iter()
                .filter_map(|&e| invlet_q.get(e).ok().map(|i| (i.0, e)))
                .collect();

                if let Some(&(_c, item_entity)) = invlet_items.first() {
                    // Volume check: floor has a hard cap of FLOOR_CAP_ML.
                    let item_vol = item_volumes
                        .get(item_entity)
                        .ok()
                        .flatten()
                        .map(|v| v.0)
                        .unwrap_or(0);
                    let floor_volume: u32 = ground_item_query
                        .iter()
                        .filter(|(_, wp, _)| {
                            wp.0.x.div_euclid(TILES_PER_OMT) == camera.x
                                && wp.0.y.div_euclid(TILES_PER_OMT) == camera.y
                                && wp.0.z.0 as i32 == camera.z
                        })
                        .filter_map(|(_, _, vol)| vol.map(|v| v.0))
                        .sum();
                    if floor_volume + item_vol > FLOOR_CAP_ML {
                        warn!(
                            "Floor ({},{}) full: {}/{} mL — cannot drop.",
                            camera.x, camera.y, floor_volume, FLOOR_CAP_ML
                        );
                        continue;
                    }

                    let drop_pos = WorldPos::new(
                        camera.x * TILES_PER_OMT,
                        camera.y * TILES_PER_OMT,
                        ZLevel::new(camera.z as i8),
                    );
                    move_writer.write(ItemMoveEvent {
                        item: item_entity,
                        from: MoveLocation::Container(player_entity),
                        to: MoveLocation::Ground(drop_pos),
                        count: 1,
                    });
                }
            }
            _ => {}
        }
    }
}
