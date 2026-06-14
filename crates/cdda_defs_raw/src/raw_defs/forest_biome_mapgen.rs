use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A forest biome mapgen definition from JSON type `"forest_biome_mapgen"`.
///
/// Defines how a forest biome is generated, including terrain, items, and components.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForestBiomeMapgenDef {
    /// Unique identifier (e.g. "biome_forest_default", "biome_forest_thick_default").
    pub id: DefId<ForestBiomeMapgenDef>,

    /// List of overmap terrains this biome applies to.
    #[serde(default)]
    pub terrains: Vec<String>,

    /// Sparseness adjacency factor.
    #[serde(default)]
    pub sparseness_adjacency_factor: Option<i32>,

    /// Item group for spawning items.
    #[serde(default)]
    pub item_group: Option<String>,

    /// Chance of item group spawning.
    #[serde(default)]
    pub item_group_chance: Option<i32>,

    /// Number of item spawn iterations.
    #[serde(default)]
    pub item_spawn_iterations: Option<i32>,

    /// Ground cover terrain with weights.
    #[serde(default)]
    pub groundcover: Vec<serde_json::Value>,

    /// List of biome component IDs.
    #[serde(default)]
    pub components: Vec<String>,

    /// Terrain furniture mapping.
    #[serde(default)]
    pub terrain_furniture: Option<serde_json::Value>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Fields to extend from the base definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,
}
