use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A mutation category type definition from JSON type `"mutation_type"`.
///
/// Defines a category of mutations (e.g. MUTCAT_BIRD, MUTCAT_FELINE, MUTCAT_LIZARD).
/// Mutation categories group related mutations together and determine which
/// category a character's mutation path belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationTypeDef {
    /// Unique identifier (e.g. "MUTCAT_BIRD").
    pub id: DefId<MutationTypeDef>,

    /// Display name (e.g. "Bird").
    #[serde(default)]
    pub name: Option<String>,

    /// Categories this mutation type belongs to (e.g. ["BIRD"]).
    #[serde(default)]
    pub category: Option<Vec<String>>,
}
