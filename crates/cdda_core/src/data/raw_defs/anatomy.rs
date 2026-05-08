use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An anatomy definition from JSON type `"anatomy"`.
///
/// Defines the body plan for a creature, listing its body parts in order.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnatomyDef {
    /// Unique identifier (e.g. "human_anatomy", "default_anatomy").
    pub id: DefId<AnatomyDef>,

    /// List of body part IDs in the order they are arranged.
    pub parts: Vec<String>,
}
