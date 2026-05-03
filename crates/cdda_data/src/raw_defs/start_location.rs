use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A start location definition from JSON type `"start_location"`.
///
/// Defines where a character can start the game (e.g. "shelter", "evac_center", "lab").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartLocationDef {
    /// Unique identifier.
    pub id: DefId<StartLocationDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Target terrain for the start location (can be string, array, or object).
    #[serde(default, rename = "terrain")]
    pub target_terrain: Option<serde_json::Value>,

    /// List of specific OMT IDs where this start can occur.
    #[serde(default)]
    pub flags: Vec<String>,

    /// List of starting locations that are excluded.
    #[serde(default)]
    pub exclude: Option<Vec<String>>,

    /// Whether this start location contains a shelter.
    #[serde(default)]
    pub shelter: Option<bool>,
}
