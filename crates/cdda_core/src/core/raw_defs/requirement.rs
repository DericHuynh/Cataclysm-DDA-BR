use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A crafting/construction requirement definition from JSON type `"requirement"`.
///
/// Defines the components, tools, qualities, and skills needed to perform
/// a recipe or construction action. Requirements can be reused across
/// multiple recipes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RequirementDef {
    /// Unique identifier (e.g. "ammo_9mm", "welding_standard").
    pub id: DefId<RequirementDef>,

    /// List of component choices. Each inner list is an alternative (OR),
    /// and each item is [item_id, count].
    /// Can also be a map/object in some cases.
    #[serde(default)]
    pub components: Option<serde_json::Value>,

    /// List of tool choices. Each inner list is an alternative (OR),
    /// and each item is [tool_id, count_or_charges].
    /// Can also be a map/object in some cases.
    #[serde(default)]
    pub tools: Option<serde_json::Value>,

    /// List of quality requirements. Each inner list is an alternative (OR),
    /// and each item is {"id": "HAMMER", "level": 2}.
    /// Can also be a map/object in some cases.
    #[serde(default)]
    pub qualities: Option<serde_json::Value>,

    /// Required skills as [skill_id, level] pairs.
    /// Can also be a map/object in some cases.
    #[serde(default)]
    pub skills: Option<serde_json::Value>,

    /// Time required in seconds (or other time format).
    #[serde(default)]
    pub time: Option<serde_json::Value>,

    /// Reference to another requirement to reuse.
    #[serde(default)]
    pub using: Option<serde_json::Value>,
}
