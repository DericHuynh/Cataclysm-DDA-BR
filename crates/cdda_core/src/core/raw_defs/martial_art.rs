use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A martial art style definition from JSON type `"martial_art"`.
///
/// Defines a combat style with buffs, techniques, weapon restrictions,
/// and learning requirements.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MartialArtDef {
    /// Unique identifier (e.g. "style_aikido", "style_bojutsu", "style_biojutsu").
    pub id: DefId<MartialArtDef>,

    /// Display name of the martial art.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description of the martial art.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Messages displayed when initiating the style: [player_message, npc_message].
    #[serde(default)]
    pub initiate: Option<Vec<String>>,

    /// Messages displayed when learning the style.
    #[serde(default)]
    pub learn: Option<Vec<String>>,

    /// Priority for auto-selection (higher = preferred).
    #[serde(default)]
    pub priority: Option<i32>,

    /// Difficulty of learning this style.
    #[serde(default)]
    pub learn_difficulty: Option<u32>,

    /// Primary skill for this style (e.g. "bashing", "unarmed", "melee").
    #[serde(default)]
    pub primary_skill: Option<String>,

    /// Number of arm blocks available.
    #[serde(default)]
    pub arm_block: Option<u32>,

    /// Number of leg blocks available.
    #[serde(default)]
    pub leg_block: Option<u32>,

    /// Static buffs that are always active while in this style.
    #[serde(default)]
    pub static_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when moving.
    #[serde(default)]
    pub onmove_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when pausing.
    #[serde(default)]
    pub onpause_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when attacking.
    #[serde(default)]
    pub onattack_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when hitting an opponent.
    #[serde(default)]
    pub onhit_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied on critical hits.
    #[serde(default)]
    pub oncrit_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when dodging.
    #[serde(default)]
    pub ondodge_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when blocking.
    #[serde(default)]
    pub onblock_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied when getting hit.
    #[serde(default)]
    pub ongethit_buffs: Option<Vec<MartialArtBuff>>,

    /// Buffs applied on killing an opponent.
    #[serde(default)]
    pub onkill_buffs: Option<Vec<MartialArtBuff>>,

    /// Technique IDs available in this style.
    #[serde(default)]
    pub techniques: Option<Vec<String>>,

    /// Allowed weapon IDs for this style.
    #[serde(default)]
    pub weapons: Option<Vec<String>>,

    /// Allowed weapon categories for this style.
    #[serde(default)]
    pub weapon_category: Option<Vec<String>>,

    /// Whether only melee weapons are allowed.
    #[serde(default)]
    pub strictly_melee: Option<bool>,

    /// Whether melee weapons are allowed at all.
    #[serde(default)]
    pub allow_melee: Option<bool>,

    /// Whether unarmed attacks are allowed.
    #[serde(default)]
    pub unarmed_allowed: Option<bool>,

    /// Whether all weapons are allowed.
    #[serde(default)]
    pub allow_all_weapons: Option<bool>,

    /// Whether weapon blocking is prevented.
    #[serde(default)]
    pub prevent_weapon_blocking: Option<bool>,

    /// Whether the style forces unarmed combat.
    #[serde(default)]
    pub force_unarmed: Option<bool>,

    /// Whether the style can be taught to others.
    #[serde(default)]
    pub teachable: Option<bool>,

    /// Whether arm blocks use bionic armor.
    #[serde(default)]
    pub arm_block_with_bio_armor_arms: Option<bool>,

    /// Whether leg blocks use bionic armor.
    #[serde(default)]
    pub leg_block_with_bio_armor_legs: Option<bool>,

    /// Bonus to the number of blocks available.
    #[serde(default)]
    pub bonus_blocks: Option<u32>,

    /// Bonus to the number of dodges available.
    #[serde(default)]
    pub bonus_dodges: Option<u32>,

    /// Number of free dodges per turn.
    #[serde(default)]
    pub free_dodges: Option<u32>,

    /// Flat bonuses applied while in this style.
    #[serde(default)]
    pub flat_bonuses: Option<Vec<serde_json::Value>>,

    /// Multiplicative bonuses applied while in this style.
    #[serde(default)]
    pub mult_bonuses: Option<Vec<serde_json::Value>>,
}

/// A buff applied by a martial art under certain conditions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MartialArtBuff {
    /// Unique identifier for this buff (e.g. "buff_aikido_static1").
    pub id: String,

    /// Display name of the buff.
    #[serde(default)]
    pub name: Option<String>,

    /// Description of the buff's effects.
    #[serde(default)]
    pub description: Option<String>,

    /// Duration of the buff in turns.
    #[serde(default)]
    pub buff_duration: Option<u32>,

    /// Maximum number of stacks.
    #[serde(default)]
    pub max_stacks: Option<u32>,

    /// Whether the buff persists across turns.
    #[serde(default)]
    pub persists: Option<bool>,

    /// Whether the buff requires unarmed combat.
    #[serde(default)]
    pub unarmed_allowed: Option<bool>,

    /// Whether the buff allows melee weapons.
    #[serde(default)]
    pub melee_allowed: Option<bool>,

    /// Skill requirements to activate this buff.
    #[serde(default)]
    pub skill_requirements: Option<Vec<serde_json::Value>>,

    /// Flat stat bonuses.
    #[serde(default)]
    pub flat_bonuses: Option<Vec<serde_json::Value>>,

    /// Multiplicative stat bonuses.
    #[serde(default)]
    pub mult_bonuses: Option<Vec<serde_json::Value>>,
}
