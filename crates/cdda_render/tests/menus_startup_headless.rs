//! Menus and operation reporting through persistent systems without a window/GPU.
use bevy::prelude::*;
use bevy_state::prelude::{NextState, State};
use cdda_components::progress::*;
use cdda_context::{ctx::Ctx, nav::FocusedCommandIndex, substate::SettingsTab, InputFocus};
use cdda_input::{bindings::default_bindings, ActionSource, GameAction, InputAction};
use cdda_render::render::{loading, main_menu, settings, theme::UiTheme, UiFontHandle};

#[test]
fn loading_errors_and_known_progress_update_in_place_without_idle_writes() {
    let mut app = App::new();
    app.init_resource::<OperationReport>()
        .init_resource::<UiTheme>()
        .add_systems(Update, loading::update);
    let text = app
        .world_mut()
        .spawn((loading::LoadingText, Text::new("Starting…")))
        .id();
    let bar = app
        .world_mut()
        .spawn((loading::LoadingBar::default(), Node::default()))
        .id();
    let retry = app
        .world_mut()
        .spawn((loading::RetryButton, Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<OperationReport>()
        .record(ReportEvent::progress("Reading JSON", "items.json").units(2, 4));
    app.update();
    assert_eq!(
        app.world().get::<loading::LoadingBar>(bar).unwrap().target,
        Some(0.5)
    );
    assert_eq!(
        app.world().get::<Node>(retry).unwrap().display,
        Display::None
    );
    let tick = app
        .world()
        .entity(text)
        .get_ref::<Text>()
        .unwrap()
        .last_changed();
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world()
            .entity(text)
            .get_ref::<Text>()
            .unwrap()
            .last_changed(),
        tick
    );
    app.world_mut().resource_mut::<OperationReport>().record(
        ReportEvent::progress("Loading failed", "items.json: invalid JSON")
            .level(ReportLevel::Error),
    );
    app.update();
    assert!(app
        .world()
        .get::<Text>(text)
        .unwrap()
        .0
        .contains("items.json: invalid JSON"));
    assert_eq!(
        app.world().get::<Node>(retry).unwrap().display,
        Display::Flex
    );
    assert_eq!(
        app.world().get::<loading::LoadingBar>(bar).unwrap().target,
        None
    );
}

#[test]
fn graphics_options_apply_scale_and_window_mode_and_preferences_validate_values() {
    let mut app = App::new();
    app.init_resource::<settings::SettingsState>()
        .init_resource::<UiTheme>()
        .init_resource::<UiScale>()
        .insert_resource(State::new(SettingsTab::Graphics))
        .init_resource::<NextState<SettingsTab>>()
        .insert_resource(default_bindings())
        .add_message::<InputAction>()
        .add_systems(
            Update,
            (settings::navigate, settings::apply_display_options).chain(),
        );
    let window = app
        .world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow))
        .id();
    app.world_mut().write_message(InputAction::new(
        GameAction::NavigateRight,
        ActionSource::Keyboard,
    ));
    app.update();
    assert_eq!(app.world().resource::<UiScale>().0, 1.1);
    app.world_mut()
        .resource_mut::<settings::SettingsState>()
        .focused_row = 1;
    app.world_mut().write_message(InputAction::new(
        GameAction::NavigateRight,
        ActionSource::Keyboard,
    ));
    app.update();
    assert!(matches!(
        app.world().get::<Window>(window).unwrap().mode,
        bevy::window::WindowMode::BorderlessFullscreen(_)
    ));
    let preferences: settings::DisplayPreferences =
        serde_json::from_str(r#"{"scale_percent":500,"theme":99}"#).unwrap();
    preferences.apply(&mut app.world_mut().resource_mut::<settings::SettingsState>());
    assert_eq!(
        app.world()
            .resource::<settings::SettingsState>()
            .ui_scale_percent,
        150
    );
    assert!(
        app.world()
            .resource::<settings::SettingsState>()
            .interface_theme
            < 3
    );
}

