//! Production screen systems running without a window or GPU.
use bevy::prelude::*;
use bevy_state::prelude::{NextState, State};
use cdda_components::{actor::*, dev::DevPlayer, SkillId};
use cdda_context::{screen::CddaScreen, state::ContextActions, substate::SettingsTab};
use cdda_data::{def_world::DefinitionWorld, interner::*};
use cdda_input::{bindings::default_bindings, ActiveKeybindings};
use cdda_input::{GameAction, InputAction, InputContextStack};
use cdda_render::render::crafting_state::{CategoryIndex, CraftEntry, CraftModel, CraftState};
use cdda_render::render::{
    character::*, crafting::*, input::crafting_menu_input, scroll::*, settings, theme::UiTheme,
    UiFontHandle,
};
use cdda_sim::crafting::systems::PendingCraft;

fn base() -> App {
    let mut app = App::new();
    app.init_resource::<UiTheme>()
        .init_resource::<UiFontHandle>()
        .init_resource::<ContextActions>()
        .init_resource::<ActiveKeybindings>()
        .add_message::<InputAction>()
        .add_systems(
            PostUpdate,
            (scroll_to_focused_row, update_virtual_windows).chain(),
        );
    app
}
fn entity<T: Component>(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<T>>()
        .single(world)
        .unwrap()
}
fn children(world: &World, entity: Entity) -> Vec<Entity> {
    world
        .get::<Children>(entity)
        .map(|c| c.to_vec())
        .unwrap_or_default()
}
fn texts(world: &World, entity: Entity) -> Vec<String> {
    let mut result = Vec::new();
    if let Some(text) = world.get::<Text>(entity) {
        result.push(text.0.clone());
    }
    for child in children(world, entity) {
        result.extend(texts(world, child));
    }
    result
}
fn viewport(world: &mut World, entity: Entity) {
    world.get_mut::<ComputedNode>(entity).unwrap().size = Vec2::new(500.0, 360.0);
}
fn bounded(world: &World, entity: Entity, expected: usize) {
    let list = world.get::<VirtualList>(entity).unwrap();
    assert_eq!(list.total_rows, expected);
    assert!(
        children(world, entity).len() <= 22,
        "only viewport, overscan, and spacers should exist"
    );
    for child in children(world, entity) {
        assert_eq!(world.get::<Node>(child).unwrap().flex_shrink, 0.0);
    }
}
fn crafting() -> (App, Entity) {
    let mut app = base();
    app.init_resource::<DefinitionWorld>()
        .init_resource::<QualityRegistry>()
        .init_resource::<SkillRegistry>()
        .init_resource::<AmmoTypeRegistry>()
        .init_resource::<BodyPartRegistry>()
        .init_resource::<ComestibleRegistry>()
        .init_resource::<PendingCraft>()
        .insert_resource(InputContextStack::new())
        .add_message::<bevy::input::keyboard::KeyboardInput>();
    let entries: Vec<_> = (0..3000)
        .map(|i| CraftEntry {
            recipe_key: format!("recipe_{i}"),
            recipe_entity: app.world_mut().spawn_empty().id(),
            result_id: format!("recipe_{i}"),
            result_name: format!("Recipe {i}"),
            result_count: 1,
            craftable: i % 2 == 0,
            reason: "Missing components".into(),
            time_turns: 10,
            components_text: Vec::new(),
            qualities_text: Vec::new(),
        })
        .collect();
    let mut categories = CategoryIndex {
        top_categories: vec!["ALL".into(), "EMPTY".into()],
        ..default()
    };
    categories.sub_recipes.insert(
        ("ALL".into(), "ALL".into()),
        entries.iter().map(|e| e.recipe_entity).collect(),
    );
    app.insert_resource(categories)
        .insert_resource(CraftModel { entries })
        .init_resource::<CraftState>();
    CraftingScreen::spawn(app.world_mut());
    app.world_mut().flush();
    let pane = entity::<RecipeListContainer>(app.world_mut());
    viewport(app.world_mut(), pane);
    app.add_systems(Update, (crafting_menu_input, update_crafting_ui).chain());
    app.update();
    (app, pane)
}

