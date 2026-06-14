use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A limb score definition from JSON type `"limb_score"`.
///
/// Defines a score for a limb, such as manipulation, lifting, or vision.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LimbScoreDef {
    /// Unique identifier (e.g. "manip", "lift", "vision").
    pub id: DefId<LimbScoreDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Whether this score is affected by wounds.
    #[serde(default)]
    pub affected_by_wounds: Option<bool>,

    /// Whether this score is affected by encumbrance.
    #[serde(default)]
    pub affected_by_encumb: Option<bool>,
}
