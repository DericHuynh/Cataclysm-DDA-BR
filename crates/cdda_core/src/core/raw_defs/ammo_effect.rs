use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An ammo effect definition from JSON type `"ammo_effect"`.
///
/// Defines an effect that can be applied to ammunition (e.g. incendiary,
/// explosive, magic). Effects can have hardcoded behaviors in the game engine.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AmmoEffectDef {
    /// Unique identifier (e.g. "AE_NULL", "INCENDIARY", "EXPLOSIVE").
    pub id: DefId<AmmoEffectDef>,

    /// Display name of the ammo effect.
    #[serde(default)]
    pub name: Option<LocalizedString>,
}
