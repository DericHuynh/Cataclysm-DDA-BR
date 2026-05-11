use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A speed description definition from JSON type `"speed_description"`.
///
/// Defines speed comparison descriptions for monsters relative to the player.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpeedDescriptionDef {
    /// Unique identifier (e.g. "DEFAULT").
    pub id: DefId<SpeedDescriptionDef>,

    /// List of speed threshold values and their descriptions.
    pub values: Vec<serde_json::Value>,
}
