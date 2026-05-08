use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A temperature removal blacklist definition from JSON type `"temperature_removal_blacklist"`.
///
/// Lists items whose temperature-tracking active flag should be removed during migration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TemperatureRemovalBlacklistDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<TemperatureRemovalBlacklistDef>,

    /// List of item IDs to remove temperature tracking from.
    pub list: Vec<String>,
}
