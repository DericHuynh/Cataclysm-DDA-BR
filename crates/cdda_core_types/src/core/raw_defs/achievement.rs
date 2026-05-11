use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An achievement definition from JSON type `"achievement"`.
///
/// Defines an achievement that can be unlocked by the player.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AchievementDef {
    /// Unique identifier.
    pub id: DefId<AchievementDef>,

    /// Display name (can be a plain string, structured object, or missing).
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text (can be a plain string or structured object).
    #[serde(default)]
    pub description: Option<serde_json::Value>,

    /// Requirements to unlock this achievement.
    #[serde(default)]
    pub requirements: Option<Vec<serde_json::Value>>,

    /// Whether this achievement is hidden until unlocked.
    #[serde(default)]
    pub hidden: Option<bool>,

    /// Whether this is a manually given achievement ("manually_given" in JSON).
    #[serde(default, alias = "manually_given")]
    pub manual_completion: Option<bool>,
}
