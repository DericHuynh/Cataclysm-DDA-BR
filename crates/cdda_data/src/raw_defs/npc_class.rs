use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An NPC class definition from JSON type `"npc_class"`.
///
/// Defines a template class for NPCs, specifying traits, skills,
/// proficiencies, stat bonuses, and equipment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NpcClassDef {
    /// Unique identifier (e.g. "NC_SOLDIER", "NC_FARMER", "NC_DOCTOR").
    pub id: DefId<NpcClassDef>,

    /// Display name (localized string object, e.g. `{"str": "Soldier"}`).
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Job description text.
    #[serde(default)]
    pub job_description: Option<String>,

    /// Whether this class is commonly used for random NPCs.
    #[serde(default)]
    pub common: Option<bool>,

    /// Common spawn weight (float, default 1.0).
    #[serde(default)]
    pub common_spawn_weight: Option<f64>,

    /// Trait assignments. Can be an array of:
    /// - `{"trait": "TRAIT_ID", "weight": 100}`
    /// - `{"group": "group_id"}`
    /// - `["TRAIT_ID", 100]`
    #[serde(default)]
    pub traits: Option<Vec<serde_json::Value>>,

    /// Skill assignments. Array of objects with skill ID and level/bonus.
    #[serde(default)]
    pub skills: Option<Vec<NpcClassSkill>>,

    /// Proficiency IDs (array of strings).
    #[serde(default)]
    pub proficiencies: Option<Vec<String>>,

    /// Strength bonus (number or dice expression).
    #[serde(default)]
    pub bonus_str: Option<serde_json::Value>,

    /// Dexterity bonus (number or dice expression).
    #[serde(default)]
    pub bonus_dex: Option<serde_json::Value>,

    /// Intelligence bonus (number or dice expression).
    #[serde(default)]
    pub bonus_int: Option<serde_json::Value>,

    /// Perception bonus (number or dice expression).
    #[serde(default)]
    pub bonus_per: Option<serde_json::Value>,

    /// Aggression bonus (for personality).
    #[serde(default)]
    pub bonus_aggression: Option<serde_json::Value>,

    /// Bravery bonus (for personality).
    #[serde(default)]
    pub bonus_bravery: Option<serde_json::Value>,

    /// Altruism bonus (for personality).
    #[serde(default)]
    pub bonus_altruism: Option<serde_json::Value>,

    /// Equipment override item group ID.
    #[serde(default)]
    pub equipment: Option<String>,

    /// Weapon item group ID.
    #[serde(default)]
    pub weapon_override: Option<String>,

    /// Worn armor item group override.
    #[serde(default)]
    pub worn_override: Option<String>,

    /// Carry inventory item group override.
    #[serde(default)]
    pub carry_override: Option<String>,

    /// Collector bonus (for NPC barter pricing).
    /// Can be a number or a dice expression.
    #[serde(default)]
    pub bonus_collector: Option<serde_json::Value>,

    /// Shopkeeper item group ID (for shop NPCs).
    /// Can be a string (group ID) or an array of shop item objects.
    #[serde(default)]
    pub shopkeeper_item_group: Option<serde_json::Value>,

    /// Shopkeeper item price rules.
    #[serde(default)]
    pub shopkeeper_item_prices: Option<serde_json::Value>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A trait assignment for an NPC class.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NpcClassTrait {
    /// Specific trait ID (alternative to `group`).
    #[serde(default)]
    pub trait_: Option<String>,

    /// Trait group to draw from (alternative to `trait_`).
    #[serde(default)]
    pub group: Option<String>,

    /// Weight for random selection.
    #[serde(default)]
    pub weight: Option<serde_json::Value>,
}

/// A skill assignment for an NPC class.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NpcClassSkill {
    /// Skill ID (e.g. "mechanics", "survival", "ALL").
    pub skill: String,

    /// Skill level (number or complex expression).
    #[serde(default)]
    pub level: Option<serde_json::Value>,

    /// Skill bonus (number or complex expression, alternative to `level`).
    #[serde(default)]
    pub bonus: Option<serde_json::Value>,
}
