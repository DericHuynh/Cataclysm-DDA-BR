use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A score definition from JSON type `"score"`.
///
/// Defines a score that the player can earn during gameplay. Scores track
/// various achievements and statistics (e.g. monsters killed, distance traveled,
/// items crafted).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScoreDef {
    /// Unique identifier (e.g. "score_kills", "score_distance_walked").
    pub id: DefId<ScoreDef>,

    /// Display name (can be localized, or missing for computed scores).
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text explaining what this score tracks.
    #[serde(default)]
    pub description: Option<String>,
}
