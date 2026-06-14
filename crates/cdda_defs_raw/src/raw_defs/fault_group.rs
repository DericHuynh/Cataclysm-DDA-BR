use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A fault group definition from JSON type `"fault_group"`.
///
/// Defines a group of faults that can be applied to items, with weighted probabilities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FaultGroupDef {
    /// Unique identifier (e.g. "plate_lc", "blade_general").
    pub id: DefId<FaultGroupDef>,

    /// List of faults with their weights.
    pub group: Vec<serde_json::Value>,
}
