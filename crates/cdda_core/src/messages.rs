//! Common messages shared across crate boundaries.
//!
//! These are globally broadcast `Message` types (not observer-based `Event`s)
//! that any system can subscribe to via `MessageReader<T>`.

use bevy_ecs::message::Message;

/// Broadcast when the game advances by one turn.
///
/// Emitted by `crate::actor::turn::tick_move_points` after granting
/// move points, updating the turn counter, and rebuilding `TurnQueue`.
///
/// Systems that tick over time (spoilage, status effects, temperature)
/// should subscribe to this to advance their timers.
#[derive(Message, Debug, Clone, Copy)]
pub struct TurnAdvanced {
    /// The new turn number (1-based after the first tick).
    pub turn: u64,
}
