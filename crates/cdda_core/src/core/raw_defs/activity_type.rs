use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An activity type definition from JSON type `"activity_type"`.
///
/// Defines a player activity (e.g. reading, reloading, crafting) with its
/// verb display text and behavioral flags.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActivityTypeDef {
    /// Unique identifier (e.g. "ACT_RELOAD", "ACT_READ").
    pub id: DefId<ActivityTypeDef>,

    /// Display name of the activity.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Verb phrase describing the activity (e.g. "reloading", "reading").
    #[serde(default)]
    pub verb: Option<LocalizedString>,

    /// Whether the activity can be suspended and resumed.
    #[serde(default)]
    pub suspendable: Option<bool>,

    /// Whether the character is rooted in place during the activity.
    #[serde(default)]
    pub rooted: Option<bool>,
}
