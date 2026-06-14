use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A sub-body-part definition from JSON type `"sub_body_part"`.
///
/// Defines a sub-division of a body part (e.g. "left hand", "right arm").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubBodyPartDef {
    /// Unique identifier (e.g. "sub_part_hand_l").
    pub id: DefId<SubBodyPartDef>,

    /// Parent body part ID (e.g. "hand_l"). Can be string or other identifier.
    #[serde(default)]
    pub parent: Option<serde_json::Value>,

    /// Side (e.g. "left", "right").
    #[serde(default)]
    pub side: Option<serde_json::Value>,

    /// Opposite sub-body-part ID (e.g. "sub_part_hand_r").
    #[serde(default)]
    pub opposite: Option<serde_json::Value>,

    /// Display name (e.g. "left hand"). Can be missing for copy-from.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Maximum coverage percentage (0-100).
    #[serde(default)]
    pub max_coverage: Option<u32>,
}
