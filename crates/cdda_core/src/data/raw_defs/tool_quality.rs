use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A tool quality definition from JSON type `"tool_quality"`.
///
/// Defines a quality that a tool can possess (e.g. cutting, welding, hammering).
/// Tools with matching qualities can be used for specific crafting recipes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolQualityDef {
    /// Unique identifier (e.g. "CUT", "WELD", "HAMMER").
    pub id: DefId<ToolQualityDef>,

    /// Display name of the tool quality.
    pub name: LocalizedString,
}
