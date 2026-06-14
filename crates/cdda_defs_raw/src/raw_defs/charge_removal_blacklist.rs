use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A charge removal blacklist definition from JSON type `"charge_removal_blacklist"`.
///
/// Lists items whose charges should be silently removed during migration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChargeRemovalBlacklistDef {
    /// Unique identifier (arbitrary).
    pub id: DefId<ChargeRemovalBlacklistDef>,

    /// List of item IDs to remove charges from.
    pub list: Vec<String>,
}