#[test]
fn crafting_retains_idle_entities_and_keeps_counter_and_details_outside_scrolling_rows() {
    let (mut app, pane) = crafting();
    bounded(app.world(), pane, 3000);
    let header = entity::<HeaderContainer>(app.world_mut());
    let detail = entity::<DetailPanelContainer>(app.world_mut());
    assert!(texts(app.world(), header)
        .iter()
        .any(|s| s.contains("Recipe 1 of 3000")));
    assert!(!texts(app.world(), pane)
        .iter()
        .any(|s| s.contains(" of 3000")));
    let rows = children(app.world(), pane);
    let header_children = children(app.world(), header);
    let detail_children = children(app.world(), detail);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(children(app.world(), pane), rows);
    app.world_mut().get_mut::<ScrollPosition>(pane).unwrap().y = 20_000.0;
    app.update();
    app.update();
    bounded(app.world(), pane, 3000);
    assert_ne!(children(app.world(), pane), rows);
    assert_eq!(children(app.world(), header), header_children);
    assert_eq!(children(app.world(), detail), detail_children);
    assert_eq!(app.world().get::<ScrollPosition>(pane).unwrap().y, 20_000.0);
}

#[test]
fn crafting_navigation_confirm_filter_and_empty_category_use_the_displayed_order() {
    let (mut app, pane) = crafting();
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateEnd));
    app.update();
    let list = app.world().get::<VirtualList>(pane).unwrap();
    assert!(list.window.0 <= 2999 && list.window.1 == 3000);
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateUp));
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::Confirm));
    app.update();
    let expected = app.world().resource::<CraftModel>().entries[2998].recipe_entity;
    assert_eq!(app.world().resource::<PendingCraft>().0, Some(expected));
    {
        let mut state = app.world_mut().resource_mut::<CraftState>();
        state.filter = "recipe_42".into();
        state.focus = 0;
    }
    app.update();
    bounded(app.world(), pane, 11);
    assert_eq!(app.world().get::<ScrollPosition>(pane).unwrap().y, 0.0);
    app.world_mut().resource_mut::<CategoryIndex>().selected_top = 1;
    app.update();
    assert_eq!(app.world().get::<VirtualList>(pane).unwrap().total_rows, 0);
    assert!(texts(app.world(), pane)
        .iter()
        .any(|s| s.contains("No recipes")));
    let detail = entity::<DetailPanelContainer>(app.world_mut());
    assert!(texts(app.world(), detail).contains(&"Select a recipe".to_string()));
}

#[test]
fn character_virtualizes_skills_retains_tabs_and_updates_changed_and_removed_data() {
    let mut app = base();
    app.init_resource::<CharacterSheetState>();
    let player = app
        .world_mut()
        .spawn((
            DevPlayer,
            Health {
                current: 80,
                max: 100,
            },
        ))
        .id();
    let skills: Vec<_> = (0..3000)
        .map(|i| {
            app.world_mut()
                .spawn((
                    SkillOf(player),
                    SkillEntry {
                        skill_id: SkillId(i),
                        level: 1,
                        ..default()
                    },
                ))
                .id()
        })
        .collect();
    spawn_character_sheet_screen(app.world_mut());
    app.world_mut().flush();
    let pane = entity::<CharSheetContentContainer>(app.world_mut());
    viewport(app.world_mut(), pane);
    app.add_systems(
        Update,
        (character_sheet_input, update_character_sheet_screen).chain(),
    );
    app.update();
    bounded(app.world(), pane, 3000);
    let tabs = entity::<CharSheetTabsContainer>(app.world_mut());
    let left = entity::<CharSheetLeftContainer>(app.world_mut());
    let initial_rows = children(app.world(), pane);
    let initial_tabs = children(app.world(), tabs);
    let initial_left = children(app.world(), left);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(children(app.world(), pane), initial_rows);
    app.world_mut().resource_mut::<CharacterSheetState>().scroll = 2999;
    app.update();
    bounded(app.world(), pane, 3000);
    assert_eq!(children(app.world(), tabs), initial_tabs);
    assert_eq!(children(app.world(), left), initial_left);
    app.world_mut()
        .get_mut::<SkillEntry>(skills[2999])
        .unwrap()
        .level = 9;
    app.update();
    assert!(texts(app.world(), pane)
        .iter()
        .any(|s| s.split_whitespace().collect::<Vec<_>>() == ["skill", "#2999", "9", "0"]));
    app.world_mut().get_mut::<Health>(player).unwrap().current = 40;
    app.update();
    assert!(texts(app.world(), left).contains(&"40 / 100".to_string()));
    app.world_mut()
        .entity_mut(skills[2999])
        .remove::<SkillEntry>();
    app.update();
    bounded(app.world(), pane, 2999);
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateNextTab));
    app.update();
    assert_eq!(app.world().get::<VirtualList>(pane).unwrap().total_rows, 0);
    assert_eq!(app.world().get::<ScrollPosition>(pane).unwrap().y, 0.0);
    assert!(texts(app.world(), pane).contains(&"No traits or mutations.".to_string()));
}

