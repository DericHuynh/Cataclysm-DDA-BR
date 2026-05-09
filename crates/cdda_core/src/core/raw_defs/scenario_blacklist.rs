use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A scenario blacklist definition from JSON type `"SCENARIO_BLACKLIST"`.
///
/// Defines scenarios that should be hidden or blocked from the player.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioBlacklistDef {
    /// Subtype of the blacklist (e.g. "blacklist").
    #[serde(default)]
    pub subtype: Option<String>,

    /// List of scenario IDs to blacklist.
    #[serde(default)]
    pub scenarios: Vec<String>,
}
