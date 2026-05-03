use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A butchery requirement definition from JSON type `"butchery_requirement"`.
///
/// Defines the requirements (tools, skills) for different butchery types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ButcheryRequirementDef {
    /// Unique identifier (e.g. "default").
    pub id: DefId<ButcheryRequirementDef>,

    /// Mapping of size multipliers to butchery requirement sets.
    #[serde(default)]
    pub requirements: Option<serde_json::Value>,
}
