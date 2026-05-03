use crate::types::DefId;
use serde::{Deserialize, Serialize};

/// An effect type definition from JSON type `"effect_type"`.
///
/// Defines a status effect that can be applied to characters or monsters
/// (e.g. "stunned", "poisoned", "bleeding", "on_fire").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectDef {
    /// Unique identifier (e.g. "poisoned", "stunned", "bleeding").
    pub id: DefId<EffectDef>,

    /// Display name (each intensity level can have its own name).
    #[serde(default)]
    pub name: Option<Vec<EffectNamedValue>>,

    /// Description text per intensity level.
    #[serde(default)]
    pub desc: Option<Vec<EffectNamedValue>>,

    /// Whether to show in the character info.
    #[serde(default)]
    pub show_in_info: Option<bool>,

    /// Whether the effect has an icon.
    #[serde(default)]
    pub show_effect: Option<bool>,

    /// Whether the effect has a visual indicator.
    #[serde(default)]
    pub blood_analysis_description: Option<String>,

    /// Maximum intensity level.
    #[serde(default)]
    pub max_intensity: Option<u32>,

    /// Maximum duration (in turns).
    #[serde(default)]
    pub max_duration: Option<String>,

    /// Duration per intensity level.
    #[serde(default)]
    pub dur_per_intensity: Option<String>,

    /// Whether the effect applies continuously.
    #[serde(default)]
    pub continuous: Option<bool>,

    /// Whether the effect decays over time.
    #[serde(default)]
    pub decay: Option<bool>,

    /// Whether the effect intensity increases.
    #[serde(default)]
    pub int_plus: Option<bool>,

    /// Whether the effect intensity decreases.
    #[serde(default)]
    pub int_minus: Option<bool>,

    /// Base effect modifier.
    #[serde(default)]
    pub base_mods: Option<EffectMods>,

    /// Modifiers per intensity level.
    #[serde(default)]
    pub scaling_mods: Option<EffectMods>,

    /// Body part modifiers.
    #[serde(default)]
    pub blood_analysis: Option<bool>,

    /// Remove effect on damage.
    #[serde(default)]
    pub remove_on_damage: Option<bool>,

    /// Remove effect on healing.
    #[serde(default)]
    pub remove_on_heal: Option<bool>,

    /// Effect disappears after max duration.
    #[serde(default)]
    pub max_effective: Option<bool>,

    /// Paint color to apply.
    #[serde(default)]
    pub paint: Option<String>,

    /// Miss messages.
    #[serde(default)]
    pub miss_messages: Option<Vec<EffectMissMessage>>,

    /// Message shown when effect starts.
    #[serde(default)]
    pub apply_message: Option<String>,

    /// Message shown when effect ends.
    #[serde(default)]
    pub remove_message: Option<String>,

    /// Death event.
    #[serde(default)]
    pub death_event: Option<bool>,

    /// Rating (good/bad/neutral).
    #[serde(default)]
    pub rating: Option<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Effect on condition.
    #[serde(default)]
    pub effect_on_condition: Option<Vec<String>>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Resist effects
    #[serde(default)]
    pub resist_effects: Option<Vec<String>>,

    /// Show intensity
    #[serde(default)]
    pub show_intensity: Option<bool>,

    /// Duration add percentage
    #[serde(default)]
    pub dur_add_perc: Option<i32>,

    /// Enchantments
    #[serde(default)]
    pub enchantments: Option<Vec<serde_json::Value>>,

    /// Intensity duration factor
    #[serde(default)]
    pub int_dur_factor: Option<i32>,

    /// Resist traits
    #[serde(default)]
    pub resist_traits: Option<Vec<String>>,

    /// Limb score modifiers
    #[serde(default)]
    pub limb_score_mods: Option<serde_json::Value>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A named value pair for effect names/descriptions per intensity level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectNamedValue {
    /// The localized string value.
    pub str: String,

    /// Context/translator note (optional, not used in game logic).
    #[serde(rename = "//~", default)]
    pub context: Option<String>,

    /// Whether this is a NO_I18N string.
    #[serde(default)]
    pub no_i18n: Option<bool>,
}

