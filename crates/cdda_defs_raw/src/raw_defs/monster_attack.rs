use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A monster special attack definition from JSON type `"monster_attack"`.
///
/// Defines a special attack that a monster can perform in combat, including
/// melee strikes, grabs, ranged pulls, and EOC-triggered abilities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterAttackDef {
    /// Unique identifier (e.g. "acid_slash", "grab", "feral_weapon_pipe").
    pub id: DefId<MonsterAttackDef>,

    /// Type of attack (e.g. "melee", "bite", "eoc").
    #[serde(default)]
    pub attack_type: Option<String>,

    /// Cooldown in turns between uses.
    #[serde(default)]
    pub cooldown: Option<u32>,

    /// Move cost to perform the attack.
    #[serde(default)]
    pub move_cost: Option<u32>,

    /// Accuracy bonus for the attack.
    #[serde(default)]
    pub accuracy: Option<i32>,

    /// Minimum size of the target hit (body part size index).
    #[serde(default)]
    pub hitsize_min: Option<u32>,

    /// Attack range in tiles.
    #[serde(default)]
    pub range: Option<u32>,

    /// Whether the attack can be dodged.
    #[serde(default)]
    pub dodgeable: Option<bool>,

    /// Whether the attack can be blocked.
    #[serde(default)]
    pub blockable: Option<bool>,

    /// Whether this attack is a grab attempt.
    #[serde(default)]
    pub grab: Option<bool>,

    /// Grab-specific data (e.g. grab effect, pull chance, pull weight ratio).
    #[serde(default)]
    pub grab_data: Option<serde_json::Value>,

    /// Maximum damage instances (array of {damage_type, amount} objects).
    #[serde(default)]
    pub damage_max_instance: Option<Vec<serde_json::Value>>,

    /// Effects applied to the target on hit.
    #[serde(default)]
    pub effects: Option<Vec<serde_json::Value>>,

    /// Whether effects require damage to be dealt.
    #[serde(default)]
    pub effects_require_dmg: Option<bool>,

    /// Effects always applied to the attacker.
    #[serde(default)]
    pub self_effects_always: Option<Vec<serde_json::Value>>,

    /// Effects applied to the attacker on hit.
    #[serde(default)]
    pub self_effects_onhit: Option<Vec<serde_json::Value>>,

    /// Condition for the attack to be available.
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// EOC (effect-on-condition) ID for EOC-type attacks.
    #[serde(default)]
    pub eoc: Option<serde_json::Value>,

    /// Message when the attack hits and deals damage to the player.
    #[serde(default)]
    pub hit_dmg_u: Option<String>,

    /// Message when the attack hits and deals damage to an NPC.
    #[serde(default)]
    pub hit_dmg_npc: Option<String>,

    /// Message when the attack hits but deals no damage to the player.
    #[serde(default)]
    pub no_dmg_msg_u: Option<String>,

    /// Message when the attack hits but deals no damage to an NPC.
    #[serde(default)]
    pub no_dmg_msg_npc: Option<String>,

    /// Message when the attack misses the player.
    #[serde(default)]
    pub miss_msg_u: Option<String>,

    /// Message when the attack misses an NPC.
    #[serde(default)]
    pub miss_msg_npc: Option<String>,

    /// Monster message displayed when the attack is used.
    #[serde(default)]
    pub monster_message: Option<String>,
}
