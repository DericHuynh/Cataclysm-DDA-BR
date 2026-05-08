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
pub use replay::{ReplayState, ReplaySpeed, inject_replay_actions};

use bevy_app::{App, Plugin, Update};
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
        app.add_systems(Update, record_actions);

        #[cfg(feature = "devtools")]
        {
            app.insert_resource(state_hash::StateHashLog::default());
            app.add_systems(Update, state_hash::hash_simulation_state);
        }
    }
}

// ---------------------------------------------------------------------------
// Replay plugin
// ---------------------------------------------------------------------------

pub struct CddaReplayModePlugin;

impl Plugin for CddaReplayModePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(crate::replay::ReplayState::default());
        app.add_systems(Update, crate::replay::inject_replay_actions);

        #[cfg(feature = "devtools")]
        {
            app.insert_resource(state_hash::StateHashLog::default());
            app.add_systems(Update, state_hash::hash_simulation_state);
            app.add_systems(Update, state_hash::check_divergence);
            app.add_message::<state_hash::SimulationDiverged>();
        }
    }
}
