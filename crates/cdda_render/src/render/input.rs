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
use bevy_ecs::message::MessageReader;
use bevy_state::prelude::NextState;

use crate::render::crafting_state::{CategoryIndex, CraftModel, CraftState, RecipeFilter};
use crate::render::dev_spawn::{DevSpawnCatalog, DevSpawnFocus, SpawnFilter};
use cdda_components::def::ItemName;
use cdda_components::dev::{DevGroundItemName, DevPlayer};
use cdda_components::intent::ActionIntent;
use cdda_components::item::{
    ContainerContents, InsideContainer, Invlet, ItemType, MountedOn, MountedPockets, WieldedBy,
    WieldedItems, WornBy, WornOn,
};
use cdda_components::sim::WorldPosition;
use cdda_context::state::{push_ctx, ContextStack, Ctx, FocusedCommandIndex};
use cdda_core_types::sim_id::SimId;
use cdda_data::interner::ItemTypeRegistry;
use cdda_input::vocabulary::{GameAction, InputAction, InputContextId, InputContextStack};
use cdda_sim::crafting::systems::PendingCraft;
use cdda_sim::inventory::examine_resource::ExaminedItem;
use cdda_sim::inventory::systems::all_items_for_creature_q;
use cdda_sim::inventory::transfer::within_reach;

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
    model: Res<CraftModel>,
    mut view: Local<RecipeFilter>,
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
                    view.update(
                        &model,
                        &craft_state,
                        &cat_index,
                        (model.last_changed().get(), cat_index.last_changed().get()),
                    );
                    let max = view.indices.len().saturating_sub(1);
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
                    view.update(
                        &model,
                        &craft_state,
                        &cat_index,
                        (model.last_changed().get(), cat_index.last_changed().get()),
                    );
                    let max = view.indices.len().saturating_sub(1);
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
                view.update(
                    &model,
                    &craft_state,
                    &cat_index,
                    (model.last_changed().get(), cat_index.last_changed().get()),
                );
                craft_state.focus = view.indices.len().saturating_sub(1);
            }
            GameAction::NavigatePageUp => {
                view.update(
                    &model,
                    &craft_state,
                    &cat_index,
                    (model.last_changed().get(), cat_index.last_changed().get()),
                );
                let max = view.indices.len().saturating_sub(1);
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
                view.update(
                    &model,
                    &craft_state,
                    &cat_index,
                    (model.last_changed().get(), cat_index.last_changed().get()),
                );
                let max = view.indices.len().saturating_sub(1);
                craft_state.focus = (craft_state.focus + 10).min(max);
            }
            GameAction::Confirm => {
                view.update(
                    &model,
                    &craft_state,
                    &cat_index,
                    (model.last_changed().get(), cat_index.last_changed().get()),
                );
                if let Some(entry) = view
                    .indices
                    .get(craft_state.focus.min(view.indices.len().saturating_sub(1)))
                    .map(|&i| &model.entries[i])
                {
                    if entry.craftable {
                        pending.0 = Some(entry.recipe_entity);
                    }
                }
            }
            GameAction::HotkeyPress('a') => {
                craft_state.show_all = !craft_state.show_all;
                view.update(
                    &model,
                    &craft_state,
                    &cat_index,
                    (model.last_changed().get(), cat_index.last_changed().get()),
                );
                let max = view.indices.len().saturating_sub(1);
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
/// - **Enter / e** — examine the focused item
/// - **w** — declare Wield/Stow for the focused row; simulation validates it
///
/// Gated by `run_if(in_state(Ctx::Inventory))` at registration in cdda_app.
/// `GameAction::Cancel` (Esc/q) is handled by `handle_navigation_input` which
/// pops the screen back to Gameplay.
///
/// Moved from `cdda_sim::inventory::systems`.
pub fn inventory_screen_input(
    mut reader: MessageReader<InputAction>,
    mut focus: ResMut<InventoryFocus>,
    player_query: Query<(Entity, Option<&ActionIntent>), With<DevPlayer>>,
    wielded_items_q: Query<&WieldedItems>,
    wielded_by_check: Query<Entity, With<WieldedBy>>,
    worn_items_q: Query<&WornBy>,
    mounted_pockets_q: Query<&MountedPockets>,
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

    let Ok((player_entity, pending_intent)) = player_query.single() else {
        return;
    };
    let mut submitted = pending_intent.is_some();

    // Collect pocket item entities: in containers/pockets with an Invlet, not
    // currently wielded.
    let all_items = all_items_for_creature_q(
        player_entity,
        &contents_q,
        &mounted_pockets_q,
        &wielded_items_q,
        &worn_items_q,
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
            GameAction::UseItem if !submitted => {
                let intent = if focus.panel == 0 {
                    pocket_items
                        .get(focus.index)
                        .map(|(_, item)| ActionIntent::Wield { item: *item })
                } else {
                    wielded_list
                        .get(focus.index)
                        .map(|item| ActionIntent::Stow { item: *item })
                };
                if let Some(intent) = intent {
                    commands.entity(player_entity).insert(intent);
                    submitted = true;
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

/// Declare one stable item action, never a transfer message or an AP mutation.
/// Pickup uses the actor's tile reach, not the OMT camera; drop uses the lowest
/// inventory letter. Existing requests are not overwritten by repeated input.
pub fn dev_pickup_drop_system(
    mut reader: MessageReader<InputAction>,
    player_query: Query<(Entity, &WorldPosition, Option<&ActionIntent>), With<DevPlayer>>,
    ground_item_query: Query<
        (Entity, &WorldPosition),
        (
            With<DevGroundItemName>,
            Without<InsideContainer>,
            Without<WieldedBy>,
            Without<WornOn>,
            Without<MountedOn>,
        ),
    >,
    wielded_items_q: Query<&WieldedItems>,
    worn_items_q: Query<&WornBy>,
    mounted_pockets_q: Query<&MountedPockets>,
    contents_q: Query<&ContainerContents>,
    invlet_q: Query<&Invlet>,
    ids: Query<&SimId>,
    mut commands: Commands,
) {
    let actions: Vec<_> = reader.read().map(|event| event.action.clone()).collect();
    let Ok((player, position, pending)) = player_query.single() else {
        return;
    };
    if pending.is_some() {
        return;
    }
    let stable_key = |entity: Entity| {
        let id = ids.get(entity).ok();
        (
            id.is_none(),
            id.map(|id| id.0).unwrap_or_default(),
            entity.to_bits(),
        )
    };
    for action in actions {
        let intent = match action {
            GameAction::Pickup => ground_item_query
                .iter()
                .filter(|(_, item_pos)| within_reach(position.get(), item_pos.get()))
                .map(|(entity, _)| entity)
                .min_by_key(|entity| stable_key(*entity))
                .map(|item| ActionIntent::Pickup { item }),
            GameAction::Drop => all_items_for_creature_q(
                player,
                &contents_q,
                &mounted_pockets_q,
                &wielded_items_q,
                &worn_items_q,
            )
            .into_iter()
            .filter_map(|item| invlet_q.get(item).ok().map(|invlet| (invlet.0, item)))
            .min_by_key(|(letter, entity)| (*letter, stable_key(*entity)))
            .map(|(_, item)| ActionIntent::Drop { item }),
            _ => None,
        };
        if let Some(intent) = intent {
            commands.entity(player).insert(intent);
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Dev spawn panel input
// ---------------------------------------------------------------------------

/// Keyboard + filter handling for the debug spawn panel (`Ctx::DevSpawnPanel`).
///
/// This is a presenter adapter (like `crafting_menu_input`): it reads the
/// decoupled `InputAction` stream and the raw `KeyboardInput` stream (while an
/// item filter is active), mutates the `DevSpawnFocus` UI state, and on `Confirm`
/// materialises a fresh item from the selected def into the dev player's body
/// pocket.
pub fn dev_spawn_input(
    mut reader: MessageReader<InputAction>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut focus: ResMut<DevSpawnFocus>,
    catalog: Res<DevSpawnCatalog>,
    mut view: Local<SpawnFilter>,
    mut input_ctx: ResMut<InputContextStack>,
    mut commands: Commands,
    mut type_registry: ResMut<ItemTypeRegistry>,
    player_query: Query<Entity, With<DevPlayer>>,
    mounted_pockets: Query<&MountedPockets>,
) {
    // Filter mode: consume actions and turn raw keys into filter text. This
    // matches the crafting-menu filter flow (see `crafting_menu_input`).
    if focus.filtering {
        for _ in reader.read() {}

        for ev in keyboard.read() {
            if ev.state == ButtonState::Released || ev.repeat {
                continue;
            }
            match &ev.logical_key {
                Key::Character(ch) if !ch.chars().any(|c| c.is_control()) => {
                    focus.filter.push_str(ch.as_str());
                    focus.index = 0;
                }
                Key::Space => {
                    focus.filter.push(' ');
                    focus.index = 0;
                }
                Key::Backspace => {
                    focus.filter.pop();
                    focus.index = 0;
                }
                Key::Enter => {
                    focus.filtering = false;
                    input_ctx.pop();
                }
                Key::Escape => {
                    focus.filtering = false;
                    focus.filter.clear();
                    focus.index = 0;
                    input_ctx.pop();
                }
                _ => {}
            }
        }
        return;
    }

    view.update(&catalog, &focus.filter, catalog.last_changed().get());
    let total = view.indices.len();
    let clamp = |i: usize| -> usize {
        if total == 0 {
            0
        } else {
            i.min(total - 1)
        }
    };

    for event in reader.read() {
        match &event.action {
            GameAction::NavigateUp | GameAction::NavigateLeft => {
                focus.index = clamp(focus.index.saturating_sub(1));
            }
            GameAction::NavigateDown | GameAction::NavigateRight => {
                focus.index = clamp(focus.index + 1);
            }
            GameAction::NavigatePageUp => focus.index = clamp(focus.index.saturating_sub(10)),
            GameAction::NavigatePageDown => focus.index = clamp(focus.index.saturating_add(10)),
            GameAction::NavigateHome => focus.index = 0,
            GameAction::NavigateEnd => focus.index = clamp(total),
            GameAction::Filter => {
                if !focus.filtering {
                    focus.filtering = true;
                    input_ctx.push(InputContextId::TextInput);
                }
            }
            GameAction::Confirm => {
                let Some(player) = player_query.iter().next() else {
                    continue;
                };
                // Fetch the selected entry fresh (avoid holding a borrow of
                // `focus` across the loop where `focus.index` is mutated).
                let Some(entry) = view.indices.get(focus.index).map(|&i| &catalog.entries[i])
                else {
                    continue;
                };
                let token = type_registry.intern(&entry.def_id);
                let pocket = mounted_pockets
                    .get(player)
                    .ok()
                    .and_then(|mp| mp.iter().next())
                    .unwrap_or(player);
                commands.spawn((
                    ItemType(token),
                    ItemName(entry.name.clone()),
                    InsideContainer(pocket),
                ));
                tracing::info!("Dev-spawned {} ({})", entry.name, entry.def_id);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod action_routing_tests {
    use super::*;
    use cdda_components::actor::ActionPoints;
    use cdda_components::events::ItemMoveEvent;
    use cdda_core_types::core::coords::{WorldPos, ZLevel};

    fn pos(x: i32, z: i8) -> WorldPosition {
        WorldPosition::new(WorldPos::new(x, 0, ZLevel::new(z)))
    }

    fn input(app: &mut App, action: GameAction) {
        app.world_mut().write_message(InputAction::keyboard(action));
        app.update();
    }

    #[test]
    fn dev_pickup_is_one_stable_intent_near_actor_without_camera_or_side_effects() {
        let mut app = App::new();
        app.add_message::<InputAction>()
            .add_message::<ItemMoveEvent>()
            .add_systems(Update, dev_pickup_drop_system);
        let player = app
            .world_mut()
            .spawn((
                DevPlayer,
                pos(101, 0),
                ActionPoints {
                    current: 100,
                    speed: 100,
                },
            ))
            .id();
        let far = app
            .world_mut()
            .spawn((DevGroundItemName("far".into()), pos(0, 0), SimId(0)))
            .id();
        let other_z = app
            .world_mut()
            .spawn((DevGroundItemName("upstairs".into()), pos(101, 1), SimId(1)))
            .id();
        let higher_id = app
            .world_mut()
            .spawn((DevGroundItemName("higher".into()), pos(102, 0), SimId(3)))
            .id();
        let chosen = app
            .world_mut()
            .spawn((DevGroundItemName("chosen".into()), pos(101, 0), SimId(2)))
            .id();
        input(&mut app, GameAction::Pickup);
        assert!(
            matches!(app.world().get::<ActionIntent>(player), Some(ActionIntent::Pickup { item }) if *item == chosen)
        );
        for item in [far, other_z, higher_id, chosen] {
            assert!(app.world().get::<WorldPosition>(item).is_some());
            assert!(app.world().get::<InsideContainer>(item).is_none());
            assert!(app.world().get::<WieldedBy>(item).is_none());
        }
        assert_eq!(
            app.world().get::<ActionPoints>(player).unwrap().current,
            100
        );
        assert!(app.world().resource::<Messages<ItemMoveEvent>>().is_empty());
        // A pending action survives subsequent input until the sim consumes it.
        input(&mut app, GameAction::Drop);
        assert!(
            matches!(app.world().get::<ActionIntent>(player), Some(ActionIntent::Pickup { item }) if *item == chosen)
        );
    }

    #[test]
    fn dev_drop_uses_lowest_letter_and_emits_no_transfer_or_ap_mutation() {
        let mut app = App::new();
        app.add_message::<InputAction>()
            .add_message::<ItemMoveEvent>()
            .add_systems(Update, dev_pickup_drop_system);
        let player = app
            .world_mut()
            .spawn((
                DevPlayer,
                pos(17, 0),
                ActionPoints {
                    current: 100,
                    speed: 100,
                },
            ))
            .id();
        let first = app
            .world_mut()
            .spawn((InsideContainer(player), Invlet('z')))
            .id();
        let chosen = app
            .world_mut()
            .spawn((InsideContainer(player), Invlet('a')))
            .id();
        input(&mut app, GameAction::Drop);
        assert!(
            matches!(app.world().get::<ActionIntent>(player), Some(ActionIntent::Drop { item }) if *item == chosen)
        );
        for item in [first, chosen] {
            assert_eq!(app.world().get::<InsideContainer>(item).unwrap().0, player);
            assert!(app.world().get::<WorldPosition>(item).is_none());
        }
        assert_eq!(
            app.world().get::<ActionPoints>(player).unwrap().current,
            100
        );
        assert!(app.world().resource::<Messages<ItemMoveEvent>>().is_empty());
    }

    #[test]
    fn inventory_wield_and_stow_only_declare_focused_intents() {
        let mut app = App::new();
        app.add_message::<InputAction>()
            .add_message::<ItemMoveEvent>()
            .init_resource::<InventoryFocus>()
            .init_resource::<ContextStack>()
            .init_resource::<FocusedCommandIndex>()
            .init_resource::<NextState<Ctx>>()
            .add_systems(Update, inventory_screen_input);
        let player = app
            .world_mut()
            .spawn((
                DevPlayer,
                ActionPoints {
                    current: 100,
                    speed: 100,
                },
            ))
            .id();
        let item = app
            .world_mut()
            .spawn((InsideContainer(player), Invlet('a')))
            .id();
        input(&mut app, GameAction::UseItem);
        assert!(
            matches!(app.world().get::<ActionIntent>(player), Some(ActionIntent::Wield { item: target }) if *target == item)
        );
        assert_eq!(app.world().get::<InsideContainer>(item).unwrap().0, player);
        assert!(app.world().get::<WieldedBy>(item).is_none());
        assert_eq!(
            app.world().get::<ActionPoints>(player).unwrap().current,
            100
        );
        // Model the sim's completed wield; the next adapter call must only Stow.
        app.world_mut().entity_mut(player).remove::<ActionIntent>();
        app.world_mut()
            .entity_mut(item)
            .remove::<InsideContainer>()
            .insert(WieldedBy(player));
        app.world_mut().resource_mut::<InventoryFocus>().panel = 1;
        input(&mut app, GameAction::UseItem);
        assert!(
            matches!(app.world().get::<ActionIntent>(player), Some(ActionIntent::Stow { item: target }) if *target == item)
        );
        assert_eq!(app.world().get::<WieldedBy>(item).unwrap().0, player);
        assert!(app.world().get::<InsideContainer>(item).is_none());
        assert_eq!(
            app.world().get::<ActionPoints>(player).unwrap().current,
            100
        );
        assert!(app.world().resource::<Messages<ItemMoveEvent>>().is_empty());
    }
}

use super::inventory::InventoryFocus;

/// Item-examine commands only submit intent; live simulation owns all mutations.
/// A persistent reader leaves the input stream intact for other adapters.
pub fn examine_item_input(
    mut reader: MessageReader<InputAction>,
    mut examined: ResMut<ExaminedItem>,
    players: Query<(Entity, Option<&ActionIntent>), With<DevPlayer>>,
    wielded: Query<&WieldedBy>,
    mut commands: Commands,
    mut stack: ResMut<ContextStack>,
    mut next: ResMut<NextState<Ctx>>,
    mut focused: ResMut<FocusedCommandIndex>,
) {
    let actions: Vec<_> = reader.read().map(|e| e.action.clone()).collect();
    let Some(item) = examined.0 else {
        return;
    };
    let Ok((player, pending)) = players.single() else {
        return;
    };
    for action in actions {
        let intent = match action {
            GameAction::Cancel => None,
            GameAction::Drop => Some(ActionIntent::Drop { item }),
            GameAction::UseItem => Some(if wielded.get(item).is_ok_and(|w| w.0 == player) {
                ActionIntent::Stow { item }
            } else {
                ActionIntent::Wield { item }
            }),
            GameAction::HotkeyPress('r') => Some(ActionIntent::ResumeCraft { craft: item }),
            _ => continue,
        };
        if let Some(intent) = intent {
            if pending.is_some() {
                continue;
            }
            commands.entity(player).insert(intent);
        }
        examined.0 = None;
        cdda_context::state::pop_ctx(&mut stack, &mut next, &mut focused);
        break;
    }
}
