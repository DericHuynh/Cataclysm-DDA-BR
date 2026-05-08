use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An overmap terrain ID migration definition from JSON type `"oter_id_migration"`.
///
/// Maps old overmap terrain IDs to new ones for migration purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OterIdMigrationDef {
    /// Unique identifier (arbitrary). Not present in all JSON entries.
    #[serde(default)]
    pub id: Option<DefId<OterIdMigrationDef>>,

    /// Map of old terrain IDs to new terrain IDs.
    pub oter_ids: HashMap<String, String>,
}
