use crate::raw_defs::cdda_types::StringOrArray;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A technique definition from JSON type `"technique"`.
///
/// Defines a combat technique used in martial arts or monster attacks.
/// Techniques can modify damage, add effects, or provide special combat maneuvers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TechniqueDef {
    /// Unique identifier (e.g. "tec_biojutsu_disarm", "FANGS_BITE").
    pub id: String,

    /// Display name.
    #[serde(default)]
    pub name: Option<String>,

    /// Description text.
    #[serde(default)]
    pub description: Option<String>,

    /// Messages shown on use: [player_message, npc_message].
    #[serde(default)]
    pub messages: Option<Vec<String>>,

    /// Skill requirements (e.g. `[{"name": "unarmed", "level": 1}]`).
    #[serde(default)]
    pub skill_requirements: Option<Vec<SkillRequirement>>,

    /// Whether the technique works with unarmed attacks.
    #[serde(default)]
    pub unarmed_allowed: Option<bool>,

    /// Whether the technique works with melee weapons.
    #[serde(default)]
    pub melee_allowed: Option<bool>,

    /// Whether the technique works with ranged weapons.
    #[serde(default)]
    pub ranged_allowed: Option<bool>,

    /// Whether a critical hit is required for this technique.
    #[serde(default)]
    pub crit_tec: Option<bool>,

    /// Whether the technique can happen on any hit (including crits).
    #[serde(default)]
    pub crit_ok: Option<bool>,

    /// Whether this is a defensive technique (used when blocking/dodging).
    #[serde(default)]
    pub defensive: Option<bool>,

    /// Whether this technique allows a miss recovery.
    #[serde(default)]
    pub miss_recovery: Option<bool>,

    /// Whether this technique breaks grabs.
    #[serde(default)]
    pub grab_break: Option<bool>,

    /// Whether the technique can be used at reach.
    #[serde(default)]
    pub reach_ok: Option<bool>,

    /// Whether the technique requires being adjacent to a wall.
    #[serde(default)]
    pub wall_adjacent: Option<bool>,

    /// Whether the technique disarms the target.
    #[serde(default)]
    pub disarms: Option<bool>,

    /// Whether the technique takes down the target.
    #[serde(default)]
    pub take_downs: Option<bool>,

    /// Whether the technique blocks hits.
    #[serde(default)]
    pub blocking: Option<bool>,

    /// Whether the technique throws the target.
    #[serde(default)]
    pub throw_attack: Option<bool>,

    /// Weighting factor for technique selection.
    #[serde(default)]
    pub weighting: Option<f64>,

    /// Flat bonuses applied by this technique (e.g. `[{"stat": "damage", "type": "bash", "scale": 1.5}]`).
    #[serde(default)]
    pub flat_bonuses: Option<Vec<TechniqueBonus>>,

    /// Multiplicative bonuses applied by this technique.
    #[serde(default)]
    pub mult_bonuses: Option<Vec<TechniqueBonus>>,

    /// Stun duration in turns.
    #[serde(default)]
    pub stun_dur: Option<u32>,

    /// Down duration in turns.
    #[serde(default)]
    pub down_dur: Option<u32>,

    /// Knockback distance in tiles.
    #[serde(default)]
    pub knockback_dist: Option<u32>,

    /// Knockback force.
    #[serde(default)]
    pub knockback_fix: Option<u32>,

    /// Area of effect type (e.g. "wide").
    #[serde(default)]
    pub aoe: Option<String>,

    /// Attack vectors allowed (e.g. `["vector_punch"]`).
    #[serde(default)]
    pub attack_vectors: Option<Vec<String>>,

    /// List of weapon categories allowed (e.g. "BIONIC_WEAPONRY").
    #[serde(default)]
    pub weapon_categories_allowed: Option<serde_json::Value>,

    /// Required buffs (all must be present). Can be a single buff ID or array.
    #[serde(default)]
    pub required_buffs_all: StringOrArray,

    /// Required buffs (any must be present). Can be a single buff ID or array.
    #[serde(default)]
    pub required_buffs_any: StringOrArray,

    /// Buffs that prevent this technique. Can be a single buff ID or array.
    #[serde(default)]
    pub forbidden_buffs_all: StringOrArray,

    /// Condition for technique activation (e.g. `{"not": {"npc_has_effect": "downed"}}`).
    #[serde(default)]
    pub condition: Option<serde_json::Value>,

    /// Description of the condition for tooltips.
    #[serde(default)]
    pub condition_desc: Option<String>,

    /// Tech effects applied by this technique (e.g. `[{"id": "disarmed", "chance": 100, ...}]`).
    #[serde(default)]
    pub tech_effects: Option<Vec<TechEffect>>,

    /// Minimum repeat count.
    #[serde(default)]
    pub repeat_min: Option<u32>,

    /// Maximum repeat count.
    #[serde(default)]
    pub repeat_max: Option<u32>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Abstract flag — if true, this definition is a template that should not be
    /// instantiated directly.
    #[serde(default)]
    pub abstract_: Option<bool>,
}

/// A skill requirement for a technique.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillRequirement {
    /// Skill name (e.g. "unarmed", "melee", "dodge").
    pub name: String,
    /// Required skill level.
    pub level: u32,
}

/// A bonus (flat or multiplicative) applied by a technique.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TechniqueBonus {
    /// Stat affected (e.g. "damage", "movecost", "arpen", "arm").
    pub stat: String,
    /// Damage type (e.g. "bash", "cut", "stab", "bullet", "heat").
    #[serde(default)]
    pub r#type: Option<String>,
    /// Bonus value (flat bonus).
    #[serde(default)]
    pub sca: Option<f64>,
    /// Scale multiplier.
    #[serde(default)]
    pub scale: Option<f64>,
    /// Arithmetic operation.
    #[serde(default)]
    pub arith: Option<String>,
}

/// A tech effect applied by a technique.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TechEffect {
    /// Effect ID.
    pub id: String,
    /// Chance of effect (percentage).
    #[serde(default)]
    pub chance: Option<serde_json::Value>,
    /// Duration in turns.
    #[serde(default)]
    pub duration: Option<serde_json::Value>,
    /// Whether the effect applies on damage.
    #[serde(default)]
    pub on_damage: Option<bool>,
    /// Message displayed when the effect triggers.
    #[serde(default)]
    pub message: Option<String>,
    /// Whether to save the message.
    #[serde(default)]
    pub save_message: Option<bool>,
    /// Intensity of the effect.
    #[serde(default)]
    pub intensity: Option<serde_json::Value>,
    /// Base chance (absolute).
    #[serde(default)]
    pub base_chance: Option<serde_json::Value>,
}