#[test]
fn settings_keybindings_use_virtual_rows_and_rebuild_when_bindings_change() {
    let mut app = base();
    app.init_resource::<settings::SettingsState>()
        .insert_resource(default_bindings())
        .insert_resource(State::new(SettingsTab::Keybindings))
        .add_systems(Startup, settings::spawn)
        .add_systems(Update, settings::rebuild_content_panel);
    app.update();
    let pane = entity::<settings::ContentPanel>(app.world_mut());
    viewport(app.world_mut(), pane);
    app.update();
    app.update();
    let total = app.world().get::<VirtualList>(pane).unwrap().total_rows;
    bounded(app.world(), pane, total);
    let rows = children(app.world(), pane);
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(children(app.world(), pane), rows);
    app.world_mut()
        .resource_mut::<settings::SettingsState>()
        .focused_row = total.saturating_sub(1);
    app.update();
    assert_eq!(
        app.world().get::<VirtualList>(pane).unwrap().window.1,
        total
    );
    app.world_mut()
        .resource_mut::<cdda_input::bindings::ContextInputMaps>()
        .contexts
        .clear();
    app.update();
    assert_eq!(app.world().get::<VirtualList>(pane).unwrap().total_rows, 0);
    assert!(texts(app.world(), pane).is_empty());
    assert_eq!(children(app.world(), pane).len(), 2, "only spacers remain");
}

fn add_native_layout(app: &mut App) {
    use bevy::app::{HierarchyPropagatePlugin, PropagateSet};
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
    use bevy::ui::{
        ui_layout_system, ui_surface::UiSurface, update::propagate_ui_target_cameras,
        ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiScale,
    };
    app.add_plugins((
        HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
        HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
    ))
    .init_resource::<UiScale>()
    .init_resource::<UiSurface>()
    .init_resource::<bevy::text::TextPipeline>()
    .init_resource::<bevy::text::CosmicFontSystem>()
    .init_resource::<bevy::text::SwashCache>();
    app.add_systems(
        PostUpdate,
        (propagate_ui_target_cameras, ui_layout_system)
            .chain()
            .after(update_virtual_windows),
    );
    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default()
            .after(propagate_ui_target_cameras)
            .before(ui_layout_system),
    );
    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default()
            .after(propagate_ui_target_cameras)
            .before(ui_layout_system),
    );
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1280, 720),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));
}

#[test]
fn native_headless_layout_keeps_recipe_header_fixed_and_full_scroll_extent() {
    let (mut app, pane) = crafting();
    add_native_layout(&mut app);
    for _ in 0..4 {
        app.update();
    }
    let header = entity::<HeaderContainer>(app.world_mut());
    let header_position = *app.world().get::<UiGlobalTransform>(header).unwrap();
    let node = app.world().get::<ComputedNode>(pane).unwrap();
    assert!(node.size().y > 300.0 && node.size().y < 720.0);
    assert_eq!(node.content_size().y, 3000.0 * 36.0);
    app.world_mut().resource_mut::<CraftState>().focus = 2999;
    for _ in 0..3 {
        app.update();
    }
    let node = app.world().get::<ComputedNode>(pane).unwrap();
    assert_eq!(node.content_size().y, 3000.0 * 36.0);
    assert_eq!(
        *app.world().get::<UiGlobalTransform>(header).unwrap(),
        header_position
    );
    assert!(children(app.world(), pane).len() < 40);
}

