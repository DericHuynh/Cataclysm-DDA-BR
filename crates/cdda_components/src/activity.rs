//! ECS components for the activity system.
//!
//! Each activity type gets its own component (e.g. `Crafting`, `Aiming`).
//! `ActivityProgress` tracks common progress fields for any activity.
//!
//! One actor owns one progress/type pair. The simulation validates this invariant
//! before dispatch; multiple type tags do not create independent work budgets.
//! Interruption removes the pair, retaining the craft item for validated resume.
//!
//! ## Why these live in `cdda_components`
//!
//! Activity state is cross-cutting: the simulation ticks it, the UI reads and
//! displays it (crafting panel), combat applies weariness penalties, and the
//! inventory/body systems coordinate with it (craft → equip → drop). Per the
//! workspace contract, all Bevy `Component`s live here so any layer can query
//! them by marker + component without importing another domain's logic.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

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
    /// Activity is in progress (advanced by the simulation budget).
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

// ===========================================================================
// ActivityTracker — weariness and calorie balance
// ===========================================================================

/// Exertion constant: no exercise at all.
pub const NO_EXERCISE: f32 = 0.0;
/// Exertion constant: light exercise.
pub const LIGHT_EXERCISE: f32 = 0.1;
/// Exertion constant: moderate exercise.
pub const MODERATE_EXERCISE: f32 = 0.2;
/// Exertion constant: brisk exercise.
pub const BRISK_EXERCISE: f32 = 0.4;
/// Exertion constant: active (heavy) exercise.
pub const ACTIVE_EXERCISE: f32 = 0.6;
/// Exertion constant: extra-active exercise.
pub const EXTRA_EXERCISE: f32 = 1.0;

/// Weariness threshold denominator. Tracker/intake ratio above this = weariness level 1.
const WEARINESS_THRESHOLD: i32 = 4000;

/// Tracks weariness (fatigue) from exertion and calorie balance.
///
/// Attached to every character entity. Updated each turn by the activity
/// tick systems via `log_activity()` and `new_turn()`. Weariness is queried by
/// UI, combat, and stamina systems to apply fatigue penalties.
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, Default)]
pub struct ActivityTracker {
    /// Cumulative weariness tracker (increases with exertion calories burned).
    pub tracker: i32,

    /// Cumulative calorie intake (increases with eating).
    pub intake: i32,

    /// Exertion level logged for the current turn.
    current_activity: f32,

    /// Accumulated exertion across all events this period.
    accumulated_activity: f32,

    /// Average exertion from the previous completed period.
    previous_activity: f32,

    /// Exertion level from the previous turn (used by `instantaneous_activity_level`).
    previous_turn_activity: f32,

    /// Whether the accumulated activity has been reset for this period.
    activity_reset: bool,

    /// Number of events logged this period (for averaging).
    num_events: i32,

    /// Semi-consecutive 5-minute ticks of low activity.
    /// Fractional to handle mixed sleeping/active periods.
    low_activity_ticks: f32,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            activity_reset: true,
            num_events: 1,
            ..Default::default()
        }
    }

    /// Log an activity level for the current turn.
    /// If called multiple times per turn, preserves the highest value.
    pub fn log_activity(&mut self, new_level: f32) {
        if self.activity_reset {
            self.accumulated_activity += self.current_activity;
            self.activity_reset = false;
        }
        if new_level > self.current_activity {
            self.current_activity = new_level;
        }
    }

    /// Inform the tracker that a new game turn has started.
    /// `sleeping` halves the low-activity tick increment.
    pub fn new_turn(&mut self, sleeping: bool) {
        self.previous_turn_activity = self.current_activity;
        self.accumulated_activity += self.current_activity;
        self.num_events += 1;

        let low_threshold = LIGHT_EXERCISE;
        if self.current_activity < low_threshold {
            let increment = if sleeping { 0.5 } else { 1.0 };
            self.low_activity_ticks += increment;
        } else {
            self.low_activity_ticks = (self.low_activity_ticks - 1.0).max(0.0);
        }

        self.current_activity = 0.0;
        self.activity_reset = true;
    }

    /// Reset the accumulated activity level for a new measurement period.
    pub fn reset_activity_level(&mut self) {
        self.previous_activity = self.average_activity();
        self.accumulated_activity = 0.0;
        self.num_events = 1;
    }

    /// Returns the average exertion level for the current measurement period.
    pub fn average_activity(&self) -> f32 {
        if self.num_events == 0 {
            return 0.0;
        }
        self.accumulated_activity / self.num_events as f32
    }

    /// Returns the activity level for the current turn.
    /// If sleeping, halves the value.
    pub fn activity(&self, sleeping: bool) -> f32 {
        if sleeping {
            self.current_activity * 0.5
        } else {
            self.current_activity
        }
    }

    /// Returns the previous turn's activity level until an action is taken
    /// this turn (i.e. before `log_activity` is called for the first time).
    pub fn instantaneous_activity_level(&self) -> f32 {
        if self.activity_reset {
            self.previous_turn_activity
        } else {
            self.current_activity
        }
    }

    /// Returns the current weariness level (0 = not weary, higher = more fatigued).
    ///
    /// Weariness is `tracker / (intake + threshold)` clamped to integer levels.
    /// Mirrors `activity_tracker::weariness()` from the C++ implementation.
    pub fn weariness(&self) -> i32 {
        if self.tracker <= 0 {
            return 0;
        }
        let denominator = self.intake + WEARINESS_THRESHOLD;
        if denominator <= 0 {
            return 0;
        }
        self.tracker / denominator
    }

    /// Attempt to reduce weariness every 5 minutes of low activity.
    ///
    /// `bmr` is the character's base metabolic rate (calories/day).
    /// `sleepiness_mod` scales the reduction rate when drowsy.
    /// `sleepiness_regen_mod` scales additional regen during sleep.
    pub fn try_reduce_weariness(
        &mut self,
        bmr: i32,
        sleepiness_mod: f32,
        sleepiness_regen_mod: f32,
    ) {
        const TICKS_PER_REDUCTION: f32 = 3.0;

        if self.low_activity_ticks < TICKS_PER_REDUCTION {
            return;
        }

        self.low_activity_ticks -= TICKS_PER_REDUCTION;

        // BMR-based recovery: lower BMR → slower weariness recovery.
        let base_reduction = (bmr as f32 / 24.0 * sleepiness_mod) as i32;
        let regen_bonus = (bmr as f32 / 48.0 * sleepiness_regen_mod) as i32;
        let total_reduction = base_reduction + regen_bonus;

        self.tracker = (self.tracker - total_reduction).max(0);
    }

    /// Adjust calorie balance.
    /// Positive `ncal` is intake (eating); negative `ncal` is exertion (adds to tracker).
    pub fn calorie_adjust(&mut self, ncal: i32) {
        if ncal >= 0 {
            self.intake += ncal;
        } else {
            self.tracker += -ncal;
        }
    }

    /// Clear all weariness state.
    pub fn weary_clear(&mut self) {
        self.tracker = 0;
        self.intake = 0;
        self.low_activity_ticks = 0.0;
    }

    /// Returns a human-readable description of the current activity level.
    pub fn activity_level_str(&self) -> &'static str {
        let avg = self.average_activity();
        if avg >= EXTRA_EXERCISE {
            "extra active"
        } else if avg >= ACTIVE_EXERCISE {
            "active"
        } else if avg >= BRISK_EXERCISE {
            "brisk"
        } else if avg >= MODERATE_EXERCISE {
            "moderate"
        } else if avg >= LIGHT_EXERCISE {
            "light"
        } else {
            "no exercise"
        }
    }
}
