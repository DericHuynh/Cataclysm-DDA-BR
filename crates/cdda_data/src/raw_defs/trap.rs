use crate::raw_defs::cdda_types::RawValue;
use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A trap definition from JSON type `"trap"`.
///
/// Defines a trap that can be placed on the map (e.g. "tr_beartrap", "tr_net", "tr_pit").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrapDef {
    /// Unique identifier (e.g. "tr_beartrap", "tr_pit").
    pub id: DefId<TrapDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text (many CDDA traps omit this).
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// ASCII symbol.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color.
    #[serde(default)]
    pub color: Option<String>,

    /// Action performed by this trap.
    #[serde(default)]
    pub action: Option<String>,

    /// Visibility for spotting the trap (can be -1 for always invisible).
    #[serde(default)]
    pub visibility: Option<i32>,

    /// Avoidance difficulty.
    #[serde(default)]
    pub avoidance: Option<i32>,

    /// Difficulty to disarm (can be negative).
    #[serde(default)]
    pub difficulty: Option<i32>,

    /// Whether the trap is triggerable.
    #[serde(default)]
    pub trigger: Option<String>,

    /// Damage dealt by the trap (map of damage_type -> amount).
    #[serde(default)]
    pub damage: Option<HashMap<String, RawValue>>,

    /// Sound made when triggered.
    #[serde(default)]
    pub sound: Option<String>,

    /// Volume of the trigger sound (can be a map value in some definitions).
    #[serde(default)]
    pub sound_volume: Option<u32>,

    /// Message shown when triggered.
    #[serde(default)]
    pub trigger_message: Option<String>,

    /// Message shown when disarmed.
    #[serde(default)]
    pub disarm_message: Option<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Catch-all for any unrecognized fields.
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Vehicle data.
    #[serde(default)]
    pub vehicle_data: Option<crate::raw_defs::cdda_types::TrapVehicleData>,

    /// Item drops.
    #[serde(default)]
    pub drops: Option<serde_json::Value>,

    /// Spell data.
    #[serde(default)]
    pub spell_data: Option<crate::raw_defs::cdda_types::TrapSpellData>,

    /// Benign trap.
    #[serde(default)]
    pub benign: Option<bool>,

    /// EOC effects.
    #[serde(default)]
    pub eocs: Option<Vec<String>>,

    /// Memorial log (female). Can be string or structured object.
    #[serde(default)]
    pub memorial_female: Option<serde_json::Value>,

    /// Memorial log (male). Can be string or structured object.
    #[serde(default)]
    pub memorial_male: Option<serde_json::Value>,

    /// Trap radius.
    #[serde(default)]
    pub trap_radius: Option<i32>,

    /// Always invisible.
    #[serde(default)]
    pub always_invisible: Option<bool>,

    /// copy-from parent (allows trap inheritance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

fn default_symbol() -> String {
    "^".to_string()
}
