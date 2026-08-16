//! Action intent types — what an entity intends to do this turn.
//!
//! Each turn entities declare intents by inserting an `ActionIntent` component.
//! These are collected, sorted by action points, and resolved by the intent
//! resolution system.  Preconditions are validated at resolution time so that
//! later intents see the results of earlier resolutions (e.g. a zombie can't
//! attack if it was killed by an earlier action).
//!
//! ## Relationship to activities
//!
//! Some intents start multi-turn activities:
//! - `StartCraft(recipe)` → inserts `(ActivityProgress, Crafting)` component
//! - `StartAim(target)` → inserts `(ActivityProgress, Aiming)` component
//!
//! Single-turn intents (Move, MeleeAttack, Pickup) are resolved immediately.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::WorldPos;

// ---------------------------------------------------------------------------
// ActionIntent — what an entity wants to do
// ---------------------------------------------------------------------------

/// A declared intent to perform an action this turn.
///
/// Insert this component on an entity to declare an intent.  The resolution
/// system sorts all intents by the entity's current action points and resolves
/// them in priority order, validating preconditions before execution.
///
/// # Cancellation
///
/// If the entity's state changes before resolution (e.g. killed by an earlier
/// actor), the intent is cancelled via precondition validation.  The entity
/// is charged no AP for cancelled intents.
#[derive(Component, Debug, Clone)]
pub enum ActionIntent {
    /// Move one tile in a direction (dx, dy, dz).
    Move { dx: i32, dy: i32 },

    /// Melee-attack a target entity.
    MeleeAttack { target: Entity },

    /// Pick up an item entity from the ground.
    Pickup { item: Entity },

    /// Wield an item entity.
    Wield { item: Entity },

    /// Use/consume an item.
    UseItem { item: Entity },

    /// Reload a weapon with ammo.
    Reload { weapon: Entity, ammo: Entity },

    /// Start a multi-turn crafting activity.
    StartCraft { recipe: Entity },

    /// Start reading a book.
    StartRead { book: Entity },

    /// Do nothing this turn.
    Wait,

    /// Interact with furniture/terrain at a position.
    Interact { position: WorldPos },
}

// ---------------------------------------------------------------------------
// Intent queue — global sortable buffer
// ---------------------------------------------------------------------------

/// A time-stamped intent ready for resolution, ordered by entity AP.
#[derive(Debug, Clone)]
pub struct QueuedIntent {
    /// The entity that declared the intent.
    pub entity: Entity,
    /// The declared intent.
    pub intent: ActionIntent,
    /// The entity's current action points (used for sorting).
    pub ap: i32,
}

/// Collects and sorts all `ActionIntent`s before resolution.
///
/// Built by `collect_intents` during `SimSet::IntentDeclare`, drained by
/// `resolve_intents` during `SimSet::IntentResolve`.  Cleared every turn.
#[derive(Resource, Debug, Default)]
pub struct IntentQueue {
    /// Intents sorted by AP descending (highest AP acts first).
    pub queued: Vec<QueuedIntent>,
    /// Count of intents rejected this turn (precondition failure).
    pub rejected: u32,
}
