use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A construction_group definition from JSON type `"construction_group"`.
///
/// Groups constructions together under a display name for the construction menu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConstructionGroupDef {
    /// Unique identifier (e.g. "armor_reinforced_window", "build_bed").
    pub id: DefId<ConstructionGroupDef>,

    /// Display name.
    pub name: LocalizedString,
}
