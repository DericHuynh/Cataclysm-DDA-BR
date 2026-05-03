use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A relic procgen data definition from JSON type `"relic_procgen_data"`.
///
/// Defines procedural generation parameters for relic artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelicProcgenDataDef {
    /// Unique identifier (e.g. "cult", "alien_reality").
    pub id: DefId<RelicProcgenDataDef>,

    /// Charge type configurations.
    #[serde(default)]
    pub charge_types: Vec<serde_json::Value>,

    /// Passive additive procgen values.
    #[serde(default)]
    pub passive_add_procgen_values: Vec<serde_json::Value>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