#[test]
fn crafting_model_preserves_selection_on_inventory_change_and_writes_nothing_when_idle() {
    use cdda_components::{def::*, item::*, recipe::RecipeIndex};
    use cdda_render::render::crafting_state::{
        build_craft_state, craft_model_changed, refresh_craft_state,
    };
    use cdda_sim::crafting::systems::CraftRevision;
    let mut app = App::new();
    app.init_resource::<CraftState>()
        .init_resource::<CategoryIndex>()
        .init_resource::<CraftRevision>()
        .init_resource::<DefinitionWorld>()
        .add_systems(Update, refresh_craft_state.run_if(craft_model_changed));
    let w = app.world_mut();
    let player = w.spawn(DevPlayer).id();
    let token = cdda_components::ItemTypeId(1);
    let ingredient = w
        .spawn((
            ItemType(token),
            StackCount::new(2).unwrap(),
            InsideContainer(player),
        ))
        .id();
    let mut recipes = Vec::new();
    for name in ["A", "B"] {
        let item = w.spawn(ItemName(name.into())).id();
        w.resource_mut::<DefinitionWorld>().register(
            cdda_data::def_world::DefCategory::Item,
            name.into(),
            item,
        );
        recipes.push(
            w.spawn((
                DefStrId(format!("recipe_{name}")),
                RecipeResult(name.into()),
                RecipeCategory("CC_TEST".into()),
                RecipeSubcategory("CSC_TEST_ALL".into()),
            ))
            .id(),
        );
    }
    w.entity_mut(recipes[0])
        .insert(RecipeComponents(vec![vec![RecipeComponentEntry {
            item_id: token,
            count: 2,
            recovered: false,
        }]]));
    w.insert_resource(RecipeIndex(recipes));
    build_craft_state(w);
    app.update();
    app.update();
    let before = app
        .world()
        .get_resource_ref::<CraftState>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .get_resource_ref::<CraftState>()
            .unwrap()
            .last_changed(),
        before
    );
    app.world_mut().despawn(ingredient);
    app.update();
    let state = app.world().resource::<CraftState>();
    assert_eq!(
        app.world().resource::<CraftModel>().entries[state.focus].recipe_key,
        "recipe_A"
    );
    assert!(!app.world().resource::<CraftModel>().entries[state.focus].craftable);
    assert_eq!(
        state.focus, 1,
        "selection follows the recipe when sorting changes"
    );
}

