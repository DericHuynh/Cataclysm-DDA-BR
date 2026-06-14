use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A fault fix definition from JSON type `"fault_fix"`.
///
/// Defines a repair method for fixing one or more faults on items
/// (e.g. welding holes, patching dents, replacing parts).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FaultFixDef {
    /// Unique identifier (e.g. "mend_armor_soft_planishing").
    pub id: DefId<FaultFixDef>,

    /// Display name of the fix.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Skills required for the fix.
    #[serde(default)]
    pub skills: Option<serde_json::Value>,

    /// Time required to perform the fix.
    #[serde(default)]
    pub time: Option<String>,

    /// Tools required for the fix.
    #[serde(default)]
    pub tools: Option<serde_json::Value>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
