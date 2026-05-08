use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An uncraft definition from JSON type `"uncraft"`.
///
/// Defines how to reverse-craft an item into its components.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UncraftDef {
    /// The item result identifier (used as the key, like in RecipeDef).
    /// Can be missing for abstract templates that use "abstract" instead.
    #[serde(default)]
    pub result: Option<String>,

    /// Activity level required.
    #[serde(default)]
    pub activity_level: Option<String>,

    /// Time required to uncraft.
    #[serde(default)]
    pub time: Option<String>,

    /// Tool qualities required (CDDA nested format: `[[{"id": "CUT", "level": 2}]]`).
    #[serde(default)]
    pub qualities: Option<serde_json::Value>,

    /// Components produced (triple-nested array format like `[[["item", count]]]`).
    #[serde(default)]
    pub components: Option<serde_json::Value>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Tools required.
    #[serde(default)]
    pub tools: Option<Vec<Vec<serde_json::Value>>>,

    /// Using a specific skill for the uncraft.
    #[serde(default)]
    pub skill_used: Option<String>,

    /// Difficulty of the uncraft.
    #[serde(default)]
    pub difficulty: Option<i32>,
}

/// A quality requirement (e.g. `{"id": "CUT_FINE", "level": 1}`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QualReq {
    /// Quality type ID.
    pub id: String,
    /// Quality level required.
    pub level: u32,
}
