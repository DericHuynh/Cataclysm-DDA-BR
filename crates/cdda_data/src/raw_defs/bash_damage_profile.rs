use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A bash damage profile definition from JSON type `"bash_damage_profile"`.
///
/// Defines a damage profile for bashing and other damage types against objects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashDamageProfileDef {
    /// Unique identifier (e.g. "default", "wooden_door").
    pub id: DefId<BashDamageProfileDef>,

    /// The damage profile mapping damage types to multipliers.
    #[serde(default)]
    pub profile: Option<serde_json::Value>,
}
