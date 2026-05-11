use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A construction category definition from JSON type `"construction_category"`.
///
/// Defines a category for grouping construction recipes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConstructionCategoryDef {
    /// Unique identifier (e.g. "ALL", "CONSTRUCT", "FURN").
    pub id: DefId<ConstructionCategoryDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,
}
