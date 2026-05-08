use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A damage type definition from JSON type `"damage_type"`.
///
/// Defines a type of damage (e.g. "bash", "cut", "stab", "bullet").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DamageTypeDef {
    /// Unique identifier (e.g. "bash", "cut", "stab").
    pub id: DefId<DamageTypeDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Whether this damage type is melee-only.
    #[serde(default)]
    pub melee_only: Option<bool>,

    /// Whether this is physical damage.
    #[serde(default)]
    pub physical: Option<bool>,

    /// Whether this is edged damage.
    #[serde(default)]
    pub edged: Option<bool>,

    /// Whether this is environmental damage.
    #[serde(default)]
    pub environmental: Option<bool>,

    /// Color used for magic damage indicators.
    #[serde(default)]
    pub magic_color: Option<String>,

    /// Skill used for this damage type.
    #[serde(default)]
    pub skill: Option<String>,

    /// Bash conversion factor.
    #[serde(default)]
    pub bash_conversion_factor: Option<f64>,

    /// Whether this damage contributes to monster difficulty.
    #[serde(default)]
    pub mon_difficulty: Option<bool>,

    /// Whether this damage requires materials to protect against.
    #[serde(default)]
    pub material_required: Option<bool>,

    /// Immune flags for characters and monsters.
    #[serde(default)]
    pub immune_flags: Option<serde_json::Value>,

    /// Derived from another damage type.
    #[serde(default)]
    pub derived_from: Option<serde_json::Value>,

    /// Whether this damage type has no resistance.
    #[serde(default)]
    pub no_resist: Option<bool>,
}
