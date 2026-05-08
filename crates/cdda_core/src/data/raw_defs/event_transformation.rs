use crate::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An event transformation definition from JSON type `"event_transformation"`.
///
/// Defines a transformation pipeline for game events, filtering and
/// reshaping event data for use in statistics and achievements.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EventTransformationDef {
    /// Unique identifier (e.g. "avatar_kills_monster", "avatar_wakes_up").
    pub id: DefId<EventTransformationDef>,

    /// The event type to filter (e.g. "character_kills_monster", "character_wakes_up").
    /// If not present, this transformation chains from another event_transformation.
    #[serde(default)]
    pub event_type: Option<String>,

    /// Constraints on field values to filter events.
    #[serde(default)]
    pub value_constraints: Option<serde_json::Value>,

    /// Fields to drop from the event data.
    #[serde(default)]
    pub drop_fields: Option<serde_json::Value>,

    /// New fields to add to the event data.
    #[serde(default)]
    pub new_fields: Option<serde_json::Value>,

    /// ID of another event transformation to chain from.
    #[serde(default)]
    pub event_transformation: Option<String>,
}
