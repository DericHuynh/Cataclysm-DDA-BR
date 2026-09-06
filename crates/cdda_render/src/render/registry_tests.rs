use super::super::scroll::*;
use super::*;

#[test]
fn headless_registry_preserves_idle_entities_and_orders_spacers() {
    let mut app = App::new();
    app.init_resource::<UiTheme>()
        .init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<RegistryViewerState>();
    app.insert_resource(RegistryCatalog {
        all_entries: vec![(0..40_000)
            .map(|i| RegistryEntry {
                id: format!("item_{i}"),
                raw_json: "{}".into(),
                parsed_fields: "fields".into(),
                status: "ok".into(),
            })
            .collect()],
        ..default()
    });
    let panel = app
        .world_mut()
        .spawn((
            RegEntryListPanel,
            RegistryPane(RegistryViewerState::PANE_ENTRY),
            RegistryListState::default(),
            RetainedRows::<RegistryRowKey>::default(),
            Node::default(),
            KeyboardScroll,
            FocusedRow(0),
            ScrollPosition::default(),
            VirtualList {
                row_height: 26.0,
                ..default()
            },
        ))
        .id();
    app.add_systems(Update, update_registry_viewer);
    app.add_systems(
        PostUpdate,
        (scroll_to_focused_row, update_virtual_windows).chain(),
    );
    for _ in 0..4 {
        app.update();
    }
    let children = app.world().get::<Children>(panel).unwrap().to_vec();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world().get::<Children>(panel).unwrap().to_vec(),
        children
    );
    assert!(children.len() < 40);
    let last = *children.last().unwrap();
    assert_eq!(app.world().get::<Node>(last).unwrap().flex_shrink, 0.0);
    assert!(
        app.world().get::<Children>(last).is_none(),
        "bottom spacer must follow rows"
    );
    app.world_mut()
        .resource_mut::<RegistryViewerState>()
        .entry_index = 20_000;
    for _ in 0..3 {
        app.update();
    }
    let list = app.world().get::<VirtualList>(panel).unwrap();
    assert!(list.window.0 <= 20_000 && list.window.1 > 20_000);
    assert!(app.world().get::<Children>(panel).unwrap().len() < 40);
}

fn fixture() -> (App, Entity, Entity, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<UiTheme>()
        .init_resource::<ButtonInput<KeyCode>>();
    spawn_registry_viewer(app.world_mut());
    app.world_mut().flush();
    app.insert_resource(RegistryCatalog {
        categories: vec![
            RegistryCategoryData {
                name: "Items".into(),
                count: 40_000,
            },
            RegistryCategoryData {
                name: "Empty".into(),
                count: 0,
            },
        ],
        all_entries: vec![
            (0..40_000)
                .map(|i| RegistryEntry {
                    id: format!("item_{i}"),
                    raw_json: format!("{{\"id\":\"item_{i}\"}}"),
                    parsed_fields: format!("Item {i}"),
                    status: "round-trip ok".into(),
                })
                .collect(),
            vec![],
        ],
    });
    app.insert_resource(RegistryViewerState {
        pane: RegistryViewerState::PANE_ENTRY,
        ..default()
    });
    let w = app.world_mut();
    let panel = w
        .query_filtered::<Entity, With<RegEntryListPanel>>()
        .single(w)
        .unwrap();
    let cats = w
        .query_filtered::<Entity, With<RegCategoryPanel>>()
        .single(w)
        .unwrap();
    let title = w
        .query_filtered::<Entity, With<RegTitleBar>>()
        .single(w)
        .unwrap();
    let raw = w
        .query_filtered::<Entity, With<RegRawJsonPanel>>()
        .single(w)
        .unwrap();
    for e in [panel, cats] {
        w.get_mut::<ComputedNode>(e).unwrap().size = Vec2::new(500.0, 360.0);
    }
    app.add_systems(Update, (registry_input, update_registry_viewer).chain());
    app.add_systems(
        PostUpdate,
        (scroll_to_focused_row, update_virtual_windows).chain(),
    );
    app.update();
    (app, panel, cats, title, raw)
}
fn children(w: &World, e: Entity) -> Vec<Entity> {
    w.get::<Children>(e).map(|c| c.to_vec()).unwrap_or_default()
}
fn texts(w: &World, e: Entity) -> Vec<String> {
    let mut result = w
        .get::<Text>(e)
        .map(|t| vec![t.0.clone()])
        .unwrap_or_default();
    for child in children(w, e) {
        result.extend(texts(w, child));
    }
    result
}
fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}

