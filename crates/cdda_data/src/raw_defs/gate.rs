use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A gate definition from JSON type `"gate"`.
///
/// Defines a gate or door that can be opened/closed via a control mechanism.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateDef {
    /// Unique identifier (e.g. "t_gates_mech_control", "t_barndoor").
    pub id: DefId<GateDef>,

    /// The door terrain placed when the gate is closed.
    pub door: Option<String>,

    /// The floor terrain placed when the gate is open.
    pub floor: Option<String>,

    /// List of wall terrains that this gate is compatible with.
    /// An empty list matches any wall.
    #[serde(default)]
    pub walls: Vec<String>,

    /// Messages displayed when interacting with the gate.
    #[serde(default)]
    pub messages: Option<serde_json::Value>,

    /// Movement cost to operate the gate.
    #[serde(default)]
    pub moves: Option<i32>,

    /// Bashing damage dealt by the gate.
    #[serde(default)]
    pub bashing_damage: Option<i32>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
