//! Common message types shared across crate boundaries.

use bevy_ecs::entity::Entity;
use bevy_ecs::message::Message;

/// Broadcast when the game advances by one turn.
#[derive(Message, Debug, Clone, Copy)]
pub struct TurnAdvanced {
    /// The new turn number (1-based after the first tick).
    pub turn: u64,
}

/// A crafting activity has completed.
///
/// The crafting system reads this message and spawns the result item.
/// This decouples the activity system from crafting logic — no function
/// pointer or global hook needed.
#[derive(Message, Debug, Clone, Copy)]
pub struct CraftCompleted {
    /// The character entity that was crafting.
    pub crafter: Entity,
    /// The `InProgressCraft` entity that finished.
    pub craft_entity: Entity,
}
