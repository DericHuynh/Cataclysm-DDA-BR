use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A vehicle prototype definition from JSON type `"vehicle"`.
///
/// Defines a vehicle blueprint with its layout, parts, items, and fuel.
/// Vehicles can inherit from abstract prototypes via `copy-from`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehicleDef {
    /// Unique identifier (e.g. "4x4_car", "beetle", "hearse").
    #[serde(default)]
    pub id: Option<DefId<VehicleDef>>,

    /// Display name of the vehicle.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Blueprint grid representing the vehicle layout (array of strings or array of arrays).
    #[serde(default)]
    pub blueprint: Option<serde_json::Value>,

    /// List of parts at each grid position.
    #[serde(default)]
    pub parts: Option<Vec<VehicleBlueprintPart>>,

    /// Items that may spawn inside the vehicle.
    /// Can be a sequence of item spawn objects or a single item group string.
    #[serde(default)]
    pub items: Option<serde_json::Value>,

    /// Fuel type and amount.
    #[serde(default)]
    pub fuel: Option<Vec<serde_json::Value>>,

    /// Reference to another vehicle definition to copy from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// If true, this is an abstract definition that should not appear in the final registry.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Extended fields (merge operations for copy-from).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,

    /// Deleted fields (merge operations for copy-from).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<serde_json::Value>,
}

/// A single part placement in a vehicle blueprint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehicleBlueprintPart {
    /// X coordinate in the blueprint grid.
    pub x: i32,

    /// Y coordinate in the blueprint grid.
    pub y: i32,

    /// List of part IDs or part objects at this position.
    #[serde(default)]
    pub parts: Option<Vec<serde_json::Value>>,
}

/// Item spawn definition within a vehicle.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehicleItemSpawn {
    /// X coordinate in the blueprint grid.
    pub x: i32,

    /// Y coordinate in the blueprint grid.
    pub y: i32,

    /// Chance of the items spawning (0-100).
    #[serde(default)]
    pub chance: Option<u32>,

    /// Specific items to spawn.
    #[serde(default)]
    pub items: Option<Vec<String>>,

    /// Item groups to spawn from.
    #[serde(default)]
    pub item_groups: Option<Vec<String>>,
}
