use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A species definition from JSON type `"SPECIES"`.
///
/// Defines a species category for creatures (e.g. HUMAN, ZOMBIE, MAMMAL, BIRD, INSECT).
/// Species control inter-species aggression, roadkill food preferences, and other
/// behavioral flags.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpeciesDef {
    /// Unique identifier (e.g. "HUMAN", "ZOMBIE", "MAMMAL").
    pub id: DefId<SpeciesDef>,

    /// Display name (can be localized).
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text (can be localized).
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Behavioral flags for this species (e.g. "HUMAN", "ZOMBIE", "ANIMAL").
    #[serde(default)]
    pub flags: Vec<String>,

    /// Whether creatures of this species are considered roadkill food.
    #[serde(default)]
    pub roadkill_food: Option<bool>,
}
