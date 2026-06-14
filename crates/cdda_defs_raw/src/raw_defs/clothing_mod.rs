use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A clothing mod definition from JSON type `"clothing_mod"`.
///
/// Defines a modification that can be applied to clothing (e.g. padding, lining).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClothingModDef {
    /// Unique identifier (e.g. "leather_padded", "furred").
    pub id: DefId<ClothingModDef>,

    /// Flag applied to the modified clothing.
    #[serde(default)]
    pub flag: Option<String>,

    /// Item used for the modification.
    #[serde(default)]
    pub item: Option<String>,

    /// Prompt text for implementing the mod.
    #[serde(default)]
    pub implement_prompt: Option<String>,

    /// Prompt text for destroying the mod.
    #[serde(default)]
    pub destroy_prompt: Option<String>,

    /// Whether this mod is restricted.
    #[serde(default)]
    pub restricted: Option<bool>,

    /// List of stat modifications.
    #[serde(default)]
    pub mod_value: Vec<serde_json::Value>,
}
