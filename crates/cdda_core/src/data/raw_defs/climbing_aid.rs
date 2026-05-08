use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A climbing aid definition from JSON type `"climbing_aid"`.
///
/// Defines how a tile, item, or ability can be used for climbing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClimbingAidDef {
    /// Unique identifier (e.g. "vehicle", "furn_CLIMBABLE", "ability_WALL_CLING").
    pub id: DefId<ClimbingAidDef>,

    /// Modifier to slip chance (negative means safer).
    #[serde(default)]
    pub slip_chance_mod: Option<i32>,

    /// Configuration for climbing down.
    #[serde(default)]
    pub down: Option<serde_json::Value>,

    /// Condition under which this climbing aid can be used.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
