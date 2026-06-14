use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A harvest definition from JSON type `"harvest"`.
///
/// Defines what drops (items, mutagens, etc.) result from harvesting a corpse or
/// dissecting a creature. Used for butchery and dissection results.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HarvestDef {
    /// Unique identifier (e.g. "dissect_alpha_sample_single").
    pub id: DefId<HarvestDef>,

    /// Message displayed when harvesting.
    #[serde(default)]
    pub message: Option<String>,

    /// List of harvest entries describing possible drops.
    #[serde(default)]
    pub entries: Vec<HarvestEntry>,

    /// Group ID for drop distribution (alternative to inline entries).
    #[serde(default)]
    pub group: Option<String>,

    /// Body part required to be available for this harvest (e.g. "head", "torso").
    #[serde(default)]
    pub source_required_body_part: Option<String>,
}

/// A single entry in a harvest definition describing what drops and under what conditions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HarvestEntry {
    /// Item or group ID to drop.
    pub drop: String,

    /// Type of drop (e.g. "mutagen_group", "flesh", "bone", "skin", "offal", "bionic").
    #[serde(default)]
    pub r#type: Option<String>,

    /// Proportion of the target's mass that this drop represents (0.0 to 1.0).
    #[serde(default)]
    pub mass_ratio: Option<f64>,

    /// Base number of drops (can be [min, max] range or fractional ratio).
    #[serde(default)]
    pub base_num: Option<Vec<serde_json::Value>>,

    /// Scale number of drops based on size/mass (can be [min, max] range or fractional ratio).
    #[serde(default)]
    pub scale_num: Option<Vec<serde_json::Value>>,

    /// Maximum number of this drop that can be obtained.
    #[serde(default)]
    pub max: Option<u32>,

    /// Flags modifying drop behavior (e.g. "NO_STERILE", "NO_ROT").
    #[serde(default)]
    pub flags: Option<Vec<String>>,
}
