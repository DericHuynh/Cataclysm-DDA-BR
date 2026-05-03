use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A shopkeeper blacklist definition from JSON type `"shopkeeper_blacklist"`.
///
/// Defines items or categories that a shopkeeper will not buy from the player.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShopkeeperBlacklistDef {
    /// Unique identifier.
    pub id: DefId<ShopkeeperBlacklistDef>,

    /// List of blacklist entries (groups, categories, etc.).
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
}
