use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A character modifier definition from JSON type `"character_mod"`.
///
/// Defines a modifier that affects character stats or behavior based on
/// limb scores, builtins, or other conditions (e.g. aim speed, stamina cost).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CharacterModDef {
    /// Unique identifier (e.g. "aim_speed_skill_mod", "stamina_move_cost_mod").
    pub id: DefId<CharacterModDef>,

    /// Description of the modifier.
    #[serde(default)]
    pub description: Option<String>,
}
