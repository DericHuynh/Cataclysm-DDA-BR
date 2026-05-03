use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A skill definition from JSON type `"skill"`.
///
/// Defines a skill (e.g. "mechanics", "cooking", "bashing").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillDef {
    /// Unique identifier (e.g. "mechanics", "cooking").
    pub id: DefId<SkillDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text (may be a string, an array-of-objects translation block, etc.).
    #[serde(default)]
    pub description: Option<serde_json::Value>,

    /// Display category.
    #[serde(default)]
    pub display_category: Option<String>,

    /// Whether the skill is combat-related.
    #[serde(default)]
    pub combat_skill: Option<bool>,

    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Abstract flag — if true, this definition is a template that should not be
    /// instantiated directly.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Extension data merged into this definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Time to attack
    #[serde(default)]
    pub time_to_attack: Option<serde_json::Value>,

    /// Companion skill practice
    #[serde(default)]
    pub companion_skill_practice: Option<serde_json::Value>,

    /// Level descriptions (practice)
    #[serde(default)]
    pub level_descriptions_practice: Option<Vec<serde_json::Value>>,

    /// Level descriptions (theoretical)
    #[serde(default)]
    pub level_descriptions_theory: Option<Vec<serde_json::Value>>,

    /// Sort rank in UI
    #[serde(default)]
    pub sort_rank: Option<i32>,
}
