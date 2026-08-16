//! ECS components for the activity system.
//!
//! Each activity type gets its own component (e.g. `Crafting`, `Aiming`).
//! `ActivityProgress` tracks common progress fields for any activity.
//!
//! ## Multi-activity architecture
//!
//! Each activity component carries its own progress tracking so that a
//! character can hold multiple activities simultaneously in the future
//! (e.g. dual-wield crafting with a trait).  Currently the simulation only
//! creates one activity at a time, but the component design does not
//! prevent multiple.

use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// ActivityTypeId — string ID of an activity_type def
// ---------------------------------------------------------------------------

/// The string identifier of an `activity_type` definition (e.g. `"ACT_READ"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// Lifecycle phase of an activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivityPhase {
    /// `start()` has not yet been called.
    #[default]
    Pending,
    /// Activity is in progress (ticked each frame).
    Active,
    /// Activity has been suspended (can be resumed later).
    Suspended,
    /// Activity has finished or been cancelled — should be removed.
    Done,
}

// ---------------------------------------------------------------------------
// ActivityProgress — shared progress tracking for any activity
// ---------------------------------------------------------------------------

/// Common move-count progress for any multi-turn activity.
///
/// Attached alongside a type-specific activity component (e.g. `Crafting`).
/// Systems tick both together.
#[derive(Component, Debug)]
pub struct ActivityProgress {
    /// Total moves required to complete the activity.
    pub moves_total: i32,
    /// Remaining moves; decremented each turn.
    pub moves_left: i32,
    /// Current lifecycle phase.
    pub phase: ActivityPhase,
}

impl Default for ActivityProgress {
    fn default() -> Self {
        Self {
            moves_total: 0,
            moves_left: 0,
            phase: ActivityPhase::Pending,
        }
    }
}

impl ActivityProgress {
    pub fn new(moves: i32) -> Self {
        Self {
            moves_total: moves,
            moves_left: moves,
            phase: ActivityPhase::Pending,
        }
    }
    pub fn is_complete(&self) -> bool {
        self.moves_left <= 0
    }
}

// ===========================================================================
// Per-activity data components
// ===========================================================================

/// Crafting activity — drives an `InProgressCraft` entity.
///
/// Each `tick_crafting` system call spends AP, advances `InProgressCraft::ap_spent`,
/// and emits `CraftCompleted` when done.
#[derive(Component, Debug, Clone)]
pub struct Crafting {
    /// The `InProgressCraft` entity in the player's inventory.
    pub craft_entity: Entity,
}

/// Aiming activity — accumulates aim percent each tick.
#[derive(Component, Debug, Clone)]
pub struct Aiming {
    pub target_aim_percent: u32,
    pub cur_aim: u32,
}

/// Reading activity — reads a book for skill/morale gains.
#[derive(Component, Debug, Clone)]
pub struct Reading {
    pub book_entity: Entity,
    pub skill_id: String,
    pub turns_read: i32,
    pub turns_total: i32,
}

/// Waiting activity — burn turns doing nothing.
#[derive(Component, Debug, Clone)]
pub struct Waiting {
    pub turns: i32,
}

/// Reloading activity — load ammo into a weapon.
#[derive(Component, Debug, Clone)]
pub struct Reloading {
    pub item_entity: Entity,
    pub ammo_entity: Entity,
    pub quantity: i32,
    pub speed_factor: f32,
}

/// Generic interaction activity (examining, using furniture, etc.).
#[derive(Component, Debug, Clone)]
pub struct Interacting {
    pub description: String,
    pub duration: i32,
}
