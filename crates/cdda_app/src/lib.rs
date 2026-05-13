//! # cdda_app — Application binary + plugin registration.
//!
//! Wires all CDDA subsystems into a Bevy application using `GameSet`
//! ordering (Input → Sim → Render).

use bevy::app::{App, Plugin, PluginGroup, Update};
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::time::common_conditions::on_timer;
use bevy::window::PresentMode;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_state::app::AppExtStates;
use bevy_state::prelude::OnEnter;
use bevy_state::state::{NextState, State};
use std::time::Duration;

use cdda_core::context::ctx::Ctx as Screen;
use cdda_core::context::screen::Screen as ScreenPlugin;
use cdda_core::context::ContextStack;
use cdda_core::{GameSet, SimSet};

use cdda_activity::plugin::ActivityPlugin;
use cdda_components::dev::DevPlayer;
use cdda_components::events::ItemMoveEvent;
use cdda_components::sim::WorldPosition;
use cdda_core::actor::bionics::tick_bionics;
use cdda_core::actor::effects::effects_phase;
use cdda_core::actor::healing::healing_phase;
use cdda_core::actor::morale::tick_morale_decay;
use cdda_core::actor::movement::movement_phase;
use cdda_core::actor::plugin::ActorPlugin;
use cdda_core::actor::temperature::temperature_phase;
use cdda_core::actor::turn::{debug_turn_queue, tick_move_points, TurnQueue};
use cdda_core::actor::vision::update_vision;
use cdda_core::ai::systems::ai_phase;
use cdda_core::combat::systems::combat_phase;
use cdda_core::context::overlay::{
    cleanup_activity_overlay, handle_overlay_cancel, sync_activity_overlay,
};
use cdda_core::crafting::plugin::CraftingPlugin;
use cdda_core::crafting::systems::on_examine_item_changed;
use cdda_core::data::assets::CddaAssetsPlugin;
use cdda_core::inventory::systems::{
    assign_invlets_system, build_inventory_bins, dev_pickup_drop_system, inventory_screen_input,
    process_item_move_events,
};
use cdda_core::item::plugin::ItemPlugin;
use cdda_core::overmap::spatial::EntitySpatialIndex;
use cdda_core::overmap_gen::pipeline::OvermapGenPlugin;
use cdda_core::overmap_gen::setup::register_game_components;
use cdda_core::overmap_gen::spatial_systems::{cleanup_spatial_index, update_spatial_index};
use cdda_core::sim::state::{AppState, StartupConfig};
use cdda_core::startup::load_data_system;
use cdda_core::startup::{examine_item_input, spawn_dev_world};
use cdda_overmap::OvermapCamera;

// ---------------------------------------------------------------------------
// Startup config
// ---------------------------------------------------------------------------

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
// Dev player movement
// ---------------------------------------------------------------------------

/// Move the dev player with arrow keys / hjkl.
pub fn dev_player_move(
    keys: Res<bevy::prelude::ButtonInput<bevy::prelude::KeyCode>>,
    mut query: Query<&mut WorldPosition, With<DevPlayer>>,
    mut camera: ResMut<cdda_overmap::OvermapCamera>,
) {
    let dx = if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL) {
        1
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH) {
        -1
    } else {
        0
    };
    let dy = if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK) {
        1
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ) {
        -1
    } else {
        0
    };

    if dx != 0 || dy != 0 {
        for mut pos in &mut query {
            pos.0.x += dx * 24; // 24 tiles per OMT
            pos.0.y += dy * 24;
            camera.move_to(pos.0.x.div_euclid(24), pos.0.y.div_euclid(24));
        }
    }
}

// ---------------------------------------------------------------------------
// Root plugin
// ---------------------------------------------------------------------------

pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        // Register ECS components first.
        register_game_components(app.world_mut());

        app.add_plugins((
            ActivityPlugin,
            ActorPlugin,
            ItemPlugin,
            CddaAssetsPlugin,
            CraftingPlugin,
            cdda_core::data::flags::CddaDataPlugin,
        ));

        // Entity spatial index for dynamic entity queries.
        app.init_resource::<EntitySpatialIndex>();
        app.init_resource::<OvermapCamera>();

        // Overmap generation pipeline.
        app.add_plugins(OvermapGenPlugin);

        app.init_state::<AppState>();
        app.init_resource::<StartupConfig>();
        app.init_resource::<TurnQueue>();
        app.init_resource::<cdda_components::item::InventoryBin>();
        app.init_resource::<cdda_core::inventory::examine_resource::ExaminedItem>();
        app.init_resource::<cdda_components::dev::DevCamera>();
        app.init_resource::<cdda_core::sim::state::LoadingStatus>();
        app.init_resource::<cdda_core::sim::state::GameTime>();

        // ── Screen transitions ─────────────────────────────────────────
        app.add_systems(
            OnEnter(AppState::MainMenu),
            |mut next: ResMut<NextState<Screen>>| next.set(Screen::MainMenu),
        );
        app.add_systems(
            OnEnter(AppState::DataLoading),
            |mut next: ResMut<NextState<Screen>>| next.set(Screen::DevWorldgen),
        );
        app.add_systems(
            OnEnter(AppState::WorldGen),
            |mut next: ResMut<NextState<Screen>>| next.set(Screen::DevWorldgen),
        );
        app.add_systems(
            OnEnter(AppState::InGame),
            |mut next: ResMut<NextState<Screen>>| next.set(Screen::Gameplay),
        );

        // ── System set ordering ────────────────────────────────────────
        app.configure_sets(
            Update,
            (GameSet::Input, GameSet::Sim, GameSet::Render).chain(),
        );
        app.configure_sets(
            Update,
            (
                SimSet::TurnTick,
                SimSet::Activity,
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
        app.add_message::<cdda_components::messages::TurnAdvanced>();

        // ── Context action registration ────────────────────────────────
        app.add_plugins(ScreenPlugin::<
            cdda_render::render::inventory::InventoryScreen,
        >::default());
        app.add_plugins(ScreenPlugin::<cdda_render::render::crafting::CraftingScreen>::default());
        app.add_plugins(ScreenPlugin::<cdda_render::render::examine::ExamineScreen>::default());
        app.add_plugins(ScreenPlugin::<
            cdda_render::render::character::CharacterScreen,
        >::default());

        app.add_plugins(cdda_render::render::CddaRenderPlugin);
        app.add_plugins(cdda_core::input::CddaInputPlugin);
        app.add_plugins(cdda_core::context::ContextPlugin);

        // ── Replay ─────────────────────────────────────────────────────
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
                Err(e) => error!("Failed to load replay: {e}"),
            }
        } else if config.record_session {
            app.add_plugins(cdda_core::replay::CddaReplayPlugin {
                world_seed: config.world_seed,
            });
        }

        // ── Startup systems ────────────────────────────────────────────
        app.add_systems(OnEnter(AppState::InGame), spawn_dev_world);
        app.add_systems(
            Update,
            load_data_system.run_if(in_state(AppState::DataLoading)),
        );
        app.add_systems(
            Update,
            cdda_core::startup::worldgen_system.run_if(in_state(AppState::WorldGen)),
        );

        // ── Turn tick ──────────────────────────────────────────────────
        app.add_systems(
            Update,
            tick_move_points
                .in_set(SimSet::TurnTick)
                .run_if(in_state(AppState::InGame))
                .run_if(on_timer(Duration::from_millis(100))),
        );

        // ── Simulation systems ─────────────────────────────────────────
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
                dev_pickup_drop_system
                    .in_set(SimSet::Inventory)
                    .run_if(in_state(Screen::Gameplay)),
                process_item_move_events.in_set(SimSet::Inventory),
                assign_invlets_system.in_set(SimSet::Inventory),
                build_inventory_bins.in_set(SimSet::Inventory),
                inventory_screen_input
                    .in_set(SimSet::Inventory)
                    .run_if(in_state(Screen::Inventory)),
                // Spatial index updates track entity positions.
                update_spatial_index.in_set(SimSet::SpatialUpdate),
                cleanup_spatial_index.in_set(SimSet::SpatialUpdate),
                debug_turn_queue.in_set(SimSet::SpatialUpdate),
            )
                .run_if(in_state(AppState::InGame)),
        );

        // ── Dev player movement ────────────────────────────────────
        app.add_systems(Update, dev_player_move.run_if(in_state(AppState::InGame)));

        // ── Overmap viewer toggle ─────────────────────────────────────
        app.add_systems(Update, toggle_overmap.run_if(in_state(AppState::InGame)));

        // ── UI overlay systems ─────────────────────────────────────────
        app.add_systems(
            Update,
            handle_overlay_cancel.run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            Update,
            (sync_activity_overlay, cleanup_activity_overlay)
                .chain()
                .in_set(SimSet::Activity)
                .run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            Update,
            examine_item_input
                .in_set(SimSet::Inventory)
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(Screen::ItemExamine)),
        );
        app.add_systems(
            Update,
            on_examine_item_changed
                .in_set(SimSet::Inventory)
                .run_if(in_state(AppState::InGame))
                .run_if(in_state(Screen::ItemExamine)),
        );
    }
}

// ---------------------------------------------------------------------------
// Hotkey handlers
// ---------------------------------------------------------------------------

/// Toggle the overmap viewer with the M key.
pub fn toggle_overmap(
    keys: Res<bevy::prelude::ButtonInput<bevy::prelude::KeyCode>>,
    state: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut stack: ResMut<ContextStack>,
) {
    if keys.just_pressed(bevy::prelude::KeyCode::KeyM) {
        match state.get() {
            Screen::Gameplay => {
                stack.0.push(Screen::Gameplay);
                next.set(Screen::Overmap);
            }
            Screen::Overmap => {
                if let Some(prev) = stack.0.pop() {
                    next.set(prev);
                } else {
                    next.set(Screen::Gameplay);
                }
            }
            _ => {}
        }
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
    app.add_plugins((
        bevy_egui::EguiPlugin::default(),
        WorldInspectorPlugin::new(),
    ));

    app.add_plugins(CddaPlugin);
    app.run();
}
