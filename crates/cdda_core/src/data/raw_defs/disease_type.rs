use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A disease type definition from JSON type `"disease_type"`.
///
/// Defines a type of disease that can affect the player.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiseaseTypeDef {
    /// Unique identifier (e.g. "bad_food", "highly_contaminated_food").
    pub id: DefId<DiseaseTypeDef>,

    /// Minimum duration of the disease.
    #[serde(default)]
    pub min_duration: Option<String>,

    /// Maximum duration of the disease.
    #[serde(default)]
    pub max_duration: Option<String>,

    /// Minimum intensity of the disease.
    #[serde(default)]
    pub min_intensity: Option<i32>,

    /// Maximum intensity of the disease.
    #[serde(default)]
    pub max_intensity: Option<i32>,

    /// Health threshold for the disease.
    #[serde(default)]
    pub health_threshold: Option<i32>,

    /// The effect/symptom ID associated with this disease.
    #[serde(default)]
    pub symptoms: Option<String>,
}
