use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An overmap terrain vision definition from JSON type `"oter_vision"`.
///
/// Defines how an overmap terrain appears on the map at various vision levels
/// (e.g. isolated building, city building, forest).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OterVisionDef {
    /// Unique identifier (e.g. "default", "isolated_building", "forested").
    pub id: DefId<OterVisionDef>,

    /// Display name of the vision level.
    #[serde(default)]
    pub name: Option<LocalizedString>,
}
