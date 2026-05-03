use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A talk topic definition from JSON type `"talk_topic"`.
///
/// Defines a node in an NPC dialogue tree with a dynamic line and
/// a set of possible responses the player can choose from.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TalkTopicDef {
    /// Unique identifier (e.g. "TALK_HELLO", "TALK_DONE").
    /// Can be a string or an array of strings (aliases).
    #[serde(default)]
    pub id: Option<serde_json::Value>,

    /// The dialogue line spoken by the NPC.
    /// Can be a string, a structured object, or an array of alternatives.
    #[serde(default)]
    pub dynamic_line: Option<serde_json::Value>,

    /// List of possible player responses.
    #[serde(default)]
    pub responses: Option<Vec<TalkResponse>>,

    /// Whether this topic can repeat (boolean or object).
    #[serde(default)]
    pub repeat: Option<serde_json::Value>,

    /// Whether to replace the current topic instead of pushing to the history stack.
    #[serde(default)]
    pub replace_current: Option<bool>,

    /// Whether this topic is a dynamic response calculated from the previous.
    #[serde(default)]
    pub dynamic: Option<bool>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A response option within a talk topic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TalkResponse {
    /// Display text for this response option.
    #[serde(default)]
    pub text: Option<String>,

    /// The topic to transition to when this response is chosen.
    /// Can be a single topic string or an array of topic strings.
    /// May be absent for responses that only use `effect`.
    #[serde(default)]
    pub topic: Option<serde_json::Value>,

    /// Optional condition that controls whether this response is available.
    /// Can be a string (e.g. "npc_available") or a condition object.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Optional trial that determines if the response succeeds.
    #[serde(default)]
    pub trial: Option<TalkTrial>,

    /// Result on trial success (alternative topic/effect).
    #[serde(default)]
    pub success: Option<TalkResult>,

    /// Result on trial failure (alternative topic/effect).
    #[serde(default)]
    pub failure: Option<TalkResult>,

    /// Whether this response switches the current topic.
    #[serde(default)]
    pub switch: Option<bool>,

    /// Whether this response is a default option.
    #[serde(default)]
    pub default: Option<bool>,

    /// Whether this response is always available.
    #[serde(default)]
    pub always: Option<bool>,

    /// Explanation text shown when the response condition fails.
    #[serde(default)]
    pub failure_explanation: Option<String>,

    /// Topic to go to on condition failure (alternative to `failure.topic`).
    #[serde(default)]
    pub failure_topic: Option<String>,

    /// Effect(s) to apply when this response is chosen.
    /// Can be a single effect object or an array of effects.
    #[serde(default)]
    pub effect: Option<serde_json::Value>,

    /// Optional speech text for the NPC when this response is chosen.
    #[serde(default)]
    pub speech: Option<String>,

    /// Whether this response falls through to the next matching response.
    #[serde(default)]
    pub falls_through: Option<bool>,
}

/// A trial that determines whether a response succeeds or fails.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TalkTrial {
    /// Trial type (e.g. "CONDITION", "PERSUADE", "INTIMIDATE", "NONE").
    pub r#type: String,

    /// Condition for the trial (used with type "CONDITION").
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Difficulty of the trial.
    #[serde(default)]
    pub difficulty: Option<i32>,

    /// Skill used for the trial.
    #[serde(default)]
    pub skill: Option<String>,

    /// Skill tier used for the trial.
    #[serde(default)]
    pub skill_tier: Option<serde_json::Value>,

    /// Modifier applied to the trial.
    #[serde(default)]
    pub mod_: Option<serde_json::Value>,

    /// High/low range for the trial.
    #[serde(default)]
    pub range: Option<Vec<i32>>,

    /// Base cost of the trial.
    #[serde(default)]
    pub cost: Option<i32>,
}

/// Result of a trial success or failure.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TalkResult {
    /// Topic to transition to.
    #[serde(default)]
    pub topic: Option<String>,

    /// Effect(s) to apply.
    #[serde(default)]
    pub effect: Option<serde_json::Value>,

    /// Speech text for the NPC.
    #[serde(default)]
    pub speech: Option<String>,

    /// Trial to use instead.
    #[serde(default)]
    pub trial: Option<TalkTrial>,

    /// Text to display (alternative to dynamic_line).
    #[serde(default)]
    pub text: Option<serde_json::Value>,
}
