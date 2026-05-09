use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A fault definition from JSON type `"fault"`.
///
/// Defines a fault or defect that can affect items (e.g. dented, punctured,
/// blown fuse). Faults can modify item properties and be repaired by fault fixes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FaultDef {
    /// Unique identifier (e.g. "fault_armor_lc_dented").
    pub id: DefId<FaultDef>,

    /// Display name of the fault.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description of the fault.
    #[serde(default)]
    pub description: Option<String>,

    /// Flags associated with the fault.
    #[serde(default)]
    pub flags: Option<Vec<String>>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Abstract fault — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,
}
