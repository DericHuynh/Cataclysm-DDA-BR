use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A recipe category definition from JSON type `"recipe_category"`.
///
/// Defines a category for grouping recipes in the crafting menu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeCategoryDef {
    /// Unique identifier (e.g. "CC_WEAPON", "CC_FOOD").
    pub id: DefId<RecipeCategoryDef>,

    /// List of subcategories.
    #[serde(default)]
    pub recipe_subcategories: Vec<String>,

    /// Whether this is a wildcard category.
    #[serde(default)]
    pub is_wildcard: Option<bool>,

    /// Whether this is a building category.
    #[serde(default)]
    pub is_building: Option<bool>,

    /// Whether this is a practice category.
    #[serde(default)]
    pub is_practice: Option<bool>,

    /// Whether this category is hidden.
    #[serde(default)]
    pub is_hidden: Option<bool>,
}
