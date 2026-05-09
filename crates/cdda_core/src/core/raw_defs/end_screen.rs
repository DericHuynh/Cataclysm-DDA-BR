use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An end screen definition from JSON type `"end_screen"`.
///
/// Defines a screen displayed when the player dies, with ASCII art and stats.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EndScreenDef {
    /// Unique identifier (e.g. "death_tombstone", "mycus_death").
    pub id: DefId<EndScreenDef>,

    /// Priority for selecting which end screen to show.
    #[serde(default)]
    pub priority: Option<i32>,

    /// ID of the ASCII art picture to display.
    #[serde(default)]
    pub picture_id: Option<String>,

    /// Condition under which this end screen is shown.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Additional info sections (lines of text with positions).
    #[serde(default)]
    pub added_info: Vec<serde_json::Value>,

    /// Label for the "last words" section.
    #[serde(default)]
    pub last_words_label: Option<String>,
}
