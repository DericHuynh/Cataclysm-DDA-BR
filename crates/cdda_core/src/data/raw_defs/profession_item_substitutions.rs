use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A profession item substitution definition from JSON type `"profession_item_substitutions"`.
///
/// Defines substitutions for items in profession starting equipment based on
/// character traits (e.g. replacing wool items for characters with wool allergy).
///
/// The JSON data may identify the item using `"item"`, `"trait"`, or `"group"`
/// instead of `"id"`, so all identifier fields are optional.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfessionItemSubstitutionsDef {
    /// Unique identifier (the item id being substituted).
    #[serde(default)]
    pub id: Option<DefId<ProfessionItemSubstitutionsDef>>,

    /// Item being substituted (used instead of `id`).
    #[serde(default)]
    pub item: Option<String>,

    /// Trait that triggers substitution.
    #[serde(default)]
    pub r#trait: Option<String>,

    /// Item group for bonus items.
    #[serde(default)]
    pub group: Option<serde_json::Value>,

    /// Bonus substitution rules.
    #[serde(default)]
    pub bonus: Option<serde_json::Value>,

    /// Substitution rules.
    #[serde(default)]
    pub sub: Option<serde_json::Value>,
}
