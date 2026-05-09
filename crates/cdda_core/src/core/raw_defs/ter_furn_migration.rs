use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A terrain/furniture migration definition from JSON type `"ter_furn_migration"`.
///
/// Migrates a terrain or furniture from one ID to another.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerFurnMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<TerFurnMigrationDef>,

    /// Source terrain ID.
    pub from_ter: Option<String>,

    /// Target terrain ID.
    pub to_ter: Option<String>,

    /// Source furniture ID.
    #[serde(default)]
    pub from_furn: Option<String>,

    /// Target furniture ID.
    #[serde(default)]
    pub to_furn: Option<String>,
}
