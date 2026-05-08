use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A city building definition from JSON type `"city_building"`.
///
/// Defines a building that can be placed in a city during overmap generation,
/// specifying its location type and overmap tiles.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CityBuildingDef {
    /// Unique identifier (e.g. "2storyModern01", "bungalow01", "magic_shop").
    pub id: DefId<CityBuildingDef>,

    /// Location types where this building can be placed (e.g. ["land"]).
    #[serde(default)]
    pub locations: Option<Vec<String>>,

    /// Overmap tile placements for this building.
    #[serde(default)]
    pub overmaps: Option<Vec<CityBuildingOvermap>>,

    /// Flags for this city building (e.g. "GLOBALLY_UNIQUE", "SAFE_AT_WORLDGEN").
    #[serde(default)]
    pub flags: Option<Vec<String>>,
}

/// A single overmap tile placement within a city building.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CityBuildingOvermap {
    /// Grid position [x, y, z] relative to the building's origin.
    pub point: Vec<i32>,

    /// Overmap terrain ID to place at this position (e.g. "bungalow01_1_north").
    pub overmap: String,
}
