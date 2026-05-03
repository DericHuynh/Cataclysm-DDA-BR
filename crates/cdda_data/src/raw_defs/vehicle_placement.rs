use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A vehicle placement definition from JSON type `"vehicle_placement"`.
///
/// Defines where vehicles can be placed during map generation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePlacementDef {
    /// Unique identifier (e.g. "highway", "subway_deadend").
    pub id: DefId<VehiclePlacementDef>,

    /// List of location configurations.
    #[serde(default)]
    pub locations: Vec<serde_json::Value>,
}
