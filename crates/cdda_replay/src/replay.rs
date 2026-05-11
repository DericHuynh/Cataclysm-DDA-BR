//! Replay injection — replays `ActionRecord`s from a `SessionLog`.
//!
//! When in replay mode, this system writes the next action from the log
//! as an `InputAction` message, replacing the normal input system.
//! Turn timing is driven by `GameTime`.

use crate::session_log::SessionLog;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::*;
use cdda_components::input::{ActionSource, InputAction};
use cdda_components::sim::GameTime;

// ---------------------------------------------------------------------------
// ReplayState
// ---------------------------------------------------------------------------

/// Current position and playback speed for replay.
#[derive(Resource, Debug, Clone)]
pub struct ReplayState {
    pub cursor: usize,
    pub speed: ReplaySpeed,
    pub paused: bool,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            cursor: 0,
            speed: ReplaySpeed::Fast,
            paused: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaySpeed {
    RealTime,
    Fast,
    Step,
}

// ---------------------------------------------------------------------------
// Replay injection
// ---------------------------------------------------------------------------

/// Injects actions from the `SessionLog` when the turn matches.
///
/// Reads `GameTime.turn` to know when to inject.
/// In `Fast` mode, injects all actions for the current turn.
/// In `RealTime` / `Step`, injects one per call.
pub fn inject_replay_actions(
    log: Res<SessionLog>,
    mut replay: ResMut<ReplayState>,
    game_time: Res<GameTime>,
    mut action_writer: MessageWriter<InputAction>,
) {
    if replay.paused {
        return;
    }

    match replay.speed {
        ReplaySpeed::Fast => {
            while let Some(record) = log.actions.get(replay.cursor) {
                if record.turn == game_time.turn {
                    action_writer.write(InputAction::new(record.action.clone(), record.source));
                    replay.cursor += 1;
                } else if record.turn > game_time.turn {
                    break;
                } else {
                    replay.cursor += 1;
                }
            }
        }
        ReplaySpeed::RealTime | ReplaySpeed::Step => {
            if let Some(record) = log.actions.get(replay.cursor) {
                if record.turn == game_time.turn {
                    action_writer.write(InputAction::new(
                        record.action.clone(),
                        ActionSource::Script,
                    ));
                    replay.cursor += 1;
                }
            }
        }
    }
}

impl ReplayState {
    pub fn is_complete(&self, log: &SessionLog) -> bool {
        self.cursor >= log.actions.len()
    }
}
