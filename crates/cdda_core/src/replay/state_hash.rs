//! State hashing and divergence detection.
//!
//! Hashes all `SimId`-tagged entities each turn.  During replay,
//! compares live hashes against recorded hashes in the `SessionLog`.

use bevy_ecs::message::Message;
use bevy_ecs::prelude::*;
use crate::SimId;
use crate::sim::state::GameTime;

use crate::replay::session_log::SessionLog;

// ---------------------------------------------------------------------------
// StateHashLog
// ---------------------------------------------------------------------------

#[derive(Resource, Default, Debug, Clone)]
pub struct StateHashLog {
    pub hashes: Vec<(u64, u64)>, // (turn, hash)
}

// ---------------------------------------------------------------------------
// hash_simulation_state
// ---------------------------------------------------------------------------

/// Hashes entity count + sorted `SimId` values each turn.
/// Stores hash in `StateHashLog` + `SessionLog.state_hashes`.
pub fn hash_simulation_state(
    entities: Query<&SimId>,
    hash_log: ResMut<StateHashLog>,
    session_log: ResMut<SessionLog>,
    game_time: Res<GameTime>,
) {
    if cfg!(not(feature = "devtools")) {
        let _ = (entities, hash_log, session_log, game_time);
        return;
    }

    #[cfg(feature = "devtools")]
    {
        let mut ids: Vec<u64> = entities.iter().map(|id| id.0).collect();
        ids.sort();

        let mut hasher = FxHasher::default();
        ids.len().hash(&mut hasher);
        for id in &ids {
            id.hash(&mut hasher);
        }

        let h = hasher.finish();
        hash_log.hashes.push((game_time.turn, h));
        session_log.state_hashes.push((game_time.turn, h));
    }
}

// ---------------------------------------------------------------------------
// check_divergence
// ---------------------------------------------------------------------------

/// During replay, compares live state hashes against the recorded log.
/// Fires `SimulationDiverged` when a mismatch is found.
pub fn check_divergence(
    hash_log: Res<StateHashLog>,
    session_log: Res<SessionLog>,
    game_time: Res<GameTime>,
    mut divergence_writer: MessageWriter<SimulationDiverged>,
) {
    let turn = game_time.turn;

    // Find the live hash for this turn
    let live = hash_log.hashes.iter().find(|(t, _)| *t == turn);

    // Find the matching recorded hash
    let recorded = session_log.state_hashes.iter().find(|(t, _)| *t == turn);

    if let (Some((_, live_hash)), Some((_, recorded_hash))) = (live, recorded) {
        if live_hash != recorded_hash {
            divergence_writer.write(SimulationDiverged {
                turn,
                detail: format!(
                    "State hash mismatch at turn {turn}: live={live_hash:x}, recorded={recorded_hash:x}"
                ),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// SimulationDiverged
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub struct SimulationDiverged {
    pub turn: u64,
    pub detail: String,
}
