use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A variable migration definition from JSON type `"var_migration"`.
///
/// Maps old variable names to new ones for save migration purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VarMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<VarMigrationDef>,

    /// The old variable name.
    pub from: Option<String>,

    /// The new variable name.
    pub to: Option<String>,
}
