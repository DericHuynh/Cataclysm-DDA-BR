use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A map extra definition from JSON type `"map_extra"`.
///
/// Defines a special feature that can appear on the overmap (e.g. craters,
/// crashed helicopters, roadblocks, portals).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapExtraDef {
    /// Unique identifier (e.g. "mx_crater", "mx_helicopter").
    pub id: DefId<MapExtraDef>,

    /// Display name of the map extra.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description of the map extra.
    #[serde(default)]
    pub description: Option<String>,

    /// Generator configuration for placing this map extra.
    #[serde(default)]
    pub generator: Option<serde_json::Value>,

    /// Mapgen palette used by this map extra.
    #[serde(default)]
    pub mapgen_palette: Option<String>,

    /// Flags associated with the map extra.
    #[serde(default)]
    pub flags: Option<Vec<String>>,
}
