use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A proficiency category definition from JSON type `"proficiency_category"`.
///
/// Defines a category for grouping proficiencies.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProficiencyCategoryDef {
    /// Unique identifier (e.g. "prof_combat", "prof_woodworking").
    pub id: DefId<ProficiencyCategoryDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<serde_json::Value>,
}
