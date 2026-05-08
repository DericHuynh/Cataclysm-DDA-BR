//! # cdda_app — Application binary + plugin registration.
//!
//! Wires all CDDA subsystems into a Bevy application using `GameSet`
//! ordering (Input → Sim → Render).

use bevy::app::{App, Plugin, PluginGroup, Update};
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::time::common_conditions::on_timer;
use bevy::window::PresentMode;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_state::app::AppExtStates;
use bevy_state::prelude::OnEnter;
use bevy_state::state::NextState;
use std::time::Duration;

use cdda_core::screen::Screen;
use cdda_core::{GameSet, SimSet};

use cdda_core::actor::plugin::ActorPlugin;
use cdda_core::data::assets::CddaAssetsPlugin;
use cdda_core::item::plugin::ItemPlugin;
use cdda_core::sim::def_world::load_data_system;
use cdda_core::sim::events::ItemMoveEvent;
use cdda_core::sim::state::AppState;
use cdda_core::sim::systems::ai::ai_phase;
use cdda_core::sim::systems::bionics::tick_bionics;
use cdda_core::sim::systems::combat::combat_phase;
use cdda_core::sim::systems::dev_spawn::{
    build_dev_spawn_catalog, dev_spawn_flush, dev_spawn_panel_input,
};
use cdda_core::sim::systems::effects::effects_phase;
use cdda_core::sim::systems::healing::healing_phase;
use cdda_core::sim::systems::inventory::{
    assign_invlets_system, build_inventory_bins, dev_pickup_drop_system, inventory_screen_input,
    process_item_move_events, spawn_dev_world,
};
use cdda_core::sim::systems::morale::tick_morale_decay;
use cdda_core::sim::systems::movement::movement_phase;
use cdda_core::sim::systems::spatial::update_spatial_index;
use cdda_core::sim::systems::spawning::spawning_phase;
use cdda_core::sim::systems::temperature::temperature_phase;
use cdda_core::sim::systems::turn::{debug_turn_queue, tick_move_points};
use cdda_core::sim::systems::vision::update_vision;
use cdda_core::sim::world_setup;

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
// StartGame transition system
// ---------------------------------------------------------------------------

/// Listens for `GameEvent::StartNewGame` and transitions `AppState`
/// from `MainMenu` → `Gameplay`, kicking off JSON loading + worldgen.
pub fn start_game_on_event(
    mut reader: MessageReader<cdda_core::screen::GameEvent>,
    mut next: ResMut<NextState<AppState>>,
) {
    for event in reader.read() {
        if *event == cdda_core::screen::GameEvent::StartNewGame {
            info!("Player confirmed start game — transitioning to DataLoading");
            next.set(AppState::DataLoading);
        }
    }
}

// ---------------------------------------------------------------------------
// Reflect type registration
// ---------------------------------------------------------------------------

fn register_reflect_types(app: &mut App) {
    use cdda_core::sim::components::{InFlight, Solid, Velocity, WorldPosition};

    app.register_type::<WorldPosition>();
    app.register_type::<Solid>();
    app.register_type::<Velocity>();
    app.register_type::<InFlight>();
}

// ---------------------------------------------------------------------------
// Root plugin
// ---------------------------------------------------------------------------

pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ActorPlugin, ItemPlugin, CddaAssetsPlugin, cdda_core::sim::flags::CddaDataPlugin));
        world_setup::setup_world(app.world_mut());
        register_reflect_types(app);

        app.init_state::<AppState>();

        // Fix #5: Drive Screen from AppState so sim and render never desync.
        app.add_systems(
            OnEnter(AppState::MainMenu),
            |mut next: ResMut<NextState<Screen>>| {
                next.set(Screen::MainMenu);
            },
        );
        app.add_systems(
            OnEnter(AppState::DataLoading),
            |mut next: ResMut<NextState<Screen>>| {
                next.set(Screen::DevWorldgen); // loading screen reuses devworldgen view
            },
        );
        app.add_systems(
            OnEnter(AppState::WorldGen),
            |mut next: ResMut<NextState<Screen>>| {
                next.set(Screen::DevWorldgen);
            },
        );
        app.add_systems(
            OnEnter(AppState::InGame),
            |mut next: ResMut<NextState<Screen>>| {
                next.set(Screen::Gameplay);
            },
        );

        app.configure_sets(
            Update,
            (GameSet::Input, GameSet::Sim, GameSet::Render).chain(),
        );
        app.configure_sets(
            Update,
            (
                SimSet::TurnTick,
                SimSet::Ai,
                SimSet::Movement,
                SimSet::Combat,
                SimSet::Effects,
                SimSet::Healing,
                SimSet::Bionics,
                SimSet::Morale,
                SimSet::Temperature,
                SimSet::Vision,
                SimSet::Spawning,
                SimSet::Inventory,
                SimSet::SpatialUpdate,
            )
                .chain()
                .in_set(GameSet::Sim),
        );

        app.add_message::<ItemMoveEvent>();

        app.add_plugins(cdda_core::render::CddaRenderPlugin);
        app.add_plugins(cdda_core::input::CddaInputPlugin);
        app.add_plugins(cdda_core::screen::ScreenNavigationPlugin);

        // Replay: record or replay based on startup config
        let config = app
            .world()
            .get_resource::<CddaStartupConfig>()
            .cloned()
            .unwrap_or_default();

        if let Some(ref replay_path) = config.replay_file {
            match cdda_core::replay::session_log::SessionLog::load_compressed(std::path::Path::new(
                replay_path,
            )) {
                Ok(log) => {
                    info!("Replay loaded: {} actions", log.len());
                    app.insert_resource(log);
                    app.add_plugins(cdda_core::replay::CddaReplayModePlugin);
                }
                Err(e) => {
                    error!("Failed to load replay: {e}");
                }
            }
        } else if config.record_session {
            app.add_plugins(cdda_core::replay::CddaReplayPlugin {
                world_seed: config.world_seed,
            });
        }

        app.add_systems(OnEnter(AppState::InGame), spawn_dev_world);
        // Build spawn catalog the first time the debug panel is opened.
        app.add_systems(OnEnter(Screen::DevSpawnPanel), build_dev_spawn_catalog);
        app.add_systems(
            Update,
            load_data_system.run_if(in_state(AppState::DataLoading)),
        );
        app.add_systems(
            Update,
            start_game_on_event.run_if(in_state(AppState::MainMenu)),
        );
        app.add_systems(
            Update,
            cdda_core::sim::def_world::worldgen_system.run_if(in_state(AppState::WorldGen)),
        );

        // Fix #6: Gate turn tick so MP isn't granted every frame.
        // tick_move_points runs at most once per 100 ms (10 turns/sec real-time max).
        app.add_systems(
            Update,
            tick_move_points
                .in_set(SimSet::TurnTick)
                .run_if(in_state(AppState::InGame))
                .run_if(on_timer(Duration::from_millis(100))),
        );

        app.add_systems(
            Update,
            (
                ai_phase.in_set(SimSet::Ai),
                movement_phase.in_set(SimSet::Movement),
                combat_phase.in_set(SimSet::Combat),
                effects_phase.in_set(SimSet::Effects),
                healing_phase.in_set(SimSet::Healing),
                tick_bionics.in_set(SimSet::Bionics),
                tick_morale_decay.in_set(SimSet::Morale),
                temperature_phase.in_set(SimSet::Temperature),
                update_vision.in_set(SimSet::Vision),
                spawning_phase.in_set(SimSet::Spawning),
                // Inventory pipeline: pickup/drop → process moves → assign letters → rebuild bins
                dev_pickup_drop_system
                    .in_set(SimSet::Inventory)
                    .run_if(in_state(Screen::Gameplay)),
                process_item_move_events.in_set(SimSet::Inventory),
                assign_invlets_system.in_set(SimSet::Inventory),
                build_inventory_bins.in_set(SimSet::Inventory),
                // Inventory screen navigation + drop-from-inventory
                inventory_screen_input
                    .in_set(SimSet::Inventory)
                    .run_if(in_state(Screen::Inventory)),
                // Debug spawn panel — navigation queues a def-entity
                dev_spawn_panel_input
                    .in_set(SimSet::Inventory)
                    .run_if(in_state(Screen::DevSpawnPanel)),
                update_spatial_index.in_set(SimSet::SpatialUpdate),
                debug_turn_queue.in_set(SimSet::SpatialUpdate),
            )
                .run_if(in_state(AppState::InGame)),
        );

        // Exclusive system: drain spawn queue and call EntityCloner-based spawn_item.
        // Must be separate from the tuple above because exclusive systems can't be
        // grouped with regular systems.
        app.add_systems(
            Update,
            dev_spawn_flush
                .in_set(SimSet::Inventory)
                .after(dev_spawn_panel_input)
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(Screen::DevSpawnPanel)),
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
    app.add_plugins((
        bevy_egui::EguiPlugin::default(),
        WorldInspectorPlugin::new(),
    ));

    app.add_plugins(CddaPlugin);
    app.run();
}
