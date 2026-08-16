//! # cdda_app — Application binary + plugin registration.
//!
//! Wires all CDDA subsystems into a Bevy application using `GameSet`
//! ordering (Input → Sim → Render).

mod data_assets;
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

use cdda_components::intent::ActionIntent;
use cdda_components::schedule::{GameSet, SimSet};
use cdda_context::ctx::Ctx as Screen;
use cdda_context::screen::Screen as ScreenPlugin;
use cdda_context::ContextStack;

use crate::data_assets::{reload_modified_data, request_data_files, CddaDataFiles};
use crate::startup::load_data_system;
use crate::startup::{examine_item_input, spawn_dev_world};
use cdda_components::actor::IsAlive;
use cdda_components::dev::{DevCamera, DevPlayer};
use cdda_components::events::ItemMoveEvent;
use cdda_components::intent::IntentQueue;
use cdda_components::item::InventoryFocus;
use cdda_components::messages;
use cdda_components::sim::WorldPosition;
use cdda_context::overlay::{
    cleanup_activity_overlay, handle_overlay_cancel, sync_activity_overlay,
};
use cdda_core_types::core::coords::{WorldPos, TILES_PER_OMT};
use cdda_data::assets::CddaAssetsPlugin;
use cdda_overmap::spatial::EntitySpatialIndex;
use cdda_overmap::OvermapCamera;
use cdda_overmap_gen::pipeline::OvermapGenPlugin;
use cdda_sim::activity::plugin::ActivityPlugin;
use cdda_sim::actor::bionics::tick_bionics;
use cdda_sim::actor::effects::effects_phase;
use cdda_sim::actor::healing::healing_phase;
use cdda_sim::actor::morale::tick_morale_decay;
use cdda_sim::actor::plugin::ActorPlugin;
use cdda_sim::actor::temperature::temperature_phase;
use cdda_sim::actor::turn::{debug_turn_queue, tick_move_points, TurnQueue};
use cdda_sim::actor::vision::update_vision;
use cdda_sim::ai::plugin::AiPlugin;
use cdda_sim::crafting::plugin::CraftingPlugin;
use cdda_sim::crafting::systems::on_examine_item_changed;
use cdda_sim::intent::plugin::IntentPlugin;
use cdda_sim::inventory::systems::{
    assign_invlets_system, build_inventory_bins, process_item_move_events, InventoryBin,
};
use cdda_sim::item::plugin::ItemPlugin;
use cdda_sim::runtime::state::{AppState, StartupConfig};

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

/// Read player keyboard input and generate `ActionIntent` on the dev player.
///
/// The intent resolution system in `SimSet::IntentResolve` handles the actual
/// movement, AP deduction, and precondition checking.
/// Read player keyboard input and generate `ActionIntent` on the dev player.
///
/// The intent resolution system in `SimSet::IntentResolve` handles the actual
/// movement, AP deduction, and precondition checking.
///
/// The dev-world viewport renders **OMT** cells (one overmap tile per cell)
/// and the [`DevCamera`] is in OMT units, while the player's `WorldPosition`
/// is in **world tiles**. Advance the player by a full `TILES_PER_OMT` step so
/// a single keypress visibly moves the viewport by exactly one cell — the
/// `DevCamera` is derived from `WorldPosition / TILES_PER_OMT`, so the units
/// line back up.
pub fn dev_player_move(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    player_query: Query<Entity, (With<DevPlayer>, With<IsAlive>)>,
) {
    let dx = if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL) {
        1
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH) {
        -1
    } else {
        0
    };
    let dy = if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK) {
        -1
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ) {
        1
    } else {
        0
    };

    if dx != 0 || dy != 0 {
        if let Some(player) = player_query.iter().next() {
            // One OMT per keypress (see doc comment above): 24 world tiles.
            commands.entity(player).insert(ActionIntent::Move {
                dx: dx * TILES_PER_OMT,
                dy: dy * TILES_PER_OMT,
            });
        }
    }
}

