use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A forest biome component definition from JSON type `"forest_biome_component"`.
///
/// Defines a component of a forest biome (trees, shrubs, clutter, water, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForestBiomeComponentDef {
    /// Unique identifier (e.g. "trees_forest", "shrubs_and_flowers_forest").
    pub id: DefId<ForestBiomeComponentDef>,

    /// Sequence order for placement.
    #[serde(default)]
    pub sequence: Option<i32>,

    /// Chance of this component appearing.
    #[serde(default)]
    pub chance: Option<i32>,

    /// Terrain/furniture types with weights.
    #[serde(default)]
    pub types: Vec<serde_json::Value>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
