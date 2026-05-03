//! # cdda_app — Application binary + plugin registration.
//!
//! Brings together cdda_sim, cdda_render, cdda_input, cdda_audio
//! and wires them into a Bevy application.

use bevy::app::{App, Plugin, Update};
use bevy::DefaultPlugins;
use cdda_core::rng::SeededRng;
use cdda_sim::events::TurnState;
use cdda_sim::world_setup;

/// Root plugin that wires all CDDA subsystems.
pub struct CddaPlugin;

impl Plugin for CddaPlugin {
    fn build(&self, app: &mut App) {
        // Register all simulation components, events, and resources.
        world_setup::setup_world(app.world_mut());

        // State: start waiting for input (no real input yet).
        // In a future iteration, this transitions on keypress.
        app.add_systems(Update, game_tick_system);
    }
}

/// Main tick system — runs once per Bevy Update (frame).
///
/// In full implementation, this would:
/// 1. Check TurnState
/// 2. If PlayerActed → run cdda_sim::tick::run_tick()
/// 3. If Animating → render inter-turn animations
/// 4. Otherwise → wait for input
///
/// For now, just advance the simulation each frame using a dead-reckoning
/// pattern: after each tick, immediately set PlayerActed again.
fn game_tick_system(world: &mut bevy::prelude::World) {
    let turn_state = *world.resource::<TurnState>();

    match turn_state {
        TurnState::WaitingForInput => {
            // No input system yet — auto-advance for testing.
            *world.resource_mut::<TurnState>() = TurnState::PlayerActed;
        }
        TurnState::PlayerActed => {
            // Run simulation tick.
            let mut rng = world
                .remove_resource::<SeededRng>()
                .unwrap_or_else(|| SeededRng::new(0));
            cdda_sim::tick::run_tick(world, &mut rng);
            world.insert_resource(rng);
            *world.resource_mut::<TurnState>() = TurnState::WaitingForInput;
        }
        TurnState::Simulating => {
            // Shouldn't happen in sync mode.
            *world.resource_mut::<TurnState>() = TurnState::WaitingForInput;
        }
        TurnState::Animating => {
            // No animation system yet — skip to waiting.
            *world.resource_mut::<TurnState>() = TurnState::WaitingForInput;
        }
    }
}

/// Launch the CDDA application.
pub fn run() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(CddaPlugin);
    app.run();
}
