use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A shopkeeper consumption rates definition from JSON type `"shopkeeper_consumption_rates"`.
///
/// Defines how quickly a shopkeeper consumes items (for restocking purposes).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShopkeeperConsumptionRatesDef {
    /// Unique identifier.
    pub id: DefId<ShopkeeperConsumptionRatesDef>,

    /// Default consumption rate.
    #[serde(default)]
    pub default_rate: Option<i32>,

    /// Threshold below which items are considered junk.
    #[serde(default)]
    pub junk_threshold: Option<String>,

    /// List of specific item/category rates.
    #[serde(default)]
    pub rates: Vec<serde_json::Value>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Fields to extend from the base definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,
}
