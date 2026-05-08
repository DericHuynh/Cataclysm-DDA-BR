use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A map extra collection definition from JSON type `"map_extra_collection"`.
///
/// Defines a collection of map extras that can spawn in a given biome/region.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapExtraCollectionDef {
    /// Unique identifier (e.g. "forest", "field", "road").
    pub id: DefId<MapExtraCollectionDef>,

    /// Chance (out of 100) for this collection to spawn extras.
    #[serde(default)]
    pub chance: Option<i32>,

    /// List of map extra entries with weights.
    #[serde(default)]
    pub extras: Vec<serde_json::Value>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Fields to extend from the base definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,
}
