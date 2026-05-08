use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A profession group definition from JSON type `"profession_group"`.
///
/// Defines a group of background professions that the player can choose from.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfessionGroupDef {
    /// Unique identifier (e.g. "adult_basic_background").
    pub id: DefId<ProfessionGroupDef>,

    /// List of profession IDs in this group.
    pub professions: Vec<String>,
}
