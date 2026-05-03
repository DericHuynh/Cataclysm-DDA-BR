use crate::types::{DefId, LocalizedString};
use serde::{Deserialize, Serialize};

/// A field type definition from JSON type `"field_type"`.
///
/// Fields are area effects like smoke, gas, fire, or electromagnetic interference
/// that occupy tiles and can have intensity levels with different properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Unique identifier (e.g. "fd_smoke", "fd_fire").
    pub id: DefId<FieldDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// ASCII symbol for display.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Display priority (higher draws first).
    #[serde(default)]
    pub priority: Option<i32>,

    /// Whether field is dangerous.
    #[serde(default)]
    pub dangerous: Option<bool>,

    /// Whether field is opaque.
    #[serde(default)]
    pub opaque: Option<bool>,

    /// Whether field is transparent.
    #[serde(default)]
    pub transparent: Option<bool>,

    /// Whether field exists in complete darkness.
    #[serde(default)]
    pub no_thickness: Option<bool>,

    /// Whether field doubles as a trap.
    #[serde(default)]
    pub is_trap: Option<bool>,

    /// Whether field doubles as a splatter.
    #[serde(default)]
    pub is_splatter: Option<bool>,

    /// Display color.
    #[serde(default)]
    pub color: Option<String>,

    /// Intensity levels with their properties.
    #[serde(default)]
    pub intensity_levels: Vec<FieldIntensity>,

    /// Time until the field decays (in turns).
    #[serde(default)]
    pub decay_time: Option<u32>,

    /// Chance per turn of field applying effect.
    #[serde(default)]
    pub apply_effects: Option<Vec<FieldEffect>>,

    /// Phases this field is present in.
    #[serde(default)]
    pub phase: Option<String>,

    /// Whether field can be seen through.
    #[serde(default)]
    pub sight_override: Option<f64>,

    /// Whether this field counts as a wall.
    #[serde(default)]
    pub wall: Option<bool>,

    /// Whether this field is edible.
    #[serde(default)]
    pub edible: Option<bool>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Ambient damage dealt by this field.
    #[serde(default)]
    pub ambient_damage: Option<crate::damage::Damage>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Gas absorption factor
    #[serde(default)]
    pub gas_absorption_factor: Option<u32>,

    /// Underwater age speedup
    #[serde(default)]
    pub underwater_age_speedup: Option<String>,

    /// Outdoor age speedup
    #[serde(default)]
    pub outdoor_age_speedup: Option<String>,

    /// Percent spread per turn
    #[serde(default)]
    pub percent_spread: Option<u32>,

    /// Decay amount factor
    #[serde(default)]
    pub decay_amount_factor: Option<u32>,

    /// Looks like
    #[serde(default)]
    pub looks_like: Option<String>,

    /// Half life
    #[serde(default)]
    pub half_life: Option<String>,

    /// Display field (parent field type)
    #[serde(default)]
    pub display_field: Option<String>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

fn default_symbol() -> String {
    "%".to_string()
}

/// Properties of a specific field intensity level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldIntensity {
    /// Name for this intensity level.
    pub name: Option<LocalizedString>,

    /// Symbol for this intensity.
    #[serde(default)]
    pub symbol: Option<String>,

    /// Color for this intensity.
    #[serde(default)]
    pub color: Option<String>,

    /// Transparency at this intensity (0.0 = opaque, 1.0 = transparent).
    #[serde(default)]
    pub transparency: Option<f64>,

    /// Movement cost modifier.
    #[serde(default)]
    pub move_cost: Option<i32>,

    /// Extra radius (for area effects).
    #[serde(default)]
    pub extra_radius: Option<u32>,

    /// Damage at this intensity.
    #[serde(default)]
    pub damage: Option<crate::damage::Damage>,

    /// Intensity of light emitted.
    #[serde(default)]
    pub light_emitted: Option<u32>,

    /// Intensity of light that can be seen through.
    #[serde(default)]
    pub light_override: Option<u32>,

    /// Effects applied at this intensity.
    #[serde(default)]
    pub effects: Option<Vec<FieldIntensityEffect>>,

    /// Humidity modifier.
    #[serde(default)]
    pub humidity: Option<i32>,

    /// Whether field is sticky at this intensity.
    #[serde(default)]
    pub sticky: Option<bool>,

    /// Whether field is dangerous at this intensity.
    #[serde(default)]
    pub dangerous: Option<bool>,
}

/// An effect applied by a field intensity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldIntensityEffect {
    /// Effect ID.
    pub effect_id: String,
    /// Intensity of the effect.
    #[serde(default)]
    pub intensity: Option<u32>,
    /// Chance per tick.
    #[serde(default)]
    pub chance: Option<u32>,
    /// Body part to apply effect to.
    #[serde(default)]
    pub body_part: Option<String>,
}

/// An effect that a field applies to creatures standing in it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEffect {
    /// Effect type ID.
    pub id: String,
    /// Minimum intensity to apply.
    #[serde(default)]
    pub intensity: Option<u32>,
    /// Chance per turn of application.
    #[serde(default)]
    pub chance: Option<u32>,
    /// Message to show on application.
    #[serde(default)]
    pub message: Option<String>,
}