#[test]
fn crafting_focus_retains_rows_labels_tabs_and_read_model() {
    let (mut app, pane) = crafting();
    let header = entity::<HeaderContainer>(app.world_mut());
    let tabs = entity::<CategoryTabsContainer>(app.world_mut());
    let subtabs = entity::<SubcategoryTabsContainer>(app.world_mut());
    let skeleton: Vec<_> = [pane, header, tabs, subtabs]
        .into_iter()
        .map(|e| (e, children(app.world(), e)))
        .collect();
    let key = Some("recipe_1".to_string());
    let row = app
        .world()
        .get::<cdda_ui::RetainedRows<Option<String>>>(pane)
        .unwrap()
        .entity(&key)
        .unwrap();
    let cells = children(app.world(), row);
    let ticks: Vec<_> = cells
        .iter()
        .map(|&e| {
            app.world()
                .entity(e)
                .get_ref::<Text>()
                .unwrap()
                .last_changed()
        })
        .collect();
    let model_tick = app
        .world()
        .get_resource_ref::<CraftModel>()
        .unwrap()
        .last_changed();
    let old_bg = app.world().get::<BackgroundColor>(row).unwrap().0;
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateDown));
    app.update();
    for (entity, expected) in skeleton {
        assert_eq!(children(app.world(), entity), expected);
    }
    assert_eq!(children(app.world(), row), cells);
    for (cell, tick) in cells.into_iter().zip(ticks) {
        assert_eq!(
            app.world()
                .entity(cell)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
    assert_ne!(app.world().get::<BackgroundColor>(row).unwrap().0, old_bg);
    assert_eq!(
        app.world()
            .get_resource_ref::<CraftModel>()
            .unwrap()
            .last_changed(),
        model_tick
    );
    assert!(texts(app.world(), header)
        .iter()
        .any(|t| t.contains("Recipe 2 of 3000")));
}

#[test]
fn recipe_membership_cache_ignores_selection_but_tracks_filter_category_and_model() {
    use cdda_render::render::crafting_state::RecipeFilter;
    let (app, _) = crafting();
    let model = app.world().resource::<CraftModel>();
    let mut categories = app.world().resource::<CategoryIndex>().clone();
    let mut state = CraftState::default();
    let mut cache = RecipeFilter::default();
    assert!(cache.update(model, &state, &categories, (1, 1)));
    for focus in 0..3000 {
        state.focus = focus;
        state.filtering = focus % 2 == 0;
        assert!(!cache.update(model, &state, &categories, (1, 1)));
    }
    assert_eq!(cache.rebuilds, 1);
    state.filter = "recipe_42".into();
    assert!(cache.update(model, &state, &categories, (1, 1)));
    assert_eq!(cache.indices.len(), 11);
    state.show_all = false;
    assert!(cache.update(model, &state, &categories, (1, 1)));
    assert!(cache.indices.iter().all(|&i| model.entries[i].craftable));
    categories.selected_top = 1;
    assert!(cache.update(model, &state, &categories, (1, 1)));
    assert!(cache.indices.is_empty());
    assert!(
        cache.update(model, &state, &categories, (2, 1)),
        "model publication invalidates membership"
    );
}

#[test]
fn spawn_catalog_refreshes_replaced_definitions_and_preserves_stable_selection() {
    use cdda_components::def::{DefStrId, IsDef, ItemName};
    use cdda_render::render::dev_spawn::{
        dev_spawn_catalog_changed, dev_spawn_populate, DevSpawnCatalog, DevSpawnFocus,
    };
    let mut app = App::new();
    app.init_resource::<DevSpawnFocus>()
        .init_resource::<DevSpawnCatalog>()
        .add_systems(Update, dev_spawn_populate.run_if(dev_spawn_catalog_changed));
    let a = app
        .world_mut()
        .spawn((IsDef, DefStrId("a".into()), ItemName("A".into())))
        .id();
    let b = app
        .world_mut()
        .spawn((IsDef, DefStrId("b".into()), ItemName("B".into())))
        .id();
    app.update();
    app.world_mut().resource_mut::<DevSpawnFocus>().index = 1;
    app.update();
    let tick = app
        .world()
        .get_resource_ref::<DevSpawnFocus>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .get_resource_ref::<DevSpawnFocus>()
            .unwrap()
            .last_changed(),
        tick
    );
    app.world_mut().despawn(b);
    let replacement = app
        .world_mut()
        .spawn((IsDef, DefStrId("b".into()), ItemName("0B".into())))
        .id();
    app.update();
    let focus = app.world().resource::<DevSpawnFocus>();
    assert_eq!(focus.index, 0);
    assert_eq!(
        app.world().resource::<DevSpawnCatalog>().entries[focus.index].def_entity,
        replacement
    );
    app.world_mut().entity_mut(replacement).remove::<ItemName>();
    app.update();
    let focus = app.world().resource::<DevSpawnFocus>();
    assert_eq!(app.world().resource::<DevSpawnCatalog>().entries.len(), 1);
    assert_eq!(
        app.world().resource::<DevSpawnCatalog>().entries[focus.index].def_entity,
        a
    );
}

fn spawn_menu() -> (App, Entity) {
    use cdda_render::render::dev_spawn::*;
    let mut app = base();
    app.init_resource::<QualityRegistry>()
        .init_resource::<SkillRegistry>()
        .init_resource::<AmmoTypeRegistry>()
        .init_resource::<BodyPartRegistry>()
        .init_resource::<ComestibleRegistry>()
        .init_resource::<ItemTypeRegistry>()
        .init_resource::<DevSpawnFocus>()
        .insert_resource(InputContextStack::new())
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Startup, spawn_dev_spawn_panel)
        .add_systems(
            Update,
            (
                cdda_render::render::input::dev_spawn_input,
                update_dev_spawn_panel,
            )
                .chain(),
        );
    let entries = (0..40_000)
        .map(|i| DevCatalogEntry {
            def_entity: app.world_mut().spawn_empty().id(),
            name: format!("Item {i}"),
            def_id: format!("item_{i}"),
        })
        .collect();
    app.insert_resource(DevSpawnCatalog { entries });
    app.update();
    let pane = entity::<SpawnListPanel>(app.world_mut());
    viewport(app.world_mut(), pane);
    app.update();
    app.update();
    (app, pane)
}

