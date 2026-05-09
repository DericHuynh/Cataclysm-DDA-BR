use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A movement_mode definition from JSON type `"movement_mode"`.
///
/// Defines a movement mode (e.g. "walk", "run", "crouch", "prone") with
/// display properties, speed multipliers, and exertion levels.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MovementModeDef {
    /// Unique identifier (e.g. "walk", "run", "crouch", "prone").
    pub id: String,

    /// Display name (e.g. "walk", "run", "crouch", "prone").
    #[serde(default)]
    pub name: Option<String>,

    /// Character shown on the map for entities in this mode.
    #[serde(default)]
    pub character: Option<String>,

    /// Character shown in the sidebar panel.
    #[serde(default)]
    pub panel_char: Option<String>,

    /// Color for the panel indicator.
    #[serde(default)]
    pub panel_color: Option<String>,

    /// Color for the map symbol.
    #[serde(default)]
    pub symbol_color: Option<String>,

    /// Exertion level (e.g. "MODERATE_EXERCISE", "ACTIVE_EXERCISE", "NO_EXERCISE").
    #[serde(default)]
    pub exertion_level: Option<String>,

    /// Exertion level when riding an animal.
    #[serde(default)]
    pub exertion_level_animal_riding: Option<String>,

    /// Preparation message (no mount).
    #[serde(default)]
    pub prepare_none: Option<String>,

    /// Preparation message (mounted on animal).
    #[serde(default)]
    pub prepare_animal: Option<String>,

    /// Preparation message (in mech).
    #[serde(default)]
    pub prepare_mech: Option<String>,

    /// Change-to-good message (no mount).
    #[serde(default)]
    pub change_good_none: Option<String>,

    /// Change-to-good message (mounted on animal).
    #[serde(default)]
    pub change_good_animal: Option<String>,

    /// Change-to-good message (in mech).
    #[serde(default)]
    pub change_good_mech: Option<String>,

    /// Change-to-bad message (no mount).
    #[serde(default)]
    pub change_bad_none: Option<String>,

    /// Change-to-bad message (in mech).
    #[serde(default)]
    pub change_bad_mech: Option<String>,

    /// Movement type string (e.g. "walking", "running", "crouching").
    #[serde(default)]
    pub move_type: Option<String>,

    /// Whether to stop hauling when entering this mode.
    #[serde(default)]
    pub stop_hauling: Option<bool>,

    /// Sound volume multiplier.
    #[serde(default)]
    pub sound_multiplier: Option<f64>,

    /// Movement speed multiplier.
    #[serde(default)]
    pub move_speed_multiplier: Option<f64>,

    /// Stamina consumption multiplier.
    #[serde(default)]
    pub stamina_multiplier: Option<f64>,

    /// Swim speed modifier (positive = slower in water).
    #[serde(default)]
    pub swim_speed_mod: Option<i32>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Abstract flag — if true, this definition is a template that should not be
    /// instantiated directly.
    #[serde(default)]
    pub abstract_: Option<bool>,
}
