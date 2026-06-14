use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How activity progress is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BasedOnType {
    /// Progress measured in real time (independent of character speed).
    Time,
    /// Progress measured in character move points (speed-dependent).
    #[default]
    Speed,
    /// Progress driven entirely by the do_turn handler or actor.
    Neither,
}

/// An event that can interrupt an ongoing activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DistractionType {
    Noise,
    Pain,
    Attacked,
    HostileSpottedFar,
    HostileSpottedNear,
    TalkedTo,
    Asthma,
    MotionAlarm,
    WeatherChange,
    PortalStormPopup,
    Eoc,
    DangerousField,
    Hunger,
    Thirst,
    Temperature,
    Mutation,
    Oxygen,
    Withdrawal,
}

/// An activity type definition from JSON type `"activity_type"`.
///
/// Defines a player activity (e.g. reading, reloading, crafting) with its
/// verb display text and behavioral flags. Maps directly to the C++
/// `activity_type` class fields loaded from `activity_type.cpp::load()`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActivityTypeDef {
    /// Unique identifier (e.g. "ACT_RELOAD", "ACT_READ").
    pub id: DefId<ActivityTypeDef>,

    /// Verb phrase describing the activity (e.g. "reloading", "reading").
    #[serde(default)]
    pub verb: Option<LocalizedString>,

    /// Whether the character is rooted in place during the activity.
    #[serde(default)]
    pub rooted: Option<bool>,

    /// Whether the activity can be interrupted by distractions.
    #[serde(default = "default_true")]
    pub interruptable: bool,

    /// Whether the activity can be interrupted by keyboard input.
    #[serde(default = "default_true")]
    pub interruptable_with_kb: bool,

    /// Whether the activity can be suspended and resumed.
    #[serde(default = "default_true")]
    pub can_resume: bool,

    /// Whether this is a multi-activity (runs multiple sub-activities).
    #[serde(default)]
    pub multi_activity: bool,

    /// Whether the activity should fetch items to an associated zone.
    #[serde(default = "default_true")]
    pub fetch_items_to_zone: bool,

    /// If true, the character will refuel an adjacent fire if firewood is nearby.
    #[serde(default)]
    pub refuel_fires: bool,

    /// If true, the character will automatically consume from auto-eat/drink zones.
    #[serde(default)]
    pub auto_needs: bool,

    /// How activity progress is measured.
    #[serde(default)]
    pub based_on: BasedOnType,

    /// Exertion level during this activity (NO_EXERCISE = 0.0 to MAX_EXERCISE = 1.0+).
    pub activity_level: f32,

    /// Distraction types that are ignored by default for this activity.
    #[serde(default)]
    pub ignored_distractions: Vec<DistractionType>,

    /// Effect-on-condition to run when the activity completes.
    #[serde(default)]
    pub completion_eoc: Option<String>,

    /// Effect-on-condition to run each turn of the activity.
    #[serde(default)]
    pub do_turn_eoc: Option<String>,
}

fn default_true() -> bool {
    true
}
