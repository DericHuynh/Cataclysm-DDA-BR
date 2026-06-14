use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A scent_type definition from JSON type `"scent_type"`.
///
/// Defines a type of scent that can be tracked by monsters.
/// Each scent type has a list of receptive species that can detect it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScentTypeDef {
    /// Unique identifier (e.g. "sc_human", "sc_flower", "sc_fetid").
    pub id: DefId<ScentTypeDef>,

    /// List of monster species that can detect this scent.
    #[serde(default)]
    pub receptive_species: Option<Vec<String>>,
}
