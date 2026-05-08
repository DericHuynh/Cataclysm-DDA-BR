use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A morale_type definition from JSON type `"morale_type"`.
///
/// Defines a type of morale modifier (e.g. "morale_food_good", "morale_music").
/// Each morale type has a display text template shown to the player.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MoraleTypeDef {
    /// Unique identifier (e.g. "morale_food_good", "morale_chat", "morale_music").
    pub id: DefId<MoraleTypeDef>,

    /// Display text template (e.g. "Enjoyed %s", "Music").
    #[serde(default)]
    pub text: Option<String>,
}
