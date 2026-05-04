//! # cdda_app — Application binary + plugin registration.
//!
//! Brings together cdda_sim, cdda_render, cdda_input, cdda_audio
//! and wires them into a Bevy application.
//!
//! ## Lifecycle (reference Section 11)
//!
//! ```text
//! AppStart
//!   │
//!   ▼
//! DataLoading ──► WorldGen ──► InGame ◄──► Paused
//!                                    │
//!                                    ▼
//!                               GameOver
//! ```

use bevy::app::{App, Plugin, Update};
use bevy::prelude::*;
use cdda_sim::events::TurnState;
use cdda_sim::state::AppState;
use cdda_sim::systems::turn::{debug_turn_queue, tick_move_points};
use cdda_sim::world_setup;

// ---------------------------------------------------------------------------
// Root plugin
// ---------------------------------------------------------------------------

/// Root plugin that wires all CDDA subsystems.
pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        // ── Register all simulation components, events, and resources ──
        world_setup::setup_world(app.world_mut());

        // ── TurnState and AppState resources ───────────────────────────
        app.insert_resource(TurnState::WaitingForInput);
        app.insert_resource(AppState::DataLoading); // start in DataLoading

        // Exclude IsDef entities from all queries by default.
        // Systems that need definition data explicitly add `With<IsDef>`.
        app.insert_resource(
            bevy_ecs::query::DefaultQueryFilters::new()
                .with::<Without<cdda_sim::def_components::IsDef>>(),
        );

        // ── All-in-one startup/game system ─────────────────────────────
        // Checks AppState each frame and dispatches accordingly.
        // This acts as a simple state machine without requiring bevy_state.
        app.add_systems(Update, app_state_dispatch);

        // ── Render systems ─────────────────────────────────────────────
        // (no-op for now — render crate is a stub)
    }
}

// ---------------------------------------------------------------------------
// Main tick system
// ---------------------------------------------------------------------------

/// App-state dispatch system — runs every Update frame.
///
/// Checks `AppState` and runs the appropriate logic:
/// - `DataLoading`: runs `load_data_system` directly (takes `&mut World`)
/// - `WorldGen`: runs `worldgen_system` directly (takes `&mut World`)
/// - `InGame`: runs the game tick loop
/// - `Paused`/`GameOver`: idle (no-op)
fn app_state_dispatch(world: &mut bevy::prelude::World) {
    let state = *world.resource::<AppState>();

    match state {
        AppState::DataLoading => {
            cdda_sim::def_world::load_data_system(world);
        }
        AppState::WorldGen => {
            cdda_sim::def_world::worldgen_system(world);
        }
        AppState::InGame => {
            game_tick_system(world);
        }
        AppState::Paused | AppState::GameOver => {
            // Idle — no simulation while paused or game over
        }
    }
}

/// Main game tick — runs once per Bevy Update (frame) while in InGame state.
///
/// ## Turn Queue Architecture
///
/// `tick_move_points` (Phase 0) rebuilds the `TurnQueue` resource — a
/// priority queue of all living actors sorted by MP descending.  Currently
/// all actors are processed in batch phases (AI-Movement-Combat-Effects-
/// Spawning).  When the game loop is refactored for proper per-actor turn
/// processing, the queue's `pop_highest()` method will drive actor-by-actor
/// iteration, allowing multiple actions per actor per turn until their MP
/// drops below `MP_MIN_FLOOR`.
///
/// ## Frame-rate behavior
///
/// In the current dead-reckoning loop, each frame advances one full tick
/// (WaitingForInput -> auto-advance -> PlayerActed -> tick ->
/// WaitingForInput).  This is intentional for testing: it makes the
/// simulation observable in real-time.  In production, the transition from
/// `WaitingForInput` to `PlayerActed` should be gated on actual player
/// input, so the game only advances when the player commits an action.
///
/// Flow:
/// 1. All actors gain move points (`tick_move_points`)
/// 2. Actors act in order (AI, movement, combat)
/// 3. Effects tick
/// 4. Spatial index updated
/// 5. Next turn
///
/// For now this is a serial dead-reckoning loop. In the full implementation,
/// the player input runs on demand and only when they commit an action do we
/// advance the simulation by several ticks.
fn game_tick_system(world: &mut bevy::prelude::World) {
    let turn_state = *world.resource::<TurnState>();

    match turn_state {
        TurnState::WaitingForInput => {
            // No input system yet — auto-advance for testing.
            // In production: only transition when the player commits an action.
            *world.resource_mut::<TurnState>() = TurnState::PlayerActed;
        }
        TurnState::PlayerActed => {
            // Run a single simulation tick.
            // Phase order (matching reference Section 10 + Section 8):
            // 0. Tick move points (all actors gain MP)
            // 1. AI — entities decide actions
            // 2. Movement — resolve movement intents
            // 3. Combat — resolve combat actions
            // 4. Effects — status effect tick, needs decay
            // 5. Spawning — spawn new entities from events

            // Phase 0: Grant MP to all actors and rebuild the TurnQueue.
            // The queue is available as a Resource for per-actor iteration
            // when the game loop is refactored.
            let mut sys = IntoSystem::into_system(tick_move_points);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Phase 1: AI
            let mut sys = IntoSystem::into_system(cdda_sim::systems::ai::ai_phase);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Phase 2: Movement
            let mut sys = IntoSystem::into_system(cdda_sim::systems::movement::movement_phase);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Phase 3: Combat
            let mut sys = IntoSystem::into_system(cdda_sim::systems::combat::combat_phase);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Phase 4: Effects
            let mut sys = IntoSystem::into_system(cdda_sim::systems::effects::effects_phase);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Phase 5: Spawning
            let mut sys = IntoSystem::into_system(cdda_sim::systems::spawning::spawning_phase);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Update spatial index after all movements
            let mut sys = IntoSystem::into_system(cdda_sim::systems::spatial::update_spatial_index);
            let _ = sys.run((), world);
            sys.apply_deferred(world);

            // Debug logging every 10 turns
            let mut sys = IntoSystem::into_system(debug_turn_queue);
            let _ = sys.run((), world);
            // No defer needed — debug is read-only

            // Go back to waiting for input
            *world.resource_mut::<TurnState>() = TurnState::WaitingForInput;
        }
        TurnState::Simulating => {
            // Used in async simulation mode (deferred). For now, skip.
            *world.resource_mut::<TurnState>() = TurnState::WaitingForInput;
        }
        TurnState::Animating => {
            // No animation system yet — skip to waiting.
            *world.resource_mut::<TurnState>() = TurnState::WaitingForInput;
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Launch the CDDA application.
pub fn run() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(CddaPlugin);
    app.run();
}