#[test]
fn spawn_menu_retains_text_and_details_on_scroll_and_recycles_large_windows() {
    use cdda_render::render::dev_spawn::*;
    let (mut app, pane) = spawn_menu();
    bounded(app.world(), pane, 40_000);
    let title = entity::<SpawnTitleBar>(app.world_mut());
    let detail = entity::<SpawnDetailPanel>(app.world_mut());
    let title_children = children(app.world(), title);
    let row = app
        .world()
        .get::<cdda_ui::RetainedRows<Option<String>>>(pane)
        .unwrap()
        .entity(&Some("item_1".into()))
        .unwrap();
    let cells = children(app.world(), row);
    let ticks: Vec<_> = cells
        .iter()
        .map(|&e| {
            app.world()
                .entity(e)
                .get_ref::<Text>()
                .unwrap()
                .last_changed()
        })
        .collect();
    let catalog_tick = app
        .world()
        .get_resource_ref::<DevSpawnCatalog>()
        .unwrap()
        .last_changed();
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateDown));
    app.update();
    assert_eq!(children(app.world(), title), title_children);
    assert_eq!(children(app.world(), row), cells);
    for (cell, tick) in cells.into_iter().zip(ticks) {
        assert_eq!(
            app.world()
                .entity(cell)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
    let detail_children = children(app.world(), detail);
    app.world_mut().get_mut::<ScrollPosition>(pane).unwrap().y = 48_000.0;
    app.update();
    app.update();
    let roots: std::collections::HashSet<_> = children(app.world(), pane).into_iter().collect();
    app.world_mut().get_mut::<ScrollPosition>(pane).unwrap().y = 960_000.0;
    app.update();
    app.update();
    assert_eq!(
        children(app.world(), pane)
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        roots
    );
    assert_eq!(children(app.world(), detail), detail_children);
    assert_eq!(children(app.world(), title), title_children);
    assert_eq!(
        app.world().get::<ScrollPosition>(pane).unwrap().y,
        960_000.0
    );
    assert_eq!(
        app.world()
            .get_resource_ref::<DevSpawnCatalog>()
            .unwrap()
            .last_changed(),
        catalog_tick
    );
    let snapshot = children(app.world(), pane);
    let text_ticks: Vec<_> = snapshot
        .iter()
        .flat_map(|&row| children(app.world(), row))
        .map(|e| {
            (
                e,
                app.world()
                    .entity(e)
                    .get_ref::<Text>()
                    .unwrap()
                    .last_changed(),
            )
        })
        .collect();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(children(app.world(), pane), snapshot);
    for (e, tick) in text_ticks {
        assert_eq!(
            app.world()
                .entity(e)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
}

#[test]
fn spawn_filter_navigation_confirm_and_empty_catalog_use_current_membership() {
    use cdda_components::def::ItemName;
    use cdda_render::render::dev_spawn::*;
    let (mut app, pane) = spawn_menu();
    app.world_mut().spawn(DevPlayer);
    app.world_mut().resource_mut::<DevSpawnFocus>().filter = "item_42".into();
    app.update();
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateEnd));
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::Confirm));
    app.update();
    let list = app.world().get::<VirtualList>(pane).unwrap();
    let focus = app.world().resource::<DevSpawnFocus>().index;
    assert!(
        list.window.0 <= focus && focus < list.window.1,
        "selection revealed in the input frame"
    );
    let mut items = app.world_mut().query::<&ItemName>();
    assert!(items.iter(app.world()).any(|name| name.0 == "Item 4299"));
    let mut cache = SpawnFilter::default();
    let model = app.world().resource::<DevSpawnCatalog>();
    for _ in 0..3000 {
        cache.update(model, "item_42", 1);
    }
    assert_eq!(cache.rebuilds, 1);
    cache.update(model, "item_100", 1);
    assert_eq!(cache.rebuilds, 2);
    cache.update(model, "item_100", 2);
    assert_eq!(cache.rebuilds, 3);
    app.world_mut()
        .resource_mut::<DevSpawnCatalog>()
        .entries
        .clear();
    app.world_mut().resource_mut::<DevSpawnFocus>().index = 0;
    app.update();
    bounded(app.world(), pane, 0);
    assert_eq!(app.world().get::<ScrollPosition>(pane).unwrap().y, 0.0);
    assert!(texts(app.world(), pane)
        .iter()
        .any(|s| s == "No item definitions"));
    let detail = entity::<SpawnDetailPanel>(app.world_mut());
    assert_eq!(texts(app.world(), detail), vec!["Select an item"]);
}

