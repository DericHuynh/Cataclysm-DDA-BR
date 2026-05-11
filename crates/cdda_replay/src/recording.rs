//! Recording system — captures `InputAction` messages into the `SessionLog`.
//!
//! Reads `GameTime.turn` from `cdda_sim` to stamp each action with
//! the current turn number.

use crate::session_log::{ActionRecord, SessionLog};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use cdda_components::input::InputAction;
use cdda_components::sim::GameTime;

/// Records `InputAction` messages into the `SessionLog`, stamped with
/// the current turn from `GameTime`.
pub fn record_actions(
    mut action_reader: MessageReader<InputAction>,
    mut log: ResMut<SessionLog>,
    game_time: Res<GameTime>,
) {
    for action in action_reader.read() {
        log.actions.push(ActionRecord {
            turn: game_time.turn,
            action: action.action.clone(),
            source: action.source,
        });
    }
}
