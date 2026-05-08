use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A mission definition from JSON type `"mission_definition"`.
///
/// Defines a quest/mission that can be given to the player by NPCs or
/// started automatically. Missions have goals, difficulty, rewards, and
/// start/end effects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MissionDefinitionDef {
    /// Unique identifier (e.g. "MISSION_ASSASSINATION", "MISSION_PATIENT").
    pub id: DefId<MissionDefinitionDef>,

    /// Display name of the mission.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description of the mission.
    #[serde(default)]
    pub description: Option<String>,

    /// Goal type (e.g. "MGOAL_KILL_MONSTER", "MGOAL_GO_TO", "MGOAL_CONDITION").
    pub goal: String,

    /// Difficulty rating (numeric, map, or array).
    #[serde(default)]
    pub difficulty: Option<serde_json::Value>,

    /// Monetary value / reward (numeric, map, or array).
    #[serde(default)]
    pub value: Option<serde_json::Value>,

    /// Deadline (if timed) - can be a number, string, array, or math expression.
    #[serde(default)]
    pub deadline: Option<serde_json::Value>,

    /// Start conditions / effects that trigger when the mission is accepted.
    #[serde(default)]
    pub start: Option<serde_json::Value>,

    /// End conditions / effects that trigger when the mission is completed.
    #[serde(default)]
    pub end: Option<serde_json::Value>,

    /// Fail conditions / effects that trigger when the mission is failed.
    #[serde(default)]
    pub fail: Option<serde_json::Value>,

    /// NPC origins that can give this mission (e.g. ["ORIGIN_GAME_START", "ORIGIN_SECONDARY"]).
    #[serde(default)]
    pub origins: Option<Vec<String>>,

    /// NPC dialogue that starts this mission.
    #[serde(default)]
    pub dialogue: Option<serde_json::Value>,

    /// Whether the mission is repeatable.
    #[serde(default)]
    pub repeatable: Option<bool>,

    /// Whether the mission should be hidden from the player.
    #[serde(default)]
    pub hidden: Option<bool>,

    /// Whether an NPC follows the player for this mission.
    #[serde(default)]
    pub follows: Option<bool>,

    /// Whether the mission is urgent.
    #[serde(default)]
    pub urgent: Option<bool>,

    /// ID of the NPC that starts this mission.
    #[serde(default)]
    pub start_npc: Option<String>,

    /// Condition for the goal to be achieved.
    #[serde(default)]
    pub goal_condition: Option<serde_json::Value>,

    /// Items to remove from the player when the mission is completed.
    #[serde(default)]
    pub remove: Option<serde_json::Value>,

    /// Whether the mission is selected automatically upon accepting.
    #[serde(default)]
    pub has_generic_rewards: Option<bool>,
}
