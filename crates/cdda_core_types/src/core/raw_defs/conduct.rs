use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A conduct definition from JSON type `"conduct"`.
///
/// Defines a challenge conduct (e.g. "Pacifist", "Vegan") that tracks
/// whether the player has violated certain conditions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConductDef {
    /// Unique identifier (e.g. "conduct_no_smash", "conduct_zero_kills").
    pub id: DefId<ConductDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// List of requirements that must be satisfied.
    #[serde(default)]
    pub requirements: Vec<serde_json::Value>,

    /// List of other conduct IDs that hide this one when satisfied.
    #[serde(default)]
    pub hidden_by: Vec<String>,
}
