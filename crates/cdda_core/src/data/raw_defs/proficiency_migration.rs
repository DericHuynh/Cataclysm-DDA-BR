use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A proficiency migration definition from JSON type `"proficiency_migration"`.
///
/// Maps old proficiency IDs to new ones for migration purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProficiencyMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<ProficiencyMigrationDef>,

    /// The old proficiency ID to migrate from.
    pub from: Option<String>,

    /// The new proficiency ID to migrate to (optional — if absent, the proficiency is simply removed).
    #[serde(default)]
    pub to: Option<String>,
}
