use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A skill display type definition from JSON type `"skill_display_type"`.
///
/// Defines a category for displaying skills in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillDisplayTypeDef {
    /// Unique identifier (e.g. "display_melee", "display_crafting").
    pub id: DefId<SkillDisplayTypeDef>,

    /// The display string shown in the UI.
    pub display_string: Option<String>,
}
