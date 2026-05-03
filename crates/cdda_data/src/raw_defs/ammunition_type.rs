use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An ammunition_type definition from JSON type `"ammunition_type"`.
///
/// Defines a category of ammunition (e.g. "9mm", "223", "shot").
/// Each ammunition type groups together compatible ammo items under a common name.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AmmunitionTypeDef {
    /// Unique identifier (e.g. "9mm", "223", "shot").
    pub id: DefId<AmmunitionTypeDef>,

    /// Display name (e.g. "9x19mm Parabellum", ".223 Remington", "shot").
    pub name: LocalizedString,

    /// Default ammunition item ID for this type.
    #[serde(default)]
    pub default: Option<String>,
}
