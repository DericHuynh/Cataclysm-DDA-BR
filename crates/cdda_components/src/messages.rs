//! Common message types shared across crate boundaries.

use bevy_ecs::message::Message;

/// Broadcast when the game advances by one turn.
#[derive(Message, Debug, Clone, Copy)]
pub struct TurnAdvanced {
    /// The new turn number (1-based after the first tick).
    pub turn: u64,
}
