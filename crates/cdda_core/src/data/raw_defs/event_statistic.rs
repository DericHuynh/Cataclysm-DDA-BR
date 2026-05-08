use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An event statistic definition from JSON type `"event_statistic"`.
///
/// Defines a statistic tracked by the game's event system
/// (e.g. number of zombie kills, distance traveled).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EventStatisticDef {
    /// Unique identifier (e.g. "num_avatar_zombie_kills").
    pub id: DefId<EventStatisticDef>,

    /// Statistic type (e.g. "count", "unique_value", "last_value").
    pub stat_type: String,

    /// Event type to track (e.g. "character_kills_zombie").
    #[serde(default)]
    pub event_type: Option<String>,

    /// Field to extract from the event.
    #[serde(default)]
    pub field: Option<String>,
}
