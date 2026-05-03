use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A vehicle part migration definition from JSON type `"vehicle_part_migration"`.
///
/// Maps old vehicle part IDs to new IDs for migration purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartMigrationDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<VehiclePartMigrationDef>,

    /// The old part ID.
    pub from: Option<String>,

    /// The new part ID.
    pub to: Option<String>,
}