#[test]
fn registry_navigation_retains_labels_and_headers_and_clears_empty_details() {
    let (mut app, panel, cats, title, raw) = fixture();
    let rows = children(app.world(), panel);
    let categories = children(app.world(), cats);
    let headers = children(app.world(), title);
    let cells: Vec<_> = rows
        .iter()
        .flat_map(|&e| children(app.world(), e))
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
    let model_tick = app
        .world()
        .get_resource_ref::<RegistryCatalog>()
        .unwrap()
        .last_changed();
    press(&mut app, KeyCode::ArrowDown);
    assert_eq!(children(app.world(), panel), rows);
    assert_eq!(children(app.world(), cats), categories);
    assert_eq!(children(app.world(), title), headers);
    for (e, tick) in cells {
        assert_eq!(
            app.world()
                .entity(e)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
    assert!(texts(app.world(), raw).iter().any(|s| s.contains("item_1")));
    assert_eq!(
        app.world()
            .get_resource_ref::<RegistryCatalog>()
            .unwrap()
            .last_changed(),
        model_tick
    );
    let detail_children = children(app.world(), raw);
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        children(app.world(), raw),
        detail_children,
        "pane focus must not rebuild selected detail"
    );
    assert!(app.world().get::<InactiveScrollPane>(raw).is_none());
    assert!(app.world().get::<InactiveScrollPane>(panel).is_some());
    app.world_mut().resource_mut::<RegistryViewerState>().pane = RegistryViewerState::PANE_ENTRY;
    press(&mut app, KeyCode::End);
    let list = app.world().get::<VirtualList>(panel).unwrap();
    assert!(
        list.window.0 <= 39_999 && list.window.1 == 40_000,
        "selection is visible in the input frame"
    );
    press(&mut app, KeyCode::ArrowRight);
    assert_eq!(app.world().get::<VirtualList>(panel).unwrap().total_rows, 0);
    assert_eq!(app.world().get::<ScrollPosition>(panel).unwrap().y, 0.0);
    assert!(texts(app.world(), raw).contains(&"Select an entry".to_string()));
    assert!(!texts(app.world(), raw)
        .iter()
        .any(|s| s.contains("item_39999")));
    assert_eq!(children(app.world(), title), headers);
    press(&mut app, KeyCode::ArrowLeft);
    assert_eq!(app.world().get::<VirtualList>(panel).unwrap().window.0, 0);
    assert!(texts(app.world(), panel).contains(&"item_0".to_string()));
}

#[test]
fn registry_manual_scroll_recycles_rows_without_changing_details_or_idle_ticks() {
    let (mut app, panel, _, title, raw) = fixture();
    let headers = children(app.world(), title);
    let details = children(app.world(), raw);
    app.world_mut().get_mut::<ScrollPosition>(panel).unwrap().y = 26_000.0;
    app.update();
    app.update();
    let roots: std::collections::HashSet<_> = children(app.world(), panel).into_iter().collect();
    app.world_mut().get_mut::<ScrollPosition>(panel).unwrap().y = 520_000.0;
    app.update();
    app.update();
    assert_eq!(
        children(app.world(), panel)
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        roots
    );
    assert_eq!(children(app.world(), raw), details);
    assert_eq!(children(app.world(), title), headers);
    assert_eq!(
        app.world().get::<ScrollPosition>(panel).unwrap().y,
        520_000.0
    );
    let all_text: Vec<_> = app
        .world_mut()
        .query::<(Entity, Ref<Text>)>()
        .iter(app.world())
        .map(|(e, t)| (e, t.last_changed()))
        .collect();
    let bg_tick = app
        .world()
        .entity(panel)
        .get_ref::<BackgroundColor>()
        .unwrap()
        .last_changed();
    for _ in 0..10 {
        app.update();
    }
    for (e, tick) in all_text {
        assert_eq!(
            app.world()
                .entity(e)
                .get_ref::<Text>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
    assert_eq!(
        app.world()
            .entity(panel)
            .get_ref::<BackgroundColor>()
            .unwrap()
            .last_changed(),
        bg_tick
    );
    app.world_mut()
        .resource_mut::<RegistryCatalog>()
        .all_entries[0][0]
        .raw_json = "changed record".into();
    app.update();
    assert!(
        texts(app.world(), raw).contains(&"changed record".to_string()),
        "source change invalidates selected detail even without index change"
    );
}

#[test]
fn registry_source_refresh_tracks_token_changes_and_removal_by_stable_identity() {
    use cdda_data::interner::SkillRegistry;
    let mut app = App::new();
    let mut skills = SkillRegistry::default();
    skills.intern("z");
    app.insert_resource(skills).add_systems(
        Update,
        refresh_registry_catalog.run_if(registry_sources_changed),
    );
    app.update();
    let category = app
        .world()
        .resource::<RegistryCatalog>()
        .categories
        .iter()
        .position(|c| c.name == "Skills (Token)")
        .unwrap();
    app.world_mut()
        .resource_mut::<RegistryViewerState>()
        .category_index = category;
    app.update();
    let tick = app
        .world()
        .get_resource_ref::<RegistryCatalog>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .get_resource_ref::<RegistryCatalog>()
            .unwrap()
            .last_changed(),
        tick
    );
    app.world_mut().resource_mut::<SkillRegistry>().intern("a");
    app.update();
    let state = app.world().resource::<RegistryViewerState>();
    let model = app.world().resource::<RegistryCatalog>();
    assert_eq!(
        model.all_entries[state.category_index][state.entry_index].id,
        "z"
    );
    assert_eq!(state.entry_index, 1);
    app.world_mut().remove_resource::<SkillRegistry>();
    app.update();
    assert_eq!(app.world().resource::<RegistryViewerState>().entry_index, 0);
    assert!(app.world().resource::<RegistryCatalog>().all_entries[category].is_empty());
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
fn native_registry_layout_preserves_full_list_extent_and_fixed_detail_headers() {
    let (mut app, panel, _, title, raw) = fixture();
    add_native_layout(&mut app);
    for _ in 0..4 {
        app.update();
    }
    let headings: Vec<_> = app
        .world_mut()
        .query::<(Entity, &RegistryDetailHeading, &UiGlobalTransform)>()
        .iter(app.world())
        .map(|(e, _, transform)| (e, *transform))
        .collect();
    assert_eq!(headings.len(), 2);
    let title_transform = *app.world().get::<UiGlobalTransform>(title).unwrap();
    let node = app.world().get::<ComputedNode>(panel).unwrap();
    assert!(node.size().y > 300.0 && node.size().y < 720.0);
    assert_eq!(node.content_size().y, 40_000.0 * 26.0);
    press(&mut app, KeyCode::End);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<ComputedNode>(panel)
            .unwrap()
            .content_size()
            .y,
        40_000.0 * 26.0
    );
    assert_eq!(
        *app.world().get::<UiGlobalTransform>(title).unwrap(),
        title_transform
    );
    // Give raw content a known long extent without depending on font rasterization.
    let text = *children(app.world(), raw).last().unwrap();
    app.world_mut().get_mut::<Node>(text).unwrap().height = Val::Px(2000.0);
    for _ in 0..2 {
        app.update();
    }
    app.world_mut().get_mut::<ScrollPosition>(raw).unwrap().y = 400.0;
    app.update();
    assert_eq!(app.world().get::<ScrollPosition>(raw).unwrap().y, 400.0);
    for (e, transform) in headings {
        assert_eq!(*app.world().get::<UiGlobalTransform>(e).unwrap(), transform);
    }
    assert!(children(app.world(), panel).len() < 40);
}
