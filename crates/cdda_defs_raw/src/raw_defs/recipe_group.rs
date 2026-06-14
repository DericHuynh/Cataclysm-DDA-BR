use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A recipe group definition from JSON type `"recipe_group"`.
///
/// Organizes recipes into groups for faction base and camp crafting menus.
/// Each group belongs to a building type and contains a list of recipes
/// with optional om_terrain restrictions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeGroupDef {
    /// Unique identifier (e.g. "fbbb_crafting_recipes_basic", "all_faction_base_types").
    pub id: DefId<RecipeGroupDef>,

    /// Building type this group belongs to (e.g. "BASE", "NONE", "WORKSHOP", "FARM").
    #[serde(default)]
    pub building_type: Option<String>,

    /// List of recipes in this group.
    #[serde(default)]
    pub recipes: Option<Vec<RecipeGroupEntry>>,
}

/// A single recipe entry within a recipe group.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeGroupEntry {
    /// Recipe ID.
    pub id: String,

    /// Description displayed in the crafting menu.
    #[serde(default)]
    pub description: Option<String>,

    /// Overmap terrains where this recipe is available.
    #[serde(default)]
    pub om_terrains: Option<Vec<serde_json::Value>>,
}
