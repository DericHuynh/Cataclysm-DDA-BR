use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A monster faction definition from JSON type `"MONSTER_FACTION"`.
///
/// Defines a faction for monsters, controlling inter-monster relationships
/// (friendly, neutral, hate, by_mood).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterFactionDef {
    /// Display name (e.g. "zombie", "human", "player", "animal").
    pub name: String,

    /// Base faction this one derives from.
    #[serde(default)]
    pub base_faction: Option<String>,

    /// Factions that are friendly — they will not attack each other.
    #[serde(default)]
    pub friendly: Option<Vec<String>>,

    /// Factions that are neutral.
    #[serde(default)]
    pub neutral: Option<Vec<String>>,

    /// Factions that are hated — they will be attacked on sight.
    #[serde(default)]
    pub hate: Option<Vec<String>>,

    /// Factions to which the attitude is determined by mood (can be a string or array).
    #[serde(default)]
    pub by_mood: Option<serde_json::Value>,
}
