use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An addiction type definition from JSON type `"addiction_type"`.
///
/// Defines a type of addiction (e.g. "caffeine", "nicotine", "alcohol").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddictionTypeDef {
    /// Unique identifier (e.g. "caffeine", "nicotine").
    pub id: DefId<AddictionTypeDef>,

    /// Display name (e.g. "Caffeine Withdrawal").
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Name of the type of substance (e.g. "caffeine", "opiates").
    #[serde(default)]
    pub type_name: Option<String>,

    /// Description of the withdrawal effects.
    #[serde(default)]
    pub description: Option<serde_json::Value>,

    /// Morale effect ID associated with craving.
    #[serde(default)]
    pub craving_morale: Option<String>,

    /// Effect on condition ID for the addiction.
    #[serde(default)]
    pub effect_on_condition: Option<String>,

    /// Built-in effect name (alternative to effect_on_condition).
    #[serde(default)]
    pub builtin: Option<String>,
}
