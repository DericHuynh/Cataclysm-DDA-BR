use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A weather generator definition from JSON type `"weather_generator"`.
///
/// Defines parameters for procedurally generating weather in a region.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeatherGeneratorDef {
    /// Unique identifier (e.g. "default", "highland_weather").
    pub id: DefId<WeatherGeneratorDef>,

    /// Base temperature in Celsius.
    #[serde(default)]
    pub base_temperature: Option<f64>,

    /// Base humidity percentage.
    #[serde(default)]
    pub base_humidity: Option<f64>,

    /// Base pressure in hPa.
    #[serde(default)]
    pub base_pressure: Option<f64>,

    /// Base wind speed.
    #[serde(default)]
    pub base_wind: Option<f64>,

    /// Base wind distribution peaks.
    #[serde(default)]
    pub base_wind_distrib_peaks: Option<i32>,

    /// Base wind seasonal variation.
    #[serde(default)]
    pub base_wind_season_variation: Option<i32>,

    /// Manual temperature modifier for spring.
    #[serde(default)]
    pub spring_temp_manual_mod: Option<i32>,

    /// Manual temperature modifier for summer.
    #[serde(default)]
    pub summer_temp_manual_mod: Option<i32>,

    /// Manual temperature modifier for autumn.
    #[serde(default)]
    pub autumn_temp_manual_mod: Option<i32>,

    /// Manual temperature modifier for winter.
    #[serde(default)]
    pub winter_temp_manual_mod: Option<i32>,

    /// Whitelist of weather types.
    #[serde(default)]
    pub weather_white_list: Vec<String>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
