//! ECS components for the activity system.

use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use cdda_core_types::core::coords::WorldPos;

use super::actor::ActivityActor;

// ---------------------------------------------------------------------------
// ActivityTypeId — string ID of an activity_type def
// ---------------------------------------------------------------------------

/// The string identifier of an `activity_type` definition (e.g. `"ACT_READ"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct ActivityTypeId(pub String);

impl ActivityTypeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// ActivityPhase
// ---------------------------------------------------------------------------

/// Lifecycle phase of a `PlayerActivity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum ActivityPhase {
    /// `start()` has not yet been called.
    #[default]
    Pending,
    /// Activity is in progress (`do_turn()` called each tick).
    Active,
    /// Activity has been suspended (can be resumed later).
    Suspended,
    /// Activity has finished or been cancelled — should be removed.
    Done,
}

// ---------------------------------------------------------------------------
// PlayerActivity — the active multi-turn task on a character
// ---------------------------------------------------------------------------

/// The current multi-turn activity assigned to a character.
///
/// This component mirrors the C++ `player_activity` class.
/// Attach to a character entity to begin an activity; remove it when done.
///
/// # Lifecycle
/// 1. Spawn entity with `PlayerActivity { phase: Pending, actor: Some(...), .. }`.
/// 2. `start_pending_activities` calls `actor.start()`, sets `phase = Active`.
/// 3. Each turn `tick_activities` calls `actor.do_turn()`, decrements `moves_left`.
/// 4. When `moves_left <= 0`, `actor.finish()` is called, `phase = Done`.
/// 5. `cleanup_done_activities` removes the component.
///
/// # Mutation
/// Do not mutate `actor` directly after attaching — use ECS commands to replace.
#[derive(Component, Debug)]
pub struct PlayerActivity {
    /// The activity type identifier.
    pub activity_type: ActivityTypeId,

    /// Total moves required to complete the activity.
    pub moves_total: i32,

    /// Remaining moves until completion; decremented each turn.
    pub moves_left: i32,

    /// Current lifecycle phase.
    pub phase: ActivityPhase,

    /// Optional world position relevant to this activity.
    pub placement: Option<WorldPos>,

    /// String targets (item IDs, location tags, etc.).
    pub targets: Vec<String>,

    /// Integer parameters used by the activity logic.
    pub values: Vec<i32>,

    /// String parameters used by the activity logic.
    pub str_values: Vec<String>,

    /// The concrete actor driving this activity's behavior.
    /// `None` only during deserialization before the actor is restored.
    pub actor: Option<ActivityActor>,
}

impl PlayerActivity {
    pub fn new(activity_type: impl Into<String>, actor: ActivityActor) -> Self {
        Self {
            activity_type: ActivityTypeId::new(activity_type),
            moves_total: 0,
            moves_left: 0,
            phase: ActivityPhase::Pending,
            placement: None,
            targets: Vec::new(),
            values: Vec::new(),
            str_values: Vec::new(),
            actor: Some(actor),
        }
    }

    /// Returns `true` if there are no moves remaining.
    pub fn is_complete(&self) -> bool {
        self.moves_left <= 0
    }

    /// Returns `true` if the activity is currently running.
    pub fn is_active(&self) -> bool {
        self.phase == ActivityPhase::Active
    }
}
