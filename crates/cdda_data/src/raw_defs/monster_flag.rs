use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A monster flag definition from JSON type `"monster_flag"`.
///
/// Defines a flag that can be applied to monsters to grant specific behaviors
/// or properties (e.g. SEES, HEARS, WARM, POISON).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterFlagDef {
    /// Unique identifier (e.g. "SEES", "HEARS", "WARM").
    pub id: DefId<MonsterFlagDef>,

    /// Info text describing the flag's purpose.
    #[serde(default)]
    pub info: Option<String>,

    /// Description text.
    #[serde(default)]
    pub description: Option<String>,
}
