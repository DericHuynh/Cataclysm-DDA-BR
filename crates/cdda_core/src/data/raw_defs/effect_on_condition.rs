use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An effect_on_condition definition from JSON type `"effect_on_condition"`.
///
/// EOCs are scripts that run when certain conditions are met.
/// They can be used for ambient effects, quest triggers, timed events, and more.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EffectOnConditionDef {
    /// Unique identifier (e.g. "EOC_SLEEP", "EOC_RandEnc").
    pub id: DefId<EffectOnConditionDef>,

    /// Type of EOC: "ACTIVATION", "EVENT", "AVATAR_DEATH", "RECURRING",
    /// "PREVENT_DEATH", "NPC_DEATH", etc.
    #[serde(default)]
    pub eoc_type: Option<String>,

    /// The effect(s) to execute when triggered.
    /// Can be a single effect object or an array of effect objects.
    #[serde(default)]
    pub effect: Option<serde_json::Value>,

    /// Condition that must be true for the effect to run.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Effect(s) to run when the condition is false.
    #[serde(default)]
    pub false_effect: Option<serde_json::Value>,

    /// Condition that deactivates this EOC.
    #[serde(default)]
    pub deactivate_condition: Option<serde_json::Value>,

    /// Recurrence interval — can be a number (turns), a time string ("1 hours"),
    /// a range ["1 hours", "2 hours"], or a complex object.
    #[serde(default)]
    pub recurrence: Option<serde_json::Value>,

    /// Whether this EOC runs globally (affects all NPCs).
    #[serde(default)]
    pub global: Option<bool>,

    /// Whether this EOC should run for NPCs.
    #[serde(default)]
    pub run_for_npcs: Option<bool>,

    /// List of NPC IDs this EOC applies to.
    #[serde(default)]
    pub npcs: Option<Vec<String>>,

    /// Event type required to trigger this EOC (for "EVENT" type).
    #[serde(default)]
    pub required_event: Option<String>,

    /// Queue type for the event.
    #[serde(default)]
    pub queue: Option<String>,

    /// Whether this EOC runs once.
    #[serde(default)]
    pub run_once: Option<bool>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
