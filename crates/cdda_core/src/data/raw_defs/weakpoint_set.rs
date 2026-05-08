use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A weakpoint set definition from JSON type `"weakpoint_set"`.
///
/// Defines a set of weakpoints for a monster, specifying locations with
/// modified damage multipliers, armor penetration, critical hit chances,
/// coverage, and special effects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeakpointSetDef {
    /// Unique identifier (e.g. "wps_amalgamation_base", "wps_arthropod").
    pub id: DefId<WeakpointSetDef>,

    /// List of weakpoints in this set.
    #[serde(default)]
    pub weakpoints: Option<Vec<Weakpoint>>,
}

/// A single weakpoint on a monster.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Weakpoint {
    /// Identifier for this weakpoint (e.g. "leg", "head", "eye", "acid_gland").
    #[serde(default)]
    pub id: Option<String>,

    /// Display name of the weakpoint (e.g. "a leg", "the head", "the eye").
    #[serde(default)]
    pub name: Option<String>,

    /// Whether this weakpoint is on the head.
    #[serde(default)]
    pub is_head: Option<bool>,

    /// Difficulty to hit this weakpoint (melee and/or ranged).
    #[serde(default)]
    pub difficulty: Option<serde_json::Value>,

    /// Damage multiplier when hitting this weakpoint (by damage type).
    #[serde(default)]
    pub damage_mult: Option<serde_json::Value>,

    /// Critical hit multiplier when hitting this weakpoint.
    #[serde(default)]
    pub crit_mult: Option<serde_json::Value>,

    /// Armor multiplier when hitting this weakpoint (by damage type).
    #[serde(default)]
    pub armor_mult: Option<serde_json::Value>,

    /// Coverage multiplier (e.g. {"point": 0.75} for smaller hit area).
    #[serde(default)]
    pub coverage_mult: Option<serde_json::Value>,

    /// Base coverage percentage (0-100).
    #[serde(default)]
    pub coverage: Option<serde_json::Value>,

    /// Effects applied when this weakpoint is hit.
    #[serde(default)]
    pub effects: Option<Vec<WeakpointEffect>>,
}

/// An effect triggered when a weakpoint is hit.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeakpointEffect {
    /// Effect ID (e.g. "staggered", "blind", "maimed_leg", "maimed_acid_gland").
    #[serde(default)]
    pub effect: Option<String>,

    /// Chance of the effect triggering (0-100).
    #[serde(default)]
    pub chance: Option<u32>,

    /// Duration of the effect in turns (can be [min, max] or single value).
    #[serde(default)]
    pub duration: Option<serde_json::Value>,

    /// Message displayed when the effect triggers.
    #[serde(default)]
    pub message: Option<String>,

    /// Damage required to trigger this effect as [min, max].
    #[serde(default)]
    pub damage_required: Option<Vec<u32>>,

    /// Whether this effect causes instant death.
    #[serde(default)]
    pub instant_death_chance: Option<u32>,

    /// Whether this effect is permanent.
    #[serde(default)]
    pub permanent: Option<bool>,

    /// Intensity (can be a single value or [min, max]).
    #[serde(default)]
    pub intensity: Option<serde_json::Value>,
}
