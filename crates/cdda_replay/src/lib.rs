//! # cdda_replay — Deterministic session recording & replay
//!
//! Records player actions into a `SessionLog`, replays them injectively.
//! Combined with a deterministic world seed, this enables exact bug
//! reproduction and regression testing.
//!
//! ## Features
//!
//! - `devtools` — enables state hashing per turn + divergence detection.

pub mod recording;
pub mod replay;
pub mod session_log;
pub mod state_hash;
pub use replay::{inject_replay_actions, ReplaySpeed, ReplayState};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use cdda_components::schedule::GameSet;
use recording::record_actions;
use session_log::SessionLog;

// ---------------------------------------------------------------------------
// Recording plugin
// ---------------------------------------------------------------------------

pub struct CddaReplayPlugin {
    pub world_seed: u64,
}

impl Plugin for CddaReplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SessionLog::new(self.world_seed));
        // Ingress ordering: record declared input BEFORE the simulation driver
        // consumes the frame; hash COMMITTED state after the driver ran.
        app.add_systems(Update, record_actions.in_set(GameSet::Input));

        #[cfg(feature = "devtools")]
        {
            app.insert_resource(state_hash::StateHashLog::default());
            app.add_systems(
                Update,
                state_hash::hash_simulation_state.in_set(GameSet::Render),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Replay plugin
// ---------------------------------------------------------------------------

pub struct CddaReplayModePlugin;

impl Plugin for CddaReplayModePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplayState::default());
        app.add_systems(Update, inject_replay_actions.in_set(GameSet::Input));

        #[cfg(feature = "devtools")]
        {
            app.insert_resource(state_hash::StateHashLog::default());
            app.add_systems(
                Update,
                (
                    state_hash::hash_simulation_state,
                    state_hash::check_divergence,
                )
                    .chain()
                    .in_set(GameSet::Render),
            );
            app.add_message::<state_hash::SimulationDiverged>();
        }
    }
}
