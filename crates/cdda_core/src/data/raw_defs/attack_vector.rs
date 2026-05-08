use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An attack vector definition from JSON type `"attack_vector"`.
///
/// Defines a body part or combination of body parts used for attacks
/// (e.g. punch, bite, headbutt, kick).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttackVectorDef {
    /// Unique identifier (e.g. "vector_punch", "vector_bite").
    pub id: DefId<AttackVectorDef>,

    /// Display name of the attack vector.
    #[serde(default)]
    pub name: Option<LocalizedString>,
}
