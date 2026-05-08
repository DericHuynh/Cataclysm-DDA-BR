use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A monster blacklist definition from JSON type `"MONSTER_BLACKLIST"`.
///
/// Defines a list of monsters that should not spawn.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterBlacklistDef {
    /// List of monster IDs to blacklist.
    #[serde(default)]
    pub monsters: Vec<String>,
}
