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

    /// Wield an owned inventory item or a nearby unowned ground item.
    Wield { item: Entity },

    /// Drop an owned item at the actor's current tile.
    Drop { item: Entity },

    /// Unwield into the actor's body pocket (or loose inventory if absent).
    Stow { item: Entity },

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
    /// Correlated request identity (stamped by `collect_intents`).
    pub request: ActionRequestId,
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

// ---------------------------------------------------------------------------
// Correlated action results — the request/result simulation contract
// ---------------------------------------------------------------------------
//
// Submitting an intent is NOT the same as completing an action. The resolver
// may reject a request (dead actor, negative AP, vanished target) or refuse an
// unsupported operation entirely, and a planner that advanced its cursor on
// mere submission would desync from the simulation. Every declared request is
// therefore stamped with an [`ActionRequestId`], and after resolution the
// simulation writes an [`ActionOutcome`] component onto the actor carrying the
// terminal verdict for THAT request id. Interim states need no component: a
// request with no outcome yet (or an outcome for an older id) is pending or
// running. Terminal outcomes persist until the actor's next declaration
// replaces them — consumers must match on the request id, never assume the
// component is fresh.

/// Monotonic allocator for [`ActionRequestId`]s.
#[derive(Resource, Debug, Default)]
pub struct ActionRequestCounter(u64);

impl ActionRequestCounter {
    /// Allocate the next request id.
    pub fn next(&mut self) -> ActionRequestId {
        self.0 += 1;
        ActionRequestId(self.0)
    }

    /// The most recently allocated id (0 before any allocation).
    pub fn last(&self) -> u64 {
        self.0
    }
}

/// Identity of one declared action request (correlation key between the
/// declaring agent and the simulation's terminal verdict).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionRequestId(pub u64);

/// Terminal verdict of the simulation for one [`ActionRequestId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOutcomeState {
    /// The operation was performed by the simulation (world effects applied or
    /// dispatched through the authoritative path).
    Completed,
    /// The request was refused before execution (dead actor, negative AP,
    /// vanished target). No AP charged.
    Rejected,
    /// The request was accepted but the operation is not implemented on the
    /// intent path — nothing was performed. No AP charged. Unsupported
    /// actions must never report `Completed`.
    Failed,
    /// The request was withdrawn before resolution (interrupt). No AP charged.
    Cancelled,
}

/// The simulation's terminal verdict for the actor's most recent action
/// request. Persisted until the actor's next declaration replaces it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionOutcome {
    /// The request this verdict belongs to.
    pub request: ActionRequestId,
    /// The terminal state.
    pub state: ActionOutcomeState,
}

impl ActionOutcome {
    /// Build an outcome for `request`.
    pub fn new(request: ActionRequestId, state: ActionOutcomeState) -> Self {
        Self { request, state }
    }

    /// Whether this outcome is the terminal verdict for `request`.
    pub fn matches(&self, request: ActionRequestId) -> bool {
        self.request == request
    }
}
