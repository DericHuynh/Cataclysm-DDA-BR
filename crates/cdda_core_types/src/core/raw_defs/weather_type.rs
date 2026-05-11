use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A weather type definition from JSON type `"weather_type"`.
///
/// Defines a type of weather (e.g. "sunny", "rain", "snowing").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeatherTypeDef {
    /// Unique identifier (e.g. "sunny", "cloudy", "rain").
    pub id: DefId<WeatherTypeDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Color used in UI.
    #[serde(default)]
    pub color: Option<String>,

    /// Color used on the map.
    #[serde(default)]
    pub map_color: Option<String>,

    /// Symbol used in display.
    #[serde(default)]
    pub sym: Option<String>,

    /// Sun symbol.
    #[serde(default)]
    pub sun_sym: Option<String>,

    /// Ranged combat penalty.
    #[serde(default)]
    pub ranged_penalty: Option<i32>,

    /// Sight penalty multiplier.
    #[serde(default)]
    pub sight_penalty: Option<f64>,

    /// Light modifier (additive).
    #[serde(default)]
    pub light_modifier: Option<i32>,

    /// Sun multiplier.
    #[serde(default)]
    pub sun_multiplier: Option<f64>,

    /// Sound attenuation.
    #[serde(default)]
    pub sound_attn: Option<i32>,

    /// Whether this weather is dangerous.
    #[serde(default)]
    pub dangerous: Option<bool>,

    /// Precipitation type.
    #[serde(default)]
    pub precip: Option<String>,

    /// Whether it rains.
    #[serde(default)]
    pub rains: Option<bool>,

    /// Sound category.
    #[serde(default)]
    pub sound_category: Option<String>,

    /// Condition for this weather to occur.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Priority for weather selection.
    #[serde(default)]
    pub priority: Option<i32>,

    /// Required preceding weather types.
    #[serde(default)]
    pub required_weathers: Vec<String>,

    /// Tiles animation type.
    #[serde(default)]
    pub tiles_animation: Option<String>,

    /// Weather animation configuration.
    #[serde(default)]
    pub weather_animation: Option<serde_json::Value>,

    /// Debug EOC for cause.
    #[serde(default)]
    pub debug_cause_eoc: Option<String>,

    /// Debug EOC for leave.
    #[serde(default)]
    pub debug_leave_eoc: Option<String>,
}
