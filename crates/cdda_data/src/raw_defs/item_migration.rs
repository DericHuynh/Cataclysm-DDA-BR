use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An item migration definition from JSON type `"MIGRATION"`.
///
/// Defines how to migrate old item IDs to new ones when the game data changes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemMigrationDef {
    /// The old item identifier(s) to migrate from (can be a single string or array of strings).
    pub id: serde_json::Value,

    /// The new item identifier to replace with.
    #[serde(default)]
    pub replace: Option<String>,

    /// Flags to apply after migration.
    #[serde(default)]
    pub flags: Option<Vec<String>>,

    /// Content items to migrate (e.g. container contents).
    #[serde(default)]
    pub content: Option<Vec<serde_json::Value>>,

    /// Additional field used in MIGRATION definitions (e.g. variant).
    #[serde(default)]
    pub variant: Option<String>,
}
