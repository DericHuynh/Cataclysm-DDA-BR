use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A weapon category definition from JSON type `"weapon_category"`.
///
/// Defines a category of weapons that share proficiency requirements.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeaponCategoryDef {
    /// Unique identifier (e.g. "AUTOMATIC_RIFLES", "KNIVES").
    pub id: DefId<WeaponCategoryDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// List of proficiency IDs required for this weapon category.
    #[serde(default)]
    pub proficiencies: Vec<String>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
