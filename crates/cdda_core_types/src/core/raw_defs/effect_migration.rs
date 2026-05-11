use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An effect migration definition from JSON type `"effect_migration"`.
///
/// Maps old effect IDs to new effect IDs for migration purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EffectMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<EffectMigrationDef>,

    /// The old effect ID to migrate from.
    pub from: Option<String>,

    /// The new effect ID to migrate to.
    pub to: Option<String>,
}
