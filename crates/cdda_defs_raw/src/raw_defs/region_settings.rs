use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A region settings definition from JSON type `"region_settings"`.
///
/// Defines regional map generation settings for a game region.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegionSettingsDef {
    /// Unique identifier (e.g. "default").
    pub id: DefId<RegionSettingsDef>,

    /// River settings reference.
    #[serde(default)]
    pub rivers: Option<String>,

    /// Lake settings reference.
    #[serde(default)]
    pub lakes: Option<String>,

    /// Ocean settings reference.
    #[serde(default)]
    pub ocean: Option<String>,

    /// Ravine settings reference.
    #[serde(default)]
    pub ravines: Option<String>,

    /// Forest settings reference.
    #[serde(default)]
    pub forests: Option<String>,

    /// Forest composition settings reference.
    #[serde(default)]
    pub forest_composition: Option<String>,

    /// Forest trail settings reference.
    #[serde(default)]
    pub forest_trails: Option<String>,

    /// Highway settings reference.
    #[serde(default)]
    pub highways: Option<String>,

    /// City settings reference.
    #[serde(default)]
    pub cities: Option<String>,

    /// Map extras settings reference.
    #[serde(default)]
    pub map_extras: Option<String>,

    /// Terrain furniture settings reference.
    #[serde(default)]
    pub terrain_furniture: Option<String>,

    /// Weather settings reference.
    #[serde(default)]
    pub weather: Option<String>,

    /// Urbanity increase curve.
    #[serde(default)]
    pub urbanity_increase: Option<serde_json::Value>,

    /// Default overmap terrain IDs for each Z-level.
    #[serde(default)]
    pub default_oter: Vec<String>,

    /// Default ground cover.
    #[serde(default)]
    pub default_groundcover: Vec<serde_json::Value>,

    /// Feature flag settings.
    #[serde(default)]
    pub feature_flag_settings: Option<serde_json::Value>,

    /// Connection types for roads, sewers, etc.
    #[serde(default)]
    pub connections: Option<serde_json::Value>,
}