/// Effect modifiers applied to the character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectMods {
    /// Stat modifiers.
    #[serde(default)]
    pub str_mod: Option<Vec<i32>>,
    #[serde(default)]
    pub dex_mod: Option<Vec<i32>>,
    #[serde(default)]
    pub per_mod: Option<Vec<i32>>,
    #[serde(default)]
    pub int_mod: Option<Vec<i32>>,

    /// Speed modifier.
    #[serde(default)]
    pub speed_mod: Option<Vec<i32>>,

    /// Pain modifier.
    #[serde(default)]
    pub pain_mod: Option<Vec<i32>>,

    /// Hunger modifier.
    #[serde(default)]
    pub hunger_mod: Option<Vec<i32>>,

    /// Thirst modifier.
    #[serde(default)]
    pub thirst_mod: Option<Vec<i32>>,

    /// Fatigue modifier.
    #[serde(default)]
    pub fatigue_mod: Option<Vec<i32>>,

    /// Health modifier (long-term).
    #[serde(default)]
    pub health_mod: Option<Vec<i32>>,

    /// Stamina modifier.
    #[serde(default)]
    pub stamina_mod: Option<Vec<i32>>,

    /// Hit modifier.
    #[serde(default)]
    pub hit_mod: Option<Vec<i32>>,

    /// Dodge modifier.
    #[serde(default)]
    pub dodge_mod: Option<Vec<i32>>,

    /// Damage amount.
    #[serde(default)]
    pub damage_amount: Option<Vec<i32>>,

    /// Damage type.
    #[serde(default)]
    pub damage_type: Option<Vec<String>>,

    /// Damage message.
    #[serde(default)]
    pub damage_message: Option<Vec<String>>,

    /// Body part damage chance.
    #[serde(default)]
    pub damage_chance: Option<Vec<i32>>,

    /// Body parts affected.
    #[serde(default)]
    pub body_part: Option<Vec<String>>,

    /// Field intensity.
    #[serde(default)]
    pub field_intensity: Option<Vec<i32>>,

    /// Field type.
    #[serde(default)]
    pub field_type: Option<Vec<String>>,

    /// Field chance.
    #[serde(default)]
    pub field_chance: Option<Vec<i32>>,

    /// Hurt chance.
    #[serde(default)]
    pub hurt_chance: Option<Vec<i32>>,

    /// Sleepiness modifier.
    #[serde(default)]
    pub sleepiness_mod: Option<Vec<i32>>,

    /// This is a sleep effect.
    #[serde(default)]
    pub is_sleep: Option<bool>,

    /// Modifier for thirst.
    #[serde(default)]
    pub thirst: Option<Vec<i32>>,

    /// Modifier for hunger.
    #[serde(default)]
    pub hunger: Option<Vec<i32>>,

    /// Modifier for fatigue.
    #[serde(default)]
    pub fatigue: Option<Vec<i32>>,

    /// Modifier for pain.
    #[serde(default)]
    pub pain: Option<Vec<i32>>,

    /// Modifier for stamina.
    #[serde(default)]
    pub stamina: Option<Vec<i32>>,

    /// Modifier for health.
    #[serde(default)]
    pub health: Option<Vec<i32>>,

    /// Modifier for speed.
    #[serde(default)]
    pub speed: Option<Vec<i32>>,

    /// Toxicity modifier.
    #[serde(default)]
    pub toxicity: Option<Vec<i32>>,

    /// Modifier for radiation.
    #[serde(default)]
    pub radiation: Option<Vec<i32>>,

    /// Modifier for thirst.
    #[serde(default)]
    pub thirst_modifier: Option<Vec<i32>>,

    /// Modifier for hunger.
    #[serde(default)]
    pub hunger_modifier: Option<Vec<i32>>,

    /// Modifier for fatigue.
    #[serde(default)]
    pub fatigue_modifier: Option<Vec<i32>>,

    /// Modifier for pain.
    #[serde(default)]
    pub pain_modifier: Option<Vec<i32>>,

    /// Modifier for stamina.
    #[serde(default)]
    pub stamina_modifier: Option<Vec<i32>>,

    /// Modifier for health.
    #[serde(default)]
    pub health_modifier: Option<Vec<i32>>,

    /// Modifier for speed.
    #[serde(default)]
    pub speed_modifier: Option<Vec<i32>>,
}

/// A miss message for when an attack fails due to this effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectMissMessage {
    /// Message text.
    pub message: String,
    /// Whether it's a global message.
    pub global: bool,
}