#[test]
fn shared_menu_frame_contains_real_art_and_commands_without_idle_focus_writes() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
    app.init_asset::<Image>();
    app.insert_resource(State::new(Ctx::MainMenu))
        .init_resource::<FocusedCommandIndex>()
        .init_resource::<InputFocus>()
        .init_resource::<cdda_input::ActiveKeybindings>()
        .init_resource::<cdda_context::state::ContextActions>()
        .init_resource::<UiFontHandle>()
        .init_resource::<UiTheme>()
        .init_resource::<settings::SettingsState>()
        .add_message::<InputAction>()
        .add_systems(Startup, main_menu::spawn)
        .add_systems(
            Update,
            (
                main_menu::sync_focus,
                cdda_render::render::refresh_all_footer_hints,
            )
                .chain(),
        );
    app.update();
    let world = app.world_mut();
    assert_eq!(world.query::<&ImageNode>().iter(world).count(), 1);
    assert_eq!(
        world
            .query::<&main_menu::CommandButton>()
            .iter(world)
            .count(),
        9
    );
    let footer = world
        .query_filtered::<&Text, With<main_menu::MenuFooter>>()
        .single(world)
        .unwrap();
    assert!(footer.0.contains("navigate") && footer.0.contains("select"));
    assert!(!footer.0.contains("close"));
    let focused = world.resource::<InputFocus>().entity.unwrap();
    let tick = world
        .entity(focused)
        .get_ref::<BorderColor>()
        .unwrap()
        .last_changed();
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world()
            .entity(focused)
            .get_ref::<BorderColor>()
            .unwrap()
            .last_changed(),
        tick
    );
}

#[test]
fn illustrated_regions_preserve_art_and_leave_controls_clear_at_supported_scales() {
    use cdda_render::render::cinematic::{region_rect, ScreenRegion::*};
    for viewport in [
        Vec2::new(2048., 1205.),
        Vec2::new(1600., 900.),
        Vec2::new(1280., 720.),
        Vec2::new(800., 600.),
    ] {
        for scale in [0.7, 1., 1.5] {
            let viewport = viewport / scale;
            let panel = region_rect(LoadingContent, viewport);
            let hub = region_rect(LoadingArt, viewport);
            assert!(
                hub.max.y + 15. <= panel.min.y,
                "Hub must stay above controls"
            );
            assert!((hub.width() / hub.height() - 1920. / 1460.).abs() < 0.001);
            assert!(panel.min.x >= 0. && panel.max.x <= viewport.x);
            assert!(panel.max.y <= viewport.y);
            assert!(panel.height() >= 190. && panel.height() <= 240.);
            let menu = region_rect(MenuContent, viewport);
            let art = region_rect(MenuArt, viewport);
            assert!(menu.width() <= 460.);
            if art.width() > 0. {
                assert!(menu.max.x < art.min.x);
                assert!((art.width() / art.height() - 1365. / 1024.).abs() < 0.001);
            }
        }
    }
}

#[test]
fn motion_settles_and_indeterminate_progress_stops_on_failure() {
    use cdda_render::render::cinematic;
    use std::time::Duration;
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<OperationReport>()
        .init_resource::<UiTheme>()
        .add_systems(
            Update,
            (
                loading::update,
                loading::animate_progress,
                cinematic::animate_accents,
            )
                .chain(),
        );
    let button = app
        .world_mut()
        .spawn((Button, cinematic::AccentMotion::default()))
        .id();
    let bar = app
        .world_mut()
        .spawn((Node::default(), loading::LoadingBar::default()))
        .id();
    app.world_mut()
        .resource_mut::<OperationReport>()
        .record(ReportEvent::progress("Discovering", "Files"));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(0.1));
    app.update();
    let old_color = app.world().get::<BackgroundColor>(button).unwrap().0;
    *app.world_mut().get_mut::<Interaction>(button).unwrap() = Interaction::Hovered;
    let first = *app.world().get::<UiTransform>(bar).unwrap();
    app.update();
    assert_ne!(*app.world().get::<UiTransform>(bar).unwrap(), first);
    assert_ne!(
        app.world().get::<BackgroundColor>(button).unwrap().0,
        old_color
    );
    assert_eq!(
        app.world().get::<loading::LoadingBar>(bar).unwrap().target,
        None
    );
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        *app.world().get::<UiTransform>(button).unwrap(),
        UiTransform::IDENTITY
    );
    let tick = app
        .world()
        .entity(button)
        .get_ref::<BackgroundColor>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(button)
            .get_ref::<BackgroundColor>()
            .unwrap()
            .last_changed(),
        tick
    );
    app.world_mut()
        .resource_mut::<OperationReport>()
        .record(ReportEvent::progress("Parsing", "items").units(1, 2));
    for _ in 0..20 {
        app.update();
    }
    assert!((app.world().get::<UiTransform>(bar).unwrap().scale.x - 0.5).abs() < 0.001);
    app.world_mut()
        .resource_mut::<OperationReport>()
        .record(ReportEvent::progress("Parsing", "invalid JSON").level(ReportLevel::Error));
    app.update();
    assert_eq!(app.world().get::<UiTransform>(bar).unwrap().scale.x, 0.);
    let tick = app
        .world()
        .entity(bar)
        .get_ref::<UiTransform>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(bar)
            .get_ref::<UiTransform>()
            .unwrap()
            .last_changed(),
        tick
    );
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
        (propagate_ui_target_cameras, ui_layout_system).chain(),
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
fn native_loading_layout_keeps_diagnostics_above_a_fixed_thin_track_and_controls() {
    use cdda_render::render::cinematic::{self, ScreenRegion};
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<UiFontHandle>()
        .init_resource::<OperationReport>()
        .init_resource::<UiTheme>()
        .add_message::<OperationCommand>()
        .add_systems(Startup, loading::spawn)
        .add_systems(Update, (cinematic::layout, loading::update));
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1280, 720).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    add_native_layout(&mut app);
    for scale in [0.7, 1., 1.5] {
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        for _ in 0..4 {
            app.update();
        }
        let world = app.world_mut();
        let mut regions = world.query::<(&ScreenRegion, &ComputedNode, &UiGlobalTransform)>();
        let bounds: Vec<_> = regions
            .iter(world)
            .map(|(region, node, transform)| {
                (
                    *region,
                    Rect::from_center_size(transform.translation, node.size()),
                )
            })
            .collect();
        let art = bounds
            .iter()
            .find(|(r, _)| matches!(r, ScreenRegion::LoadingArt))
            .unwrap()
            .1;
        let panel = bounds
            .iter()
            .find(|(r, _)| matches!(r, ScreenRegion::LoadingContent))
            .unwrap()
            .1;
        assert!(art.max.y < panel.min.y);
        assert!(
            (art.width() - art.height() * 1920. / 1460.).abs() < 2.,
            "rounded artwork bounds: {art:?}"
        );
        let track = world
            .query_filtered::<Entity, With<loading::LoadingTrack>>()
            .single(world)
            .unwrap();
        let track_node = world.get::<ComputedNode>(track).unwrap();
        assert!((track_node.size().y - 2. * scale).abs() <= 1.);
        let before = *world.get::<UiGlobalTransform>(track).unwrap();
        for i in 0..30 {
            world.resource_mut::<OperationReport>().record(
                ReportEvent::progress(
                    "Warning",
                    format!("Problem in file {i}: {}", "detailed diagnostic ".repeat(30)),
                )
                .level(ReportLevel::Warning),
            );
        }
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            *app.world().get::<UiGlobalTransform>(track).unwrap(),
            before,
            "diagnostics cannot move the track"
        );
        assert!(app.world().get::<Window>(window).is_some());
    }
}

