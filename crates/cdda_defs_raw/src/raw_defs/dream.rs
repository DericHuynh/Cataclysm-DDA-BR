use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A dream definition from JSON type `"dream"`.
///
/// Defines dream messages associated with a mutation category threshold.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DreamDef {
    /// List of dream messages.
    #[serde(default)]
    pub messages: Vec<String>,

    /// Mutation category (e.g. "MUTCAT_BIRD").
    pub category: String,

    /// Dream strength/level threshold.
    pub strength: u32,
}
