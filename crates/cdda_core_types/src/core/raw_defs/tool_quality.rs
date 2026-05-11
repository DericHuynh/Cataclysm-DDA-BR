use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A tool quality definition from JSON type `"tool_quality"`.
///
/// Defines a quality that a tool can possess (e.g. cutting, welding, hammering).
/// Tools with matching qualities can be used for specific crafting recipes.
/// The `usages` field maps quality levels to the named actions they unlock.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolQualityDef {
    /// Unique identifier (e.g. "CUT", "WELD", "HAMMER").
    pub id: DefId<ToolQualityDef>,

    /// Display name of the tool quality.
    pub name: LocalizedString,

    /// Usage actions unlocked at each quality level.
    ///
    /// CDDA format: `[ [level, [action_name, ...]], ... ]`
    /// Example: `[ [1, ["salvage", "inscribe"]], [2, ["LUMBER"]] ]`
    /// At level 1, this quality enables "salvage" and "inscribe" actions.
    /// At level 2, it additionally enables "LUMBER".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usages: Option<Vec<QualityUsage>>,
}

/// A single usage entry linking a quality level to its enabled actions.
#[derive(Debug, Clone, JsonSchema)]
pub struct QualityUsage {
    /// Minimum quality level required to unlock these actions.
    pub level: u32,
    /// Action names enabled at this level (e.g. "salvage", "LUMBER", "FISH_ROD").
    pub actions: Vec<String>,
}

impl<'de> Deserialize<'de> for QualityUsage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (level, actions) = <(u32, Vec<String>)>::deserialize(deserializer)?;
        Ok(QualityUsage { level, actions })
    }
}

/// Serialize as CDDA tuple format `[level, [action, ...]]`.
impl Serialize for QualityUsage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&self.level)?;
        tup.serialize_element(&self.actions)?;
        tup.end()
    }
}
