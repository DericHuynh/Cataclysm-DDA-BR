use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A proficiency definition from JSON type `"proficiency"`.
///
/// Defines a learnable proficiency (e.g. "Parkour Expert", "Archery").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProficiencyDef {
    /// Unique identifier (e.g. "prof_parkour").
    pub id: DefId<ProficiencyDef>,

    /// Category identifier (e.g. "prof_athletics").
    pub category: String,

    /// Display name (e.g. "Parkour Expert").
    pub name: LocalizedString,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Whether the proficiency can be learned.
    #[serde(default)]
    pub can_learn: Option<bool>,

    /// Estimated time to learn.
    #[serde(default)]
    pub time_to_learn: Option<String>,

    /// Required proficiencies.
    #[serde(default)]
    pub required_proficiencies: Option<Vec<serde_json::Value>>,

    /// Default skill used for learning.
    #[serde(default)]
    pub default_skill: Option<String>,

    /// Default skill level required.
    #[serde(default)]
    pub default_skill_level: Option<u32>,

    /// Abstract flag.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
