//! Offscreen visual smoke fixture using the production screen and motion systems.
//! cargo run -p cdda_app --example menu_capture -- menu /tmp/menu.png 1600 900 100
//! Modes: menu, menu-last, settings, settings-tabs, loading, error. Final optional
//! argument: theme index 0/1/2. settings-tabs captures 24 consecutive transition
//! frames to PATH-FRAME.png. Requires a working GPU adapter, no display server.
use bevy::{
    app::ScheduleRunnerPlugin,
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::{TextureFormat, TextureUsages},
        view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
    },
    window::{ExitCondition, PrimaryWindow},
    winit::WinitPlugin,
};
use bevy_state::prelude::State;
use cdda_components::progress::{OperationCommand, OperationReport, ReportEvent, ReportLevel};
use cdda_context::{nav::FocusedCommandIndex, Ctx, InputFocus};
use cdda_input::InputAction;
use cdda_render::render::{cinematic, loading, main_menu, settings::SettingsState, UiFontHandle};

#[derive(Resource)]
struct Capture {
    target: Handle<Image>,
    path: String,
    frames: u32,
    sequence: bool,
    remaining: u32,
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).map_or("menu", String::as_str);
    let path = args.get(2).cloned().unwrap_or("/tmp/cdda-menu.png".into());
    let number = |index: usize, fallback: u32| {
        args.get(index)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(fallback)
    };
    let (width, height, scale) = (number(3, 1600), number(4, 900), number(5, 100));
    let selected_theme = cdda_render::render::theme::ThemePreset::ALL[number(6, 1) as usize % 3];
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<WinitPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            }),
    )
    .add_plugins(ScheduleRunnerPlugin::run_loop(
        std::time::Duration::from_millis(16),
    ))
    .insert_resource(State::new(Ctx::MainMenu))
    .insert_resource(UiScale(scale as f32 / 100.))
    .init_resource::<SettingsState>()
    .init_resource::<cdda_render::render::theme::UiTheme>()
    .init_resource::<FocusedCommandIndex>()
    .init_resource::<InputFocus>()
    .insert_resource(cdda_input::ActiveKeybindings {
        keys: [
            (cdda_input::BindableAction::NavigateUp, "Up/Down".into()),
            (cdda_input::BindableAction::Confirm, "Enter".into()),
        ]
        .into(),
    })
    .init_resource::<OperationReport>()
    .add_message::<OperationCommand>()
    .add_message::<InputAction>();
    app.insert_resource(cdda_render::render::theme::UiTheme {
        preset: selected_theme,
    });
    app.add_plugins(cdda_render::render::UiPresentationPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load("fonts/ShareTechMono-Regular.ttf");
    app.insert_resource(UiFontHandle(Some(font)));
    // Logical viewport for the production responsive layout; no OS window is created.
    app.world_mut().spawn((
        Window {
            resolution: (width, height).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    let mut image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.world_mut().spawn((
        Camera2d,
        IsDefaultUiCamera,
        RenderTarget::Image(target.clone().into()),
    ));
    app.insert_resource(Capture {
        target,
        path,
        frames: 0,
        sequence: mode == "settings-tabs",
        remaining: if mode == "settings-tabs" { 24 } else { 1 },
    });
    if mode.starts_with("menu") {
        if mode == "menu-last" {
            app.world_mut().resource_mut::<FocusedCommandIndex>().set(8);
        }
        app.add_systems(
            PostUpdate,
            (
                cdda_render::render::scroll::scroll_to_focused_row,
                cdda_render::render::scroll::update_virtual_windows,
            )
                .chain()
                .before(bevy::ui::UiSystems::Layout),
        );
        app.add_systems(Startup, main_menu::spawn)
            .add_systems(Update, main_menu::sync_focus);
    } else if mode.starts_with("settings") {
        app.insert_resource(State::new(cdda_context::substate::SettingsTab::Interface))
            .insert_resource(cdda_input::bindings::default_bindings())
            .add_systems(Startup, cdda_render::render::settings::spawn)
            .add_systems(
                Update,
                (
                    cdda_render::render::settings::rebuild_content_panel,
                    cdda_render::render::settings::sync_tab_highlight,
                )
                    .chain(),
            );
        app.world_mut()
            .resource_mut::<SettingsState>()
            .interface_theme = number(6, 1) as usize % 3;
    } else {
        let report = if mode == "error" {
            ReportEvent::progress(
                "Parsing JSON",
                "data/items/example.json: invalid JSON at line 42",
            )
            .level(ReportLevel::Error)
        } else {
            ReportEvent::progress("Resolving and converting definitions", "ITEM").units(728, 2000)
        };
        app.world_mut()
            .resource_mut::<OperationReport>()
            .record(report);
        app.add_systems(Startup, loading::spawn)
            .add_systems(Update, (loading::update, loading::animate_progress).chain());
    }
    app.add_systems(First, advance_capture)
        .add_systems(
            Update,
            (
                cinematic::layout,
                cinematic::reveal_art,
                cinematic::animate_accents.after(main_menu::sync_focus),
            ),
        )
        .add_systems(Last, capture)
        .run();
}
fn advance_capture(mut commands: Commands, mut capture: ResMut<Capture>) {
    capture.frames += 1;
    if capture.sequence {
        use cdda_context::substate::SettingsTab;
        let tab = match capture.frames {
            120 => Some(SettingsTab::Graphics),
            126 => Some(SettingsTab::Keybindings),
            132 => Some(SettingsTab::General),
            138 => Some(SettingsTab::Interface),
            _ => None,
        };
        if let Some(tab) = tab {
            commands.insert_resource(State::new(tab));
        }
    }
    assert!(capture.frames < 600, "offscreen capture timed out");
}
fn capture(mut commands: Commands, capture: Res<Capture>) {
    if capture.frames == 120 || (capture.sequence && (121..144).contains(&capture.frames)) {
        let path = if capture.sequence {
            format!(
                "{}-{}.png",
                capture.path.trim_end_matches(".png"),
                capture.frames
            )
        } else {
            capture.path.clone()
        };
        commands
            .spawn(Screenshot::image(capture.target.clone()))
            .observe(save_to_disk(path))
            .observe(
                |_: On<ScreenshotCaptured>,
                 mut capture: ResMut<Capture>,
                 mut exit: MessageWriter<AppExit>| {
                    capture.remaining -= 1;
                    if capture.remaining == 0 {
                        exit.write(AppExit::Success);
                    }
                },
            );
    }
}
