use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A terrain/furniture transform definition from JSON type `"ter_furn_transform"`.
///
/// Defines a transformation that changes terrain or furniture tiles based on
/// matching conditions (e.g. turning doors to frames, walls to rubble).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerFurnTransformDef {
    /// Unique identifier (e.g. "remove_door", "clearcut").
    pub id: DefId<TerFurnTransformDef>,

    /// Terrain transformations to apply.
    #[serde(default)]
    pub terrain: Option<serde_json::Value>,

    /// Furniture transformations to apply.
    #[serde(default)]
    pub furniture: Option<serde_json::Value>,

    /// Message displayed when the transform fails.
    #[serde(default)]
    pub fail_message: Option<String>,
}
