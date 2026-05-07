//! # cdda_app — Application binary + plugin registration.
//!
//! Wires all CDDA subsystems into a Bevy application using `GameSet`
//! ordering (Input → Sim → Render).

use bevy::app::{App, Plugin, PluginGroup, Update};
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::PresentMode;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_state::app::AppExtStates;
use bevy_state::state::{NextState, State};

use cdda_core::GameSet;
use cdda_input::{GameAction, InputAction};

use cdda_sim::def_world::load_data_system;
use cdda_sim::state::AppState;
use cdda_sim::systems::ai::ai_phase;
use cdda_sim::systems::combat::combat_phase;
use cdda_sim::systems::effects::effects_phase;
use cdda_sim::systems::movement::movement_phase;
use cdda_sim::systems::spatial::update_spatial_index;
use cdda_sim::systems::spawning::spawning_phase;
use cdda_sim::systems::turn::{debug_turn_queue, tick_move_points};
use cdda_sim::world_setup;

// ---------------------------------------------------------------------------
// Startup config
// ---------------------------------------------------------------------------

/// Passed from CLI args to configure the app.
#[derive(Resource, Clone)]
pub struct CddaStartupConfig {
    pub world_seed: u64,
    pub replay_file: Option<String>,
    pub record_session: bool,
}

impl Default for CddaStartupConfig {
    fn default() -> Self {
        Self {
            world_seed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            replay_file: None,
            record_session: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Run condition functions
// ---------------------------------------------------------------------------

fn in_main_menu(state: Res<State<AppState>>) -> bool {
    *state.get() == AppState::MainMenu
}
fn in_data_loading(state: Res<State<AppState>>) -> bool {
    *state.get() == AppState::DataLoading
}
fn in_world_gen(state: Res<State<AppState>>) -> bool {
    *state.get() == AppState::WorldGen
}
fn in_ingame(state: Res<State<AppState>>) -> bool {
    *state.get() == AppState::InGame
}

// ---------------------------------------------------------------------------
// StartGame transition system
// ---------------------------------------------------------------------------

/// Listens for `GameEvent::StartNewGame` and transitions `AppState`
/// from `MainMenu` → `Gameplay`, kicking off JSON loading + worldgen.
pub fn start_game_on_event(
    mut reader: MessageReader<cdda_ui::GameEvent>,
    mut next: ResMut<NextState<AppState>>,
) {
    for event in reader.read() {
        if *event == cdda_ui::GameEvent::StartNewGame {
            info!("Player confirmed start game — transitioning to DataLoading");
            next.set(AppState::DataLoading);
        }
    }
}

// ---------------------------------------------------------------------------
// Root plugin
// ---------------------------------------------------------------------------

pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        world_setup::setup_world(app.world_mut());

        app.init_state::<AppState>();

        app.configure_sets(
            Update,
            (GameSet::Input, GameSet::Sim, GameSet::Render).chain(),
        );

        app.add_plugins(cdda_render::CddaRenderPlugin);
        app.add_plugins(cdda_input::CddaInputPlugin);
        app.add_plugins(cdda_ui::ScreenNavigationPlugin);

        // Replay: record or replay based on startup config
        let config = app
            .world()
            .get_resource::<CddaStartupConfig>()
            .cloned()
            .unwrap_or_default();

        if let Some(ref replay_path) = config.replay_file {
            match cdda_replay::session_log::SessionLog::load_compressed(std::path::Path::new(
                replay_path,
            )) {
                Ok(log) => {
                    info!("Replay loaded: {} actions", log.len());
                    app.insert_resource(log);
                    app.add_plugins(cdda_replay::CddaReplayModePlugin);
                }
                Err(e) => {
                    error!("Failed to load replay: {e}");
                }
            }
        } else if config.record_session {
            app.add_plugins(cdda_replay::CddaReplayPlugin {
                world_seed: config.world_seed,
            });
        }

        app.add_systems(Update, load_data_system.run_if(in_data_loading));
        app.add_systems(Update, start_game_on_event.run_if(in_main_menu));
        app.add_systems(
            Update,
            cdda_sim::def_world::worldgen_system.run_if(in_world_gen),
        );

        app.add_systems(
            Update,
            (
                tick_move_points.run_if(in_ingame).in_set(GameSet::Sim),
                ai_phase
                    .run_if(in_ingame)
                    .after(tick_move_points)
                    .in_set(GameSet::Sim),
                movement_phase
                    .run_if(in_ingame)
                    .after(ai_phase)
                    .in_set(GameSet::Sim),
                combat_phase
                    .run_if(in_ingame)
                    .after(movement_phase)
                    .in_set(GameSet::Sim),
                effects_phase
                    .run_if(in_ingame)
                    .after(combat_phase)
                    .in_set(GameSet::Sim),
                spawning_phase
                    .run_if(in_ingame)
                    .after(effects_phase)
                    .in_set(GameSet::Sim),
                update_spatial_index
                    .run_if(in_ingame)
                    .after(spawning_phase)
                    .in_set(GameSet::Sim),
                debug_turn_queue
                    .run_if(in_ingame)
                    .after(update_spatial_index)
                    .in_set(GameSet::Sim),
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() {
    let config = CddaStartupConfig::default();
    info!("World seed: {}", config.world_seed);

    let mut app = App::new();
    app.insert_resource(config);
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings { ..default() }),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Cataclysm: Dark Days Ahead".into(),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
    );
    // Debug: inspector panel (toggle with F3 in windowed mode)
    app.add_plugins((bevy_egui::EguiPlugin::default(), WorldInspectorPlugin::new()));

    app.add_plugins(CddaPlugin);
    app.run();
}
