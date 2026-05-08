use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A body part definition from JSON type `"body_part"`.
///
/// Defines a body part (e.g. "head", "torso", "arm_l", "leg_l").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyPartDef {
    /// Unique identifier (e.g. "head").
    pub id: DefId<BodyPartDef>,

    /// Display name (can be a plain string, structured object, or missing for copy-from).
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Accusative form (can be a string, localized object, or missing).
    #[serde(default)]
    pub accusative: Option<serde_json::Value>,

    /// Heading (e.g. "Head").
    #[serde(default)]
    pub heading: Option<serde_json::Value>,

    /// Heading multiple form (e.g. "Head").
    #[serde(default)]
    pub heading_multiple: Option<serde_json::Value>,

    /// HP bar UI text (e.g. "HEAD").
    #[serde(default)]
    pub hp_bar_ui_text: Option<String>,

    /// Main (parent) body part ID.
    #[serde(default)]
    pub main_part: Option<String>,

    /// Connected body part ID.
    #[serde(default)]
    pub connected_to: Option<String>,

    /// Opposite body part ID.
    #[serde(default)]
    pub opposite_part: Option<String>,

    /// Limb type (e.g. "head", "arm", "leg", "torso").
    #[serde(default)]
    pub limb_type: Option<String>,

    /// Whether this is a vital body part.
    #[serde(default)]
    pub is_vital: Option<bool>,

    /// Hit size (determines probability of being hit). Can be integer or float.
    #[serde(default)]
    pub hit_size: Option<serde_json::Value>,

    /// Hit difficulty (harder to hit with called shots). Can be integer or float.
    #[serde(default)]
    pub hit_difficulty: Option<serde_json::Value>,

    /// Side ("left", "right", or neither).
    #[serde(default)]
    pub side: Option<String>,

    /// Base hit points.
    #[serde(default)]
    pub base_hp: Option<u32>,

    /// Bionic slots available.
    #[serde(default)]
    pub bionic_slots: Option<u32>,

    /// Smash message when this part is damaged.
    #[serde(default)]
    pub smash_message: Option<String>,

    /// Hot morale modifier.
    #[serde(default)]
    pub hot_morale_mod: Option<f64>,

    /// Cold morale modifier.
    #[serde(default)]
    pub cold_morale_mod: Option<f64>,

    /// Drench capacity.
    #[serde(default)]
    pub drench_capacity: Option<u32>,

    /// Sub-parts of this body part.
    #[serde(default)]
    pub sub_parts: Option<Vec<String>>,

    /// Legacy ID for backwards compatibility.
    #[serde(default)]
    pub legacy_id: Option<String>,

    /// Abstract flag.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Catch-all for any additional fields.
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}
