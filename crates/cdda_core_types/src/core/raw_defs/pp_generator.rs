use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A post-process generator definition from JSON type `"pp_generator"`.
///
/// Defines a post-process generator that applies modifications to
/// generated maps (e.g. riot damage, fire).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PpGeneratorDef {
    /// Unique identifier (e.g. "riot_damage", "aftershock_ruin").
    pub id: DefId<PpGeneratorDef>,

    /// List of sub-generators with their parameters.
    #[serde(default)]
    pub sub_generators: Vec<serde_json::Value>,
}
