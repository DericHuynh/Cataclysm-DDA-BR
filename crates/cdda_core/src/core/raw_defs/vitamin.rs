use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A vitamin definition from JSON type `"vitamin"`.
///
/// Defines a vitamin/nutrient tracked by the game (e.g. vitamin C, calcium, iron).
/// Vitamins have minimum and maximum levels, and can cause diseases when deficient
/// or in excess.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VitaminDef {
    /// Unique identifier (e.g. "vitC", "calcium", "iron").
    pub id: DefId<VitaminDef>,

    /// Display name (can be localized).
    pub name: LocalizedString,

    /// Type of vitamin (e.g. "vitamin", "toxin", "drug", "fatigue").
    #[serde(default)]
    pub vit_type: Option<String>,

    /// Minimum deficiency threshold (below this, disease triggers).
    #[serde(default)]
    pub min: Option<i32>,

    /// Maximum healthy level (above this, disease_excess triggers).
    #[serde(default)]
    pub max: Option<i32>,

    /// Disease applied when below the minimum threshold: `["disease_id", intensity]`.
    #[serde(default)]
    pub disease: Option<Vec<serde_json::Value>>,

    /// Disease applied when above the maximum threshold: `["disease_id", intensity]`.
    #[serde(default)]
    pub disease_excess: Option<Vec<serde_json::Value>>,

    /// Flags for this vitamin.
    #[serde(default)]
    pub flags: Vec<String>,
}
