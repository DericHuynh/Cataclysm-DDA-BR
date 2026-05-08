use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A faction mission definition from JSON type `"faction_mission"`.
///
/// Defines a mission that can be assigned to companions at a faction camp.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionMissionDef {
    /// Unique identifier (e.g. "camp_gathering", "camp_hunting").
    pub id: DefId<FactionMissionDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description of the mission.
    #[serde(default)]
    pub desc: Option<serde_json::Value>,

    /// Skill used for the mission.
    #[serde(default)]
    pub skill: Option<String>,

    /// Label for items produced.
    #[serde(default)]
    pub items_label: Option<String>,

    /// Possible items that can be produced.
    #[serde(default)]
    pub items_possibilities: Vec<String>,

    /// Risk level.
    #[serde(default)]
    pub risk: Option<String>,

    /// Difficulty level.
    #[serde(default)]
    pub difficulty: Option<String>,

    /// Activity level.
    #[serde(default)]
    pub activity: Option<String>,

    /// Time estimate.
    #[serde(default)]
    pub time: Option<String>,

    /// Number of positions available.
    #[serde(default)]
    pub positions: Option<i32>,

    /// List of effects.
    #[serde(default)]
    pub effects: Vec<serde_json::Value>,

    /// Footer text.
    #[serde(default)]
    pub footer: Option<String>,
}
