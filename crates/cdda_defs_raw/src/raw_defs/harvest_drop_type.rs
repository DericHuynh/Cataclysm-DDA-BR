use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A harvest drop type definition from JSON type `"harvest_drop_type"`.
///
/// Defines a type of harvest drop (e.g. "flesh", "bone", "skin").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HarvestDropTypeDef {
    /// Unique identifier (e.g. "flesh", "bone", "skin").
    pub id: DefId<HarvestDropTypeDef>,

    /// Message displayed when dissection fails.
    #[serde(default)]
    pub msg_dissect_fail: Option<String>,

    /// Whether this drop type is only available through dissection.
    #[serde(default)]
    pub dissect_only: Option<bool>,

    /// Skills required for harvesting.
    #[serde(default)]
    pub harvest_skills: Vec<String>,

    /// Message displayed when field dressing fails.
    #[serde(default)]
    pub msg_fielddress_fail: Option<String>,

    /// Message displayed when butchering fails.
    #[serde(default)]
    pub msg_butcher_fail: Option<String>,

    /// Whether this is a group type.
    #[serde(default)]
    pub group: Option<bool>,
}
