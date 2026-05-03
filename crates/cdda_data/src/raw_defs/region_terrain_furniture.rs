use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A region terrain furniture definition from JSON type `"region_terrain_furniture"`.
///
/// Defines how regional terrain and furniture types are resolved to concrete
/// tiles within a region's map settings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegionTerrainFurnitureDef {
    /// Unique identifier (e.g. "default_t_region_groundcover").
    pub id: DefId<RegionTerrainFurnitureDef>,

    /// Terrain configuration.
    #[serde(default)]
    pub terrain: Option<serde_json::Value>,

    /// Furniture configuration.
    #[serde(default)]
    pub furniture: Option<serde_json::Value>,
}