#[test]
fn native_scaled_command_menu_reveals_last_selection_without_moving_art_or_footer() {
    use cdda_render::render::{cinematic, scroll};
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<UiFontHandle>()
        .insert_resource(State::new(Ctx::MainMenu))
        .init_resource::<settings::SettingsState>()
        .init_resource::<UiTheme>()
        .init_resource::<FocusedCommandIndex>()
        .init_resource::<InputFocus>()
        .init_resource::<cdda_input::ActiveKeybindings>()
        .add_message::<InputAction>()
        .add_systems(Startup, main_menu::spawn)
        .add_systems(Update, (cinematic::layout, main_menu::sync_focus))
        .add_systems(
            PostUpdate,
            (
                scroll::scroll_to_focused_row,
                scroll::update_virtual_windows,
            )
                .chain()
                .before(bevy::ui::ui_layout_system),
        );
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    add_native_layout(&mut app);
    app.world_mut().resource_mut::<UiScale>().0 = 1.5;
    for _ in 0..4 {
        app.update();
    }
    let world = app.world_mut();
    let pane = world
        .query_filtered::<Entity, With<main_menu::CommandPane>>()
        .single(world)
        .unwrap();
    let footer = world
        .query_filtered::<Entity, With<main_menu::MenuFooter>>()
        .single(world)
        .unwrap();
    let art = world
        .query_filtered::<Entity, With<main_menu::MenuArtwork>>()
        .single(world)
        .unwrap();
    let footer_before = *world.get::<UiGlobalTransform>(footer).unwrap();
    let art_before = *world.get::<UiGlobalTransform>(art).unwrap();
    world.resource_mut::<FocusedCommandIndex>().set(8);
    for _ in 0..4 {
        app.update();
    }
    let world = app.world();
    let selected = world.resource::<InputFocus>().entity.unwrap();
    let bounds = |e| {
        Rect::from_center_size(
            world.get::<UiGlobalTransform>(e).unwrap().translation,
            world.get::<ComputedNode>(e).unwrap().size(),
        )
    };
    let row_bounds = bounds(selected);
    let pane_bounds = bounds(pane);
    assert!(
        row_bounds.min.y >= pane_bounds.min.y - 1. && row_bounds.max.y <= pane_bounds.max.y + 1.
    );
    assert!(world.get::<ScrollPosition>(pane).unwrap().y > 0.);
    assert_eq!(
        *world.get::<UiGlobalTransform>(footer).unwrap(),
        footer_before
    );
    assert_eq!(*world.get::<UiGlobalTransform>(art).unwrap(), art_before);
}

