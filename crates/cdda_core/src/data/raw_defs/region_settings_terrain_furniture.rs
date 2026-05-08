use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A region settings terrain/furniture definition from JSON type `"region_settings_terrain_furniture"`.
///
/// Defines terrain and furniture overrides for a region.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegionSettingsTerrainFurnitureDef {
    /// Unique identifier (e.g. "default", "highland").
    pub id: DefId<RegionSettingsTerrainFurnitureDef>,

    /// List of terrain/furniture replacement mappings.
    pub ter_furn: Vec<String>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
