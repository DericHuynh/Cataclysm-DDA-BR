use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A trap migration definition from JSON type `"trap_migration"`.
///
/// Maps old trap IDs to new trap IDs for migration purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrapMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<TrapMigrationDef>,

    /// The old trap ID.
    pub from_trap: Option<String>,

    /// The new trap ID.
    pub to_trap: Option<String>,
}
