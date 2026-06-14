use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A trait migration definition from JSON type `"TRAIT_MIGRATION"`.
///
/// Maps old trait IDs to new trait IDs or marks them for removal.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraitMigrationDef {
    /// Unique identifier (the old trait ID).
    pub id: DefId<TraitMigrationDef>,

    /// The new trait ID to replace the old one.
    #[serde(default)]
    pub trait_: Option<String>,

    /// Whether to remove this trait.
    #[serde(default)]
    pub remove: Option<bool>,
}
