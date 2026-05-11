use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A profession definition from JSON type `"profession"`.
///
/// Defines a character profession (background/class) that the player can choose
/// at character creation. Professions determine starting skills, items, traits,
/// spells, and other character attributes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfessionDef {
    /// Unique identifier (e.g. "some_prof").
    pub id: DefId<ProfessionDef>,

    /// Display name (can be localized).
    pub name: LocalizedString,

    /// Description text (can be localized).
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Point cost of the profession (positive = costs points, negative = grants points).
    pub points: i32,

    /// Starting skills with levels (e.g. `[{"level": 2, "name": "mechanics"}]`).
    #[serde(default)]
    pub skills: Option<Vec<serde_json::Value>>,

    /// Starting items organized by category (e.g. `{"both": {"entries": [{"item": "tshirt"}]}}`).
    #[serde(default)]
    pub items: Option<serde_json::Value>,

    /// Starting spells with levels.
    #[serde(default)]
    pub spells: Option<Vec<serde_json::Value>>,

    /// Starting proficiencies.
    #[serde(default)]
    pub proficiencies: Option<Vec<String>>,

    /// Starting traits (mutation IDs).
    #[serde(default)]
    pub traits: Option<Vec<String>>,

    /// Starting missions.
    #[serde(default)]
    pub missions: Option<Vec<String>>,

    /// Special flags for this profession.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Starting addictions.
    #[serde(default)]
    pub addictions: Option<Vec<serde_json::Value>>,

    /// Whether the profession is pinned to the top of the selection list.
    #[serde(default)]
    pub pinned: Option<serde_json::Value>,

    /// NPC background reference ID.
    #[serde(default)]
    pub npc_background: Option<String>,
}
