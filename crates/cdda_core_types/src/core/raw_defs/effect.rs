use crate::core::raw_defs::cdda_types::{LimbScoreMod, RawValue};
use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An effect type definition from JSON type `"effect_type"`.
///
/// Defines a status effect that can be applied to characters or monsters
/// (e.g. "stunned", "poisoned", "bleeding", "on_fire").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EffectDef {
    /// Unique identifier (e.g. "poisoned", "stunned", "bleeding").
    pub id: DefId<EffectDef>,

    /// Display name (each intensity level can have its own name).
    /// Can be an array of strings or array of named-value objects.
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

    /// Maximum duration (in turns). Can be a number or time string like "30 s".
    #[serde(default)]
    pub max_duration: Option<RawValue>,

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

    /// Value added to intensity each tick.
    #[serde(default)]
    pub int_add_val: Option<i32>,

    /// Intensity decay step.
    #[serde(default)]
    pub int_decay_step: Option<i32>,

    /// Intensity decay tick.
    #[serde(default)]
    pub int_decay_tick: Option<u32>,

    /// Whether to remove effect when intensity reaches 0 from decay.
    #[serde(default)]
    pub int_decay_remove: Option<bool>,

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
    /// Can be a single string, or an array of [message, type] pairs per intensity.
    #[serde(default)]
    pub apply_message: Option<RawValue>,

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
    /// CDDA enchantments can be bare strings, objects, or arrays.
    #[serde(default)]
    pub enchantments: Option<Vec<crate::core::raw_defs::cdda_types::RawValue>>,

    /// Intensity duration factor. Can be a number or time string like "5 s".
    #[serde(default)]
    pub int_dur_factor: Option<RawValue>,

    /// Resist traits
    #[serde(default)]
    pub resist_traits: Option<Vec<String>>,

    /// Removed effects when this one is applied
    #[serde(default)]
    pub removes_effects: Option<Vec<String>>,

    /// Limb score modifiers (array of per-limb modifiers).
    #[serde(default)]
    pub limb_score_mods: Option<Vec<LimbScoreMod>>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A named value pair for effect names/descriptions per intensity level.
/// Can be either a plain string or an object with `str` and optional context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum EffectNamedValue {
    /// Plain string name/description.
    Plain(String),
    /// Object with `str` field and optional translator context.
    Structured {
        /// The actual string value.
        str: String,
        /// Context/translator note (e.g. "NO_I18N").
        #[serde(rename = "//~", default, skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
}

/// Effect modifiers applied to the character.
///
/// Each modifier is a vector of numeric values (typically 1 or 2 elements).
/// This uses RawValue to accept all CDDA formats including integers and floats.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EffectMods {
    /// Stat modifiers.
    #[serde(default)]
    pub str_mod: Option<Vec<RawValue>>,
    #[serde(default)]
    pub dex_mod: Option<Vec<RawValue>>,
    #[serde(default)]
    pub per_mod: Option<Vec<RawValue>>,
    #[serde(default)]
    pub int_mod: Option<Vec<RawValue>>,

    /// Speed modifier.
    #[serde(default)]
    pub speed_mod: Option<Vec<RawValue>>,

    /// Pain modifier.
    #[serde(default)]
    pub pain_mod: Option<Vec<RawValue>>,

    /// Hunger modifier.
    #[serde(default)]
    pub hunger_mod: Option<Vec<RawValue>>,

    /// Thirst modifier.
    #[serde(default)]
    pub thirst_mod: Option<Vec<RawValue>>,

    /// Fatigue modifier.
    #[serde(default)]
    pub fatigue_mod: Option<Vec<RawValue>>,

    /// Health modifier (long-term).
    #[serde(default)]
    pub health_mod: Option<Vec<RawValue>>,

    /// Stamina modifier.
    #[serde(default)]
    pub stamina_mod: Option<Vec<RawValue>>,

    /// Hit modifier.
    #[serde(default)]
    pub hit_mod: Option<Vec<RawValue>>,

    /// Dodge modifier.
    #[serde(default)]
    pub dodge_mod: Option<Vec<RawValue>>,

    /// Damage amount.
    #[serde(default)]
    pub damage_amount: Option<Vec<RawValue>>,

    /// Damage type.
    #[serde(default)]
    pub damage_type: Option<Vec<String>>,

    /// Damage message.
    #[serde(default)]
    pub damage_message: Option<Vec<String>>,

    /// Body part damage chance.
    #[serde(default)]
    pub damage_chance: Option<Vec<RawValue>>,

    /// Body parts affected.
    #[serde(default)]
    pub body_part: Option<Vec<String>>,

    /// Field intensity.
    #[serde(default)]
    pub field_intensity: Option<Vec<RawValue>>,

    /// Field type.
    #[serde(default)]
    pub field_type: Option<Vec<String>>,

    /// Field chance.
    #[serde(default)]
    pub field_chance: Option<Vec<RawValue>>,

    /// Hurt chance.
    #[serde(default)]
    pub hurt_chance: Option<Vec<RawValue>>,

    /// Hurt amount.
    #[serde(default)]
    pub hurt_amount: Option<Vec<RawValue>>,

    /// Hurt minimum.
    #[serde(default)]
    pub hurt_min: Option<Vec<RawValue>>,

    /// Sleepiness modifier.
    #[serde(default)]
    pub sleepiness_mod: Option<Vec<RawValue>>,

    /// This is a sleep effect.
    #[serde(default)]
    pub is_sleep: Option<bool>,

    /// Bash damage modifier.
    #[serde(default)]
    pub bash_mod: Option<Vec<RawValue>>,

    /// Size modifier.
    #[serde(default)]
    pub size_mod: Option<Vec<RawValue>>,

    /// Pain minimum value.
    #[serde(default)]
    pub pain_min: Option<Vec<RawValue>>,

    /// Pain chance (negative = more?).
    #[serde(default)]
    pub pain_chance: Option<Vec<RawValue>>,

    /// Pain max value.
    #[serde(default)]
    pub pain_max_val: Option<Vec<RawValue>>,

    /// Pain chance bottom.
    #[serde(default)]
    pub pain_chance_bot: Option<Vec<RawValue>>,

    /// Health amount modifier.
    #[serde(default)]
    pub health_amount: Option<Vec<RawValue>>,

    /// Health minimum modifier.
    #[serde(default)]
    pub health_min: Option<Vec<RawValue>>,

    /// Vomit chance.
    #[serde(default)]
    pub vomit_chance: Option<Vec<RawValue>>,

    /// Vomit tick.
    #[serde(default)]
    pub vomit_tick: Option<Vec<RawValue>>,

    /// Toxicity modifier.
    #[serde(default)]
    pub toxicity: Option<Vec<RawValue>>,

    /// Radiation modifier.
    #[serde(default)]
    pub radiation: Option<Vec<RawValue>>,

    /// Catch-all for any other modifier fields.
    #[serde(flatten)]
    pub extra: HashMap<String, RawValue>,
}

/// A miss message for when an attack fails due to this effect.
/// CDDA format: `["message", chance]` — a 2-element array (tuple).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EffectMissMessage(
    /// Message text.
    pub String,
    /// Chance of this message.
    pub u32,
);