#[test]
fn native_headless_spawn_layout_keeps_header_fixed_and_full_scroll_extent() {
    use cdda_render::render::dev_spawn::*;
    let (mut app, pane) = spawn_menu();
    add_native_layout(&mut app);
    for _ in 0..4 {
        app.update();
    }
    let title = entity::<SpawnTitleBar>(app.world_mut());
    let transform = *app.world().get::<UiGlobalTransform>(title).unwrap();
    let node = app.world().get::<ComputedNode>(pane).unwrap();
    assert!(node.size().y > 300.0 && node.size().y < 720.0);
    assert_eq!(node.content_size().y, 40_000.0 * 48.0);
    app.world_mut().resource_mut::<DevSpawnFocus>().index = 39_999;
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<ComputedNode>(pane)
            .unwrap()
            .content_size()
            .y,
        40_000.0 * 48.0
    );
    assert_eq!(
        *app.world().get::<UiGlobalTransform>(title).unwrap(),
        transform
    );
    assert!(children(app.world(), pane).len() < 40);
}

#[test]
fn crafting_model_updates_when_nested_inventory_is_sealed_or_unmounted() {
    use cdda_components::{def::*, item::*, recipe::RecipeIndex};
    use cdda_render::render::crafting_state::{
        build_craft_state, craft_model_changed, refresh_craft_state,
    };
    let mut app = App::new();
    app.init_resource::<DefinitionWorld>()
        .add_systems(Update, refresh_craft_state.run_if(craft_model_changed));
    let w = app.world_mut();
    let player = w.spawn(DevPlayer).id();
    let bag = w.spawn(InsideContainer(player)).id();
    let pocket = w.spawn((IsPocket, MountedOn(bag))).id();
    let token = cdda_components::ItemTypeId(1);
    w.spawn((
        ItemType(token),
        StackCount::new(2).unwrap(),
        InsideContainer(pocket),
    ));
    let result = w.spawn(ItemName("cord".into())).id();
    w.resource_mut::<DefinitionWorld>().register(
        cdda_data::def_world::DefCategory::Item,
        "cord".into(),
        result,
    );
    let recipe = w
        .spawn((
            DefStrId("recipe_cord".into()),
            RecipeResult("cord".into()),
            RecipeCategory("CC_TEST".into()),
            RecipeSubcategory("CSC_TEST_ALL".into()),
            RecipeComponents(vec![vec![RecipeComponentEntry {
                item_id: token,
                count: 2,
                recovered: false,
            }]]),
        ))
        .id();
    w.insert_resource(RecipeIndex(vec![recipe]));
    build_craft_state(w);
    app.update();
    assert!(app.world().resource::<CraftModel>().entries[0].craftable);
    app.world_mut().entity_mut(bag).insert(Sealed);
    app.update();
    assert!(!app.world().resource::<CraftModel>().entries[0].craftable);
    app.world_mut().entity_mut(bag).remove::<Sealed>();
    app.update();
    assert!(app.world().resource::<CraftModel>().entries[0].craftable);
    app.world_mut().entity_mut(pocket).remove::<MountedOn>();
    app.update();
    assert!(!app.world().resource::<CraftModel>().entries[0].craftable);
}

