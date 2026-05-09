use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A construction definition from JSON type `"construction"`.
///
/// Defines a construction or deconstruction activity that the player
/// can perform to modify the world.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConstructionDef {
    /// Unique identifier (e.g. "constr_remove_window_empty").
    pub id: DefId<ConstructionDef>,

    /// Group this construction belongs to (for UI grouping).
    #[serde(default)]
    pub group: Option<DefId<crate::core::raw_defs::construction_group::ConstructionGroupDef>>,

    /// Construction category (e.g. "CONSTRUCT", "DECONSTRUCT", "REPAIR",
    /// "REINFORCE", "WINDOWS", "BULK", "FURNITURE").
    #[serde(default)]
    pub category: Option<String>,

    /// Required skills as pairs of [skill_id, level].
    /// e.g. [["fabrication", 3]].
    #[serde(default)]
    pub required_skills: Option<Vec<Vec<serde_json::Value>>>,

    /// Primary skill for this construction (alternative to required_skills).
    #[serde(default)]
    pub skill: Option<String>,

    /// Difficulty of the construction.
    #[serde(default)]
    pub difficulty: Option<i32>,

    /// Time required to complete the construction (e.g. "60 m", "5 m").
    #[serde(default)]
    pub time: Option<String>,

    /// Tools required as nested arrays.
    /// e.g. [[{"id": "HAMMER", "level": 2}], [{"id": "SAW_M", "level": 1}]]
    /// Can also be a flat array of tool objects.
    #[serde(default)]
    pub qualities: Option<serde_json::Value>,

    /// Tools required (alternative format).
    /// e.g. [[["oxy_torch", 4], ["welder", 20]]]
    #[serde(default)]
    pub tools: Option<Vec<Vec<serde_json::Value>>>,

    /// Item groups/recipes to use (for pre-defined tool/component sets).
    /// e.g. [["wood_removal_standard", 1]]
    #[serde(default)]
    pub using: Option<Vec<Vec<serde_json::Value>>>,

    /// Components required as nested arrays.
    /// e.g. [[["nail", 16, "LIST"]], [["2x4", 4]]]
    #[serde(default)]
    pub components: Option<Vec<Vec<serde_json::Value>>>,

    /// Items produced as byproducts.
    /// Can be a string (deconstruction result set), a map, or an array of byproduct objects.
    #[serde(default)]
    pub byproducts: Option<serde_json::Value>,

    /// Pre-construction terrain requirement.
    /// Can be a string or an object.
    #[serde(default)]
    pub pre_terrain: Option<serde_json::Value>,

    /// Pre-construction furniture requirement.
    /// Can be a string or an object.
    #[serde(default)]
    pub pre_furniture: Option<serde_json::Value>,

    /// Pre-construction flags (any of these flags must be present on the tile).
    /// Can be a string, an array of strings, or a map with flag options.
    #[serde(default)]
    pub pre_flags: Option<serde_json::Value>,

    /// Pre-construction special check (e.g. "check_empty").
    /// Can be a string or an array of strings.
    #[serde(default)]
    pub pre_special: Option<serde_json::Value>,

    /// Post-construction terrain to place.
    /// Can be a string or an object.
    #[serde(default)]
    pub post_terrain: Option<serde_json::Value>,

    /// Post-construction furniture to place.
    /// Can be a string or an object.
    #[serde(default)]
    pub post_furniture: Option<serde_json::Value>,

    /// Post-construction flags to apply to the tile.
    /// Can be a string or an array of strings.
    #[serde(default)]
    pub post_flags: Option<serde_json::Value>,

    /// Post-construction special action (e.g. "done_dig_grave").
    /// Can be a string or an object.
    #[serde(default)]
    pub post_special: Option<serde_json::Value>,

    /// Special action to perform each turn during construction.
    /// Can be a string or an object.
    #[serde(default)]
    pub do_turn_special: Option<serde_json::Value>,

    /// Activity level (e.g. "EXTRA_EXERCISE", "MODERATE_EXERCISE", "BRISK_EXERCISE").
    #[serde(default)]
    pub activity_level: Option<String>,

    /// Whether this can be crafted in the dark.
    #[serde(default)]
    pub dark_craftable: Option<bool>,

    /// Whether this construction is displayed in the menu.
    #[serde(default)]
    pub on_display: Option<bool>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A byproduct produced by a construction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConstructionByproduct {
    /// Item ID produced.
    #[serde(default)]
    pub item: Option<String>,

    /// Item group ID to draw from.
    #[serde(default)]
    pub group: Option<String>,

    /// Number of items produced (single value or [min, max] range).
    #[serde(default)]
    pub count: Option<serde_json::Value>,

    /// Number of charges produced.
    #[serde(default)]
    pub charges: Option<serde_json::Value>,
}