#[test]
fn every_command_menu_and_loading_repaint_live_without_moving_buttons_or_rewriting_text() {
    use cdda_render::render::{
        cinematic,
        theme::{self, ThemePreset},
    };
    let screens = std::iter::once(Some(Ctx::MainMenu))
        .chain(main_menu::COMMAND_MENUS.iter().copied().map(Some))
        .chain(std::iter::once(None));
    for screen in screens {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<UiFontHandle>()
            .init_resource::<UiTheme>()
            .init_resource::<settings::SettingsState>()
            .init_resource::<FocusedCommandIndex>()
            .init_resource::<InputFocus>()
            .init_resource::<cdda_input::ActiveKeybindings>()
            .init_resource::<OperationReport>()
            .add_message::<InputAction>()
            .add_message::<OperationCommand>()
            .add_systems(Update, cinematic::animate_accents)
            .add_systems(PostUpdate, theme::apply_palette);
        if let Some(screen) = screen {
            app.insert_resource(State::new(screen))
                .add_systems(Startup, main_menu::spawn)
                .add_systems(
                    Update,
                    main_menu::sync_focus.before(cinematic::animate_accents),
                );
        } else {
            app.add_systems(Startup, loading::spawn);
        }
        app.update();
        let world = app.world_mut();
        let original_text: Vec<_> = world
            .query_filtered::<Entity, With<Text>>()
            .iter(world)
            .map(|e| (e, world.entity(e).get_ref::<Text>().unwrap().last_changed()))
            .collect();
        let original_nodes: Vec<_> = world
            .query_filtered::<(Entity, &Node), With<Button>>()
            .iter(world)
            .map(|(e, n)| (e, n.clone()))
            .collect();
        assert!(!original_nodes.is_empty());
        for preset in ThemePreset::ALL {
            app.world_mut().resource_mut::<UiTheme>().preset = preset;
            app.update();
            let world = app.world_mut();
            let palette = world.resource::<UiTheme>().clone();
            for (paint, color) in world.query::<(&theme::TextPaint, &TextColor)>().iter(world) {
                assert_eq!(color.0, palette.color(paint.0), "{screen:?}");
            }
            for (paint, color) in world
                .query::<(&theme::SurfacePaint, &BackgroundColor)>()
                .iter(world)
            {
                assert_eq!(color.0, palette.color(paint.0), "{screen:?}");
            }
            for &(entity, tick) in &original_text {
                assert_eq!(
                    world
                        .entity(entity)
                        .get_ref::<Text>()
                        .unwrap()
                        .last_changed(),
                    tick
                );
            }
            for (entity, node) in &original_nodes {
                assert_eq!(world.get::<Node>(*entity).unwrap(), node);
                assert_eq!(
                    *world.get::<UiTransform>(*entity).unwrap(),
                    UiTransform::IDENTITY
                );
            }
        }
    }
}

#[test]
fn settings_first_interface_row_changes_the_live_theme_with_keyboard_and_confirm() {
    use cdda_render::render::theme::{self, Role, ThemePreset};
    let mut app = App::new();
    app.init_resource::<settings::SettingsState>()
        .init_resource::<UiTheme>()
        .init_resource::<cdda_input::RebindCapture>()
        .insert_resource(State::new(SettingsTab::Interface))
        .init_resource::<NextState<SettingsTab>>()
        .insert_resource(default_bindings())
        .add_message::<InputAction>()
        .add_systems(
            Update,
            (
                settings::navigate,
                settings::handle_confirm,
                settings::apply_display_options,
            )
                .chain(),
        )
        .add_systems(PostUpdate, theme::apply_palette);
    let caption = app
        .world_mut()
        .spawn((
            Text::new("Retained caption"),
            theme::TextPaint(Role::Accent),
        ))
        .id();
    let panel = app
        .world_mut()
        .spawn((Node::default(), theme::SurfacePaint(Role::Surface)))
        .id();
    app.update();
    assert_eq!(app.world().resource::<UiTheme>().preset, ThemePreset::Green);
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::NavigateRight));
    app.update();
    assert_eq!(app.world().resource::<UiTheme>().preset, ThemePreset::Amber);
    app.world_mut()
        .write_message(InputAction::keyboard(GameAction::Confirm));
    app.update();
    let palette = app.world().resource::<UiTheme>();
    assert_eq!(palette.preset, ThemePreset::Blue);
    assert_eq!(
        app.world().get::<TextColor>(caption).unwrap().0,
        palette.color(Role::Accent)
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(panel).unwrap().0,
        palette.color(Role::Surface)
    );
    let tick = app
        .world()
        .entity(caption)
        .get_ref::<TextColor>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world()
            .entity(caption)
            .get_ref::<TextColor>()
            .unwrap()
            .last_changed(),
        tick
    );
}
