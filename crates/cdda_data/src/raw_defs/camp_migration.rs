use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A camp migration definition from JSON type `"camp_migration"`.
///
/// Defines a migration mapping for faction camps, linking a camp name
/// to an overmap terrain and faction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CampMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<CampMigrationDef>,

    /// The camp migration data (name, overmap_terrain, faction).
    pub camp_migrations: Option<serde_json::Value>,
}
