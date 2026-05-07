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
use std::time::Duration;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_state::app::AppExtStates;
use bevy_state::prelude::OnEnter;
use bevy_state::state::NextState;

use cdda_ui::Screen;
use cdda_core::{GameSet, SimSet};
use cdda_input::{GameAction, InputAction};

use cdda_sim::def_world::load_data_system;
use cdda_sim::state::AppState;
use cdda_sim::systems::ai::ai_phase;
use cdda_sim::systems::bionics::tick_bionics;
use cdda_sim::systems::combat::combat_phase;
use cdda_sim::systems::effects::effects_phase;
use cdda_sim::systems::healing::healing_phase;
use cdda_sim::systems::morale::tick_morale_decay;
use cdda_sim::systems::movement::movement_phase;
use cdda_sim::systems::spatial::update_spatial_index;
use cdda_sim::systems::spawning::spawning_phase;
use cdda_sim::systems::temperature::temperature_phase;
use cdda_sim::systems::turn::{debug_turn_queue, tick_move_points};
use cdda_sim::systems::vision::update_vision;
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
// Reflect type registration
// ---------------------------------------------------------------------------

fn register_reflect_types(app: &mut App) {
    use cdda_sim::components::{InFlight, Solid, Velocity, WorldPosition};
    use cdda_actor::components::*;
    use cdda_item::components::*;

    // Spatial
    app.register_type::<WorldPosition>();
    app.register_type::<Solid>();
    app.register_type::<Velocity>();
    app.register_type::<InFlight>();

    // Creature identity
    app.register_type::<Creature>();
    app.register_type::<Gender>();
    app.register_type::<PlayerData>();
    app.register_type::<NpcPersonality>();
    app.register_type::<NpcData>();

    // Stats
    app.register_type::<Health>();
    app.register_type::<Stats>();
    app.register_type::<Faction>();
    app.register_type::<BodyTemperature>();
    app.register_type::<Wetness>();

    // Combat
    app.register_type::<DamageReduction>();
    app.register_type::<CombatStats>();
    app.register_type::<Vision>();

    // Skills (relationship-based)
    app.register_type::<SkillOf>();
    app.register_type::<CreatureSkills>();
    app.register_type::<SkillEntry>();

    // Mutations (relationship-based)
    app.register_type::<MutationOf>();
    app.register_type::<CreatureMutations>();
    app.register_type::<MutationEntry>();

    // Proficiencies (relationship-based)
    app.register_type::<ProficiencyOf>();
    app.register_type::<CreatureProficiencies>();
    app.register_type::<ProficiencyEntry>();

    // Bionics
    app.register_type::<BionicOf>();
    app.register_type::<InstalledBionics>();
    app.register_type::<Bionic>();

    // Morale
    app.register_type::<MoraleBonusOf>();
    app.register_type::<MoraleBonuses>();
    app.register_type::<MoraleBonus>();
    app.register_type::<Morale>();

    // Status effects
    app.register_type::<EffectOn>();
    app.register_type::<ActiveEffects>();
    app.register_type::<StatusEffect>();

    // Turn scheduling + status markers
    app.register_type::<MovePoints>();
    app.register_type::<Speed>();
    app.register_type::<IsAlive>();
    app.register_type::<Stunned>();
    app.register_type::<Bleeding>();
    app.register_type::<OnFire>();

    // Body parts
    app.register_type::<BodyPartOf>();
    app.register_type::<CreatureBodyParts>();
    app.register_type::<BodyPartDef>();
    app.register_type::<BodyPartSlot>();
    app.register_type::<BodyPartHp>();
    app.register_type::<BodyPartBroken>();
    app.register_type::<BodyPartSevered>();

    // Item state
    app.register_type::<DefOrigin>();
    app.register_type::<StackCount>();
    app.register_type::<CurrentCharges>();
    app.register_type::<LoadedAmmo>();
    app.register_type::<Spoilable>();
    app.register_type::<ItemDamage>();

    // Container tags
    app.register_type::<Sealed>();
    app.register_type::<Rigid>();
    app.register_type::<Watertight>();
    app.register_type::<PreservesTemp>();
    app.register_type::<Fireproof>();
    app.register_type::<GasTight>();

    // Relationships
    app.register_type::<InsideContainer>();
    app.register_type::<ContainerContents>();
    app.register_type::<WieldedBy>();
    app.register_type::<WieldedItems>();
    app.register_type::<WornOn>();
    app.register_type::<WornBy>();
    app.register_type::<MountedOn>();
    app.register_type::<MountedPockets>();

    // Pocket system
    app.register_type::<Pocket>();
    app.register_type::<PocketType>();
    app.register_type::<PocketRestriction>();
    app.register_type::<AttachmentSlot>();
    app.register_type::<AttachmentType>();
    app.register_type::<Container>();
}

// ---------------------------------------------------------------------------
// Root plugin
// ---------------------------------------------------------------------------

pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        world_setup::setup_world(app.world_mut());
        register_reflect_types(app);

        app.init_state::<AppState>();

        // Fix #5: Drive Screen from AppState so sim and render never desync.
        app.add_systems(OnEnter(AppState::MainMenu), |mut next: ResMut<NextState<Screen>>| {
            next.set(Screen::MainMenu);
        });
        app.add_systems(OnEnter(AppState::DataLoading), |mut next: ResMut<NextState<Screen>>| {
            next.set(Screen::DevWorldgen); // loading screen reuses devworldgen view
        });
        app.add_systems(OnEnter(AppState::WorldGen), |mut next: ResMut<NextState<Screen>>| {
            next.set(Screen::DevWorldgen);
        });
        app.add_systems(OnEnter(AppState::InGame), |mut next: ResMut<NextState<Screen>>| {
            next.set(Screen::Gameplay);
        });

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
                SimSet::SpatialUpdate,
            )
                .chain()
                .in_set(GameSet::Sim),
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

        app.add_systems(Update, load_data_system.run_if(in_state(AppState::DataLoading)));
        app.add_systems(Update, start_game_on_event.run_if(in_state(AppState::MainMenu)));
        app.add_systems(
            Update,
            cdda_sim::def_world::worldgen_system.run_if(in_state(AppState::WorldGen)),
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
                update_spatial_index.in_set(SimSet::SpatialUpdate),
                debug_turn_queue.in_set(SimSet::SpatialUpdate),
            )
                .run_if(in_state(AppState::InGame)),
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
