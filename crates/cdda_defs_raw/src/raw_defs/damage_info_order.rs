use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A damage info order definition from JSON type `"damage_info_order"`.
///
/// Defines how damage type information is displayed in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DamageInfoOrderDef {
    /// Unique identifier matching a damage type (e.g. "bash", "cut").
    pub id: DefId<DamageInfoOrderDef>,

    /// Display mode for info ("detailed", "basic", or "none").
    #[serde(default)]
    pub info_display: Option<String>,

    /// Verb describing the damage.
    #[serde(default)]
    pub verb: Option<String>,

    /// Bionic info display settings.
    #[serde(default)]
    pub bionic_info: Option<serde_json::Value>,

    /// Protection info display settings.
    #[serde(default)]
    pub protection_info: Option<serde_json::Value>,

    /// Pet protection info display settings.
    #[serde(default)]
    pub pet_prot_info: Option<serde_json::Value>,

    /// Melee combat info display settings.
    #[serde(default)]
    pub melee_combat_info: Option<serde_json::Value>,

    /// Ablative info display settings.
    #[serde(default)]
    pub ablative_info: Option<serde_json::Value>,
}
