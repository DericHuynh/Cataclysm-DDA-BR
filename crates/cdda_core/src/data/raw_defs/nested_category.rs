use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A nested category definition from JSON type `"nested_category"`.
///
/// Defines a category that groups related recipes in the crafting menu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NestedCategoryDef {
    /// Unique identifier.
    pub id: DefId<NestedCategoryDef>,

    /// Display name.
    pub name: String,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Category (e.g. "CC_WEAPON").
    #[serde(default)]
    pub category: Option<String>,

    /// Subcategory (e.g. "CSC_WEAPON_RANGED").
    #[serde(default)]
    pub subcategory: Option<String>,

    /// Activity level required.
    #[serde(default)]
    pub activity_level: Option<String>,

    /// List of recipe IDs in this nested category.
    #[serde(default)]
    pub nested_category_data: Vec<String>,

    /// Abstract flag.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
