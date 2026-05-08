use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A body graph definition from JSON type `"body_graph"`.
///
/// Defines an ASCII-art style graphical representation of a body part
/// for use in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyGraphDef {
    /// Unique identifier (e.g. "arm_r", "head", "full_body").
    pub id: DefId<BodyGraphDef>,

    /// Parent body part this graph is associated with.
    #[serde(default)]
    pub parent_bodypart: Option<String>,

    /// Fill symbol for unlabeled areas.
    #[serde(default)]
    pub fill_sym: Option<String>,

    /// Fill color for unlabeled areas.
    #[serde(default)]
    pub fill_color: Option<String>,

    /// Rows of the ASCII art graph.
    #[serde(default)]
    pub rows: Option<Vec<String>>,

    /// Mapping of labels to sub body parts and colors.
    #[serde(default)]
    pub parts: Option<serde_json::Value>,

    /// Another body graph ID to mirror.
    #[serde(default)]
    pub mirror: Option<String>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
