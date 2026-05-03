use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A vehicle spawn definition from JSON type `"vehicle_spawn"`.
///
/// Defines how vehicles can spawn in a given location.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehicleSpawnDef {
    /// Unique identifier (e.g. "default_bridge", "default_parkinglot").
    pub id: DefId<VehicleSpawnDef>,

    /// List of spawn types with weights and configurations.
    #[serde(default)]
    pub spawn_types: Vec<serde_json::Value>,
}