#[test]
fn character_combat_capabilities_update_and_remove_independently() {
    let mut app = base();
    app.init_resource::<CharacterSheetState>();
    let player = app
        .world_mut()
        .spawn((
            DevPlayer,
            CombatStats {
                melee_skill: 3,
                melee_dice: 2,
                melee_dice_sides: 6,
                dodge: 4,
                armor: DamageReduction {
                    bash: 5,
                    ..Default::default()
                },
            }
            .into_bundle(),
        ))
        .id();
    spawn_character_sheet_screen(app.world_mut());
    app.world_mut().flush();
    let pane = entity::<CharSheetContentContainer>(app.world_mut());
    viewport(app.world_mut(), pane);
    app.add_systems(Update, update_character_sheet_screen);
    app.update();
    let left = entity::<CharSheetLeftContainer>(app.world_mut());
    assert!(texts(app.world(), left).contains(&"2d6 (skill 3)".into()));
    assert!(texts(app.world(), left).contains(&"bash 5 / cut 0 / pierce 0".into()));
    app.world_mut()
        .entity_mut(player)
        .remove::<MeleeCapability>();
    app.update();
    assert!(!texts(app.world(), left).contains(&"2d6 (skill 3)".into()));
    assert!(
        texts(app.world(), left).contains(&"4".into()),
        "dodge survives attack removal"
    );
    app.world_mut().get_mut::<DodgeDefense>(player).unwrap().0 = 8;
    app.update();
    assert!(texts(app.world(), left).contains(&"8".into()));
    app.world_mut()
        .entity_mut(player)
        .remove::<IntrinsicArmor>();
    app.update();
    assert!(!texts(app.world(), left).contains(&"bash 5 / cut 0 / pierce 0".into()));
    app.world_mut().entity_mut(player).remove::<DodgeDefense>();
    app.update();
    assert!(!texts(app.world(), left).contains(&"8".into()));
    let retained = children(app.world(), left);
    app.update();
    assert_eq!(
        children(app.world(), left),
        retained,
        "idle overview retains its entities"
    );
}

#[test]
fn examine_adapter_submits_one_command_without_mutating_items_or_draining_input() {
    use cdda_components::{
        intent::ActionIntent, item::InsideContainer, schedule::GameSet, sim::WorldPosition,
    };
    use cdda_context::state::{ContextStack, Ctx, FocusedCommandIndex};
    use cdda_sim::{
        inventory::examine_resource::ExaminedItem,
        runtime::{step_simulation, SimulationControl, SimulationMode, SimulationPlugin},
    };
    let mut app = App::new();
    app.add_plugins(SimulationPlugin)
        .init_resource::<ContextStack>()
        .init_resource::<FocusedCommandIndex>()
        .init_resource::<NextState<Ctx>>()
        .init_resource::<ExaminedItem>()
        .add_message::<InputAction>()
        .add_systems(
            Update,
            cdda_render::render::input::examine_item_input.in_set(GameSet::Input),
        );
    app.world_mut().resource_mut::<SimulationControl>().mode = SimulationMode::Manual;
    let position = cdda_core_types::core::coords::WorldPos::new(
        7,
        9,
        cdda_core_types::core::coords::ZLevel::new(0),
    );
    let player = app
        .world_mut()
        .spawn((
            DevPlayer,
            ActionPoints {
                current: 0,
                speed: 100,
            },
            WorldPosition(position),
        ))
        .id();
    let item = app.world_mut().spawn(InsideContainer(player)).id();
    app.world_mut().resource_mut::<ExaminedItem>().0 = Some(item);
    for action in [GameAction::Drop, GameAction::UseItem] {
        app.world_mut().write_message(InputAction::keyboard(action));
    }
    app.update();
    assert!(
        matches!(app.world().get::<ActionIntent>(player), Some(ActionIntent::Drop { item: target }) if *target == item)
    );
    assert_eq!(app.world().get::<InsideContainer>(item).unwrap().0, player);
    assert_eq!(app.world().get::<ActionPoints>(player).unwrap().current, 0);
    let mut cursor = bevy_ecs::message::MessageCursor::<InputAction>::default();
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<InputAction>>())
            .count(),
        2
    );
    step_simulation(app.world_mut());
    assert!(app.world().get::<InsideContainer>(item).is_none());
    assert_eq!(
        app.world().get::<WorldPosition>(item).unwrap().get(),
        position
    );
    assert_eq!(app.world().get::<ActionPoints>(player).unwrap().current, 0);
    // Existing requests are retained, including when the resume hotkey arrives.
    app.world_mut()
        .entity_mut(player)
        .insert(ActionIntent::Wait);
    app.world_mut().resource_mut::<ExaminedItem>().0 = Some(item);
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::HotkeyPress('r')));
    app.update();
    assert!(matches!(
        app.world().get::<ActionIntent>(player),
        Some(ActionIntent::Wait)
    ));
    assert_eq!(app.world().resource::<ExaminedItem>().0, Some(item));
}
