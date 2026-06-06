//! # cdda_app — Application binary + plugin registration.
//!
//! Wires all CDDA subsystems into a Bevy application using `GameSet`
//! ordering (Input → Sim → Render).

mod startup;

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

use cdda_components::schedule::{GameSet, SimSet};
use cdda_context::ctx::Ctx as Screen;
use cdda_context::screen::Screen as ScreenPlugin;
use cdda_context::ContextStack;

use crate::startup::load_data_system;
use crate::startup::{examine_item_input, spawn_dev_world};
use cdda_activity::plugin::ActivityPlugin;
use cdda_actor::bionics::tick_bionics;
use cdda_actor::effects::effects_phase;
use cdda_actor::healing::healing_phase;
use cdda_actor::morale::tick_morale_decay;
use cdda_actor::movement::movement_phase;
use cdda_actor::plugin::ActorPlugin;
use cdda_actor::temperature::temperature_phase;
use cdda_actor::turn::{debug_turn_queue, tick_move_points, TurnQueue};
use cdda_actor::vision::update_vision;
use cdda_ai::systems::ai_phase;
use cdda_combat::systems::combat_phase;
use cdda_components::dev::{DevCamera, DevPlayer};
use cdda_components::events::ItemMoveEvent;
use cdda_components::item::InventoryFocus;
use cdda_components::messages;
use cdda_components::sim::WorldPosition;
use cdda_context::overlay::{
    cleanup_activity_overlay, handle_overlay_cancel, sync_activity_overlay,
};
use cdda_core_types::core::coords::TILES_PER_OMT;
use cdda_crafting::plugin::CraftingPlugin;
use cdda_crafting::systems::on_examine_item_changed;
use cdda_data::assets::CddaAssetsPlugin;
use cdda_inventory::systems::{
    assign_invlets_system, build_inventory_bins, dev_pickup_drop_system, inventory_screen_input,
    process_item_move_events, InventoryBin,
};
use cdda_item::plugin::ItemPlugin;
use cdda_overmap::spatial::EntitySpatialIndex;
use cdda_overmap::OvermapCamera;
use cdda_overmap_gen::pipeline::OvermapGenPlugin;
use cdda_sim::state::{AppState, StartupConfig};

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

/// Move the dev player using raw keyboard input, gated on Screen::Gameplay.
///
/// Uses `ButtonInput<KeyCode>` (not `MessageReader`) to avoid ordering
/// dependencies on `bridge_actionstate`. The y-axis follows CDDA convention:
/// ArrowUp / K = north = -y, ArrowDown / J = south = +y.
///
/// Updates both `DevCamera` (ASCII viewport follows) and `OvermapCamera`
/// (overmap viewer follows).
pub fn dev_player_move(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut WorldPosition, With<DevPlayer>>,
    mut dev_cam: ResMut<DevCamera>,
    mut overmap_cam: ResMut<OvermapCamera>,
) {
    let dx = if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL) {
        1 // east = +x
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH) {
        -1 // west = -x
    } else {
        0
    };
    let dy = if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK) {
        -1 // CDDA: north = -y
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ) {
        1 // CDDA: south = +y
    } else {
        0
    };

    if dx != 0 || dy != 0 {
        for mut pos in &mut query {
            pos.0.x += dx * TILES_PER_OMT;
            pos.0.y += dy * TILES_PER_OMT;
            let omt_x = pos.0.x.div_euclid(TILES_PER_OMT);
            let omt_y = pos.0.y.div_euclid(TILES_PER_OMT);
            dev_cam.x = omt_x;
            dev_cam.y = omt_y;
            dev_cam.z = 0;
            overmap_cam.move_to(omt_x, omt_y);
        }
    }
}

// ---------------------------------------------------------------------------
// Root plugin
// ---------------------------------------------------------------------------

pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ActivityPlugin,
            ActorPlugin,
            ItemPlugin,
            CddaAssetsPlugin,
            CraftingPlugin,
            cdda_data::flags::CddaDataPlugin,
        ));

        app.init_resource::<EntitySpatialIndex>();
        app.init_resource::<OvermapCamera>();

        app.add_plugins(OvermapGenPlugin);

        app.init_state::<AppState>();
        app.init_resource::<StartupConfig>();
        app.init_resource::<TurnQueue>();
        app.init_resource::<InventoryBin>();
        app.init_resource::<cdda_inventory::examine_resource::ExaminedItem>();
        app.init_resource::<InventoryFocus>();
        app.init_resource::<DevCamera>();
        app.init_resource::<cdda_sim::state::LoadingStatus>();
        app.init_resource::<cdda_sim::state::GameTime>();

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
        app.add_message::<messages::TurnAdvanced>();

        // ── Screen plugins ─────────────────────────────────────────────
        app.add_plugins(ScreenPlugin::<
            cdda_render::render::inventory::InventoryScreen,
        >::default());
        app.add_plugins(ScreenPlugin::<cdda_render::render::crafting::CraftingScreen>::default());
        app.add_plugins(ScreenPlugin::<cdda_render::render::examine::ExamineScreen>::default());
        app.add_plugins(ScreenPlugin::<
            cdda_render::render::character::CharacterScreen,
        >::default());
        app.add_plugins(ScreenPlugin::<cdda_render::render::registry::RegistryScreen>::default());

        app.add_plugins(cdda_render::render::CddaRenderPlugin);
        app.add_plugins(cdda_input::CddaInputPlugin);
        app.add_plugins(cdda_context::ContextPlugin);

        // ── Replay ─────────────────────────────────────────────────────
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
                Err(e) => error!("Failed to load replay: {e}"),
            }
        } else if config.record_session {
            app.add_plugins(cdda_replay::CddaReplayPlugin {
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
            crate::startup::worldgen_system.run_if(in_state(AppState::WorldGen)),
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
                debug_turn_queue.in_set(SimSet::SpatialUpdate),
            )
                .run_if(in_state(AppState::InGame)),
        );

        // ── Dev player movement — gated on Screen::Gameplay ────────
        // Must run before render systems so the viewport updates immediately.
        app.add_systems(
            Update,
            dev_player_move
                .in_set(GameSet::Input)
                .run_if(in_state(Screen::Gameplay)),
        );

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
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut stack: ResMut<ContextStack>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
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