/// After intent resolution, sync camera positions to the dev player's
/// new location.  Runs in `SimSet::IntentResolve` to update the viewport
/// in the same frame.
pub fn dev_player_intent_generate(
    mut dev_cam: ResMut<DevCamera>,
    mut overmap_cam: ResMut<OvermapCamera>,
    q: Query<&WorldPosition, (With<DevPlayer>, Changed<WorldPosition>)>,
) {
    for pos in &q {
        let current = pos.get();
        let omt_x = current.x.div_euclid(TILES_PER_OMT);
        let omt_y = current.y.div_euclid(TILES_PER_OMT);
        dev_cam.x = omt_x;
        dev_cam.y = omt_y;
        dev_cam.z = 0;
        overmap_cam.move_to(omt_x, omt_y);
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
            AiPlugin,
            ActorPlugin,
            ItemPlugin,
            CddaAssetsPlugin,
            CraftingPlugin,
            IntentPlugin,
            cdda_data::flags::CddaDataPlugin,
        ));

        app.init_resource::<EntitySpatialIndex>();
        app.init_resource::<OvermapCamera>();

        app.add_plugins(OvermapGenPlugin);

        app.init_state::<AppState>();
        app.init_resource::<StartupConfig>();
        app.init_resource::<TurnQueue>();
        app.init_resource::<InventoryBin>();
        app.init_resource::<cdda_sim::inventory::examine_resource::ExaminedItem>();
        app.init_resource::<InventoryFocus>();
        app.init_resource::<DevCamera>();
        app.init_resource::<cdda_sim::runtime::state::LoadingStatus>();
        app.init_resource::<cdda_sim::runtime::state::GameTime>();
        app.init_resource::<IntentQueue>();
        app.init_resource::<CddaDataFiles>();

        // ── Screen transitions ─────────────────────────────────────────
        app.add_systems(
            OnEnter(AppState::MainMenu),
            |mut next: ResMut<NextState<Screen>>| next.set(Screen::MainMenu),
        );
        app.add_systems(
            OnEnter(AppState::DataLoading),
            (
                |mut next: ResMut<NextState<Screen>>| next.set(Screen::DevWorldgen),
                request_data_files,
            ),
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
                SimSet::IntentDeclare,
                SimSet::IntentResolve,
                SimSet::Activity,
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
        // Asset-driven hot reload of CDDA data files currently in use.
        app.add_systems(
            Update,
            reload_modified_data.run_if(in_state(AppState::InGame)),
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
                // Intent generation: player input → ActionIntent (AI is in AiPlugin)
                dev_player_intent_generate.in_set(SimSet::Vision),
                effects_phase.in_set(SimSet::Effects),
                healing_phase.in_set(SimSet::Healing),
                tick_bionics.in_set(SimSet::Bionics),
                tick_morale_decay.in_set(SimSet::Morale),
                temperature_phase.in_set(SimSet::Temperature),
                update_vision.in_set(SimSet::Vision),
                process_item_move_events.in_set(SimSet::Inventory),
                assign_invlets_system.in_set(SimSet::Inventory),
                build_inventory_bins.in_set(SimSet::Inventory),
            )
                .run_if(in_state(AppState::InGame)),
        );

        // ── UI input adapters (presenter layer) — screen keyboard input that
        // translates `InputAction` into sim use-cases. Lives in cdda_render so
        // cdda_sim never matches the display-UI `GameAction` enum.
        app.add_systems(
            Update,
            cdda_render::render::input::dev_pickup_drop_system
                .in_set(SimSet::Inventory)
                .run_if(in_state(Screen::Gameplay))
                .run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            Update,
            cdda_render::render::input::inventory_screen_input
                .in_set(SimSet::Inventory)
                .run_if(in_state(Screen::Inventory))
                .run_if(in_state(AppState::InGame)),
        );

        // ── Dev player movement — now generates ActionIntent instead of
        // moving directly.  intent resolution handles the actual move.
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
    // Register the custom `cdda` asset source (rooted at repo `data/`) BEFORE
    // DefaultPlugins adds AssetPlugin, so the source is built at startup.
    data_assets::register_cdda_asset_source(&mut app);
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
