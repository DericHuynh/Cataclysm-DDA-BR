use crate::damage::Damage;
use crate::types::{DefId, LocalizedString};
use crate::units::{Volume, Weight};
use serde::{Deserialize, Serialize};

/// A monster definition from JSON type `"MONSTER"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDef {
    /// Unique identifier.
    pub id: DefId<MonsterDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    pub description: LocalizedString,

    /// Default faction this monster belongs to.
    #[serde(default)]
    pub default_faction: Option<String>,

    /// Body type (e.g. "human", "quadruped", "insect", "bird").
    #[serde(default)]
    pub bodytype: Option<String>,

    /// Species this monster belongs to (e.g. "ZOMBIE", "HUMAN", "FUNGUS").
    #[serde(default)]
    pub species: Vec<String>,

    /// Physical volume of the monster.
    #[serde(default)]
    pub volume: Option<Volume>,

    /// Physical weight of the monster.
    #[serde(default)]
    pub weight: Option<Weight>,

    /// Hit points.
    #[serde(default = "default_hp")]
    pub hp: u32,

    /// Base movement speed (higher = faster).
    #[serde(default)]
    pub speed: u32,

    /// Materials this monster is made of (e.g. "flesh", "bone", "steel").
    #[serde(default)]
    pub material: Vec<DefId<crate::defs::material::MaterialDef>>,

    /// ASCII symbol on the map.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for map display (can be string or map).
    #[serde(default)]
    pub color: Option<serde_json::Value>,

    /// Aggression (0 = passive, higher = more aggressive).
    #[serde(default)]
    pub aggression: u32,

    /// Morale (willingness to continue fighting).
    #[serde(default)]
    pub morale: u32,

    /// Melee skill level.
    #[serde(default)]
    pub melee_skill: u32,

    /// Number of melee damage dice.
    #[serde(default)]
    pub melee_dice: u32,

    /// Sides per melee damage die.
    #[serde(default)]
    pub melee_dice_sides: u32,

    /// Additional flat melee damage by type.
    #[serde(default)]
    pub melee_damage: Vec<DamageByType>,

    /// Daytime vision range.
    #[serde(default = "default_vision")]
    pub vision_day: u32,

    /// Nighttime vision range.
    #[serde(default = "default_vision")]
    pub vision_night: u32,

    /// Armor values by damage type.
    #[serde(default)]
    pub armor: Option<ArmorSet>,

    /// Grab strength (for grappling attacks).
    #[serde(default)]
    pub grab_strength: Option<u32>,

    /// Special attacks this monster can perform.
    #[serde(default)]
    pub special_attacks: Vec<SpecialAttack>,

    /// ID of the item group used for death drops.
    #[serde(default)]
    pub death_drops: Option<serde_json::Value>,

    /// What the monster burns into when set on fire.
    #[serde(default)]
    pub burn_into: Option<DefId<MonsterDef>>,

    /// What the monster becomes when fungalized.
    #[serde(default)]
    pub fungalize_into: Option<DefId<MonsterDef>>,

    /// Upgrade path (evolution into another monster type).
    #[serde(default)]
    pub upgrades: Option<UpgradeInfo>,

    /// Weakpoint sets for targeting.
    #[serde(default)]
    pub weakpoint_sets: Vec<String>,

    /// Proficiency families for dissection.
    #[serde(default)]
    pub families: Vec<String>,

    /// Harvest drop definition.
    #[serde(default)]
    pub harvest: Option<String>,

    /// Decay result definition.
    #[serde(default)]
    pub decay: Option<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Categories (e.g. "CLASSIC").
    #[serde(default)]
    pub categories: Vec<String>,

    /// Pathfinding settings.
    #[serde(default)]
    pub path_settings: Option<PathSettings>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Aggro character
    #[serde(default)]
    pub aggro_character: Option<bool>,

    /// Baby flags
    #[serde(default)]
    pub baby_flags: Option<Vec<String>>,

    /// Move skills
    #[serde(default)]
    pub move_skills: Option<Vec<serde_json::Value>>,

    /// Looks like another monster
    #[serde(default)]
    pub looks_like: Option<String>,

    /// Fear triggers
    #[serde(default)]
    pub fear_triggers: Option<Vec<String>>,

    /// Anger triggers
    #[serde(default)]
    pub anger_triggers: Option<Vec<String>>,

    /// Zombify into
    #[serde(default)]
    pub zombify_into: Option<String>,

    /// Difficulty
    #[serde(default)]
    pub diff: Option<u32>,

    /// Death function
    #[serde(default)]
    pub death_function: Option<serde_json::Value>,

    /// Reproduction data
    #[serde(default)]
    pub reproduction: Option<serde_json::Value>,

    /// Bleed rate
    #[serde(default)]
    pub bleed_rate: Option<u32>,

    /// Dissect result
    #[serde(default)]
    pub dissect: Option<serde_json::Value>,

    /// Dodge skill
    #[serde(default)]
    pub dodge: Option<u32>,

    /// Proportional modifications
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proportional: Option<serde_json::Value>,

    /// Delete operations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<serde_json::Value>,

    /// Extend operations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

fn default_hp() -> u32 {
    80
}

fn default_symbol() -> String {
    "Z".to_string()
}

fn default_vision() -> u32 {
    30
}

/// Damage by type (e.g. `{ "damage_type": "cut", "amount": 2 }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageByType {
    #[serde(rename = "damage_type")]
    pub damage_type: String,
    pub amount: i32,
}

/// Armor values for different damage types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmorSet {
    #[serde(default)]
    pub bash: u32,
    #[serde(default)]
    pub cut: u32,
    #[serde(default)]
    pub stab: u32,
    #[serde(default)]
    pub bullet: u32,
    #[serde(default)]
    pub heat: u32,
    #[serde(default)]
    pub cold: u32,
    #[serde(default)]
    pub electric: u32,
    #[serde(default)]
    pub acid: u32,
    #[serde(default)]
    pub biological: u32,
}

/// A special attack entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpecialAttack {
    /// Simple pair: ["attack_id", cooldown]
    SimplePair(String, u32),
    /// Object with detailed attack data.
    Object(SpecialAttackObj),
}

/// Detailed special attack object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialAttackObj {
    pub id: String,
    #[serde(default)]
    pub cooldown: Option<u32>,
    #[serde(default)]
    pub cost: Option<u32>,
    #[serde(default)]
    pub min_range: Option<u32>,
    #[serde(default)]
    pub max_range: Option<u32>,
    #[serde(default)]
    pub accuracy: Option<i32>,
    #[serde(default)]
    pub damage: Option<Damage>,
    #[serde(default)]
    pub dodgeable: Option<bool>,
    #[serde(default)]
    pub blockable: Option<bool>,
    #[serde(default)]
    pub hit_msg: Option<String>,
    #[serde(default)]
    pub miss_msg: Option<String>,
    #[serde(default)]
    pub no_dmg_msg: Option<String>,
}

/// Monster upgrade / evolution info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeInfo {
    #[serde(default)]
    pub half_life: Option<u32>,
    #[serde(default)]
    pub age_grow: Option<u32>,
    #[serde(default)]
    pub into_group: Option<String>,
    #[serde(default)]
    pub multi_level: Option<bool>,
}

/// Pathfinding settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSettings {
    #[serde(default)]
    pub max_dist: Option<i32>,
    #[serde(default)]
    pub max_length: Option<i32>,
    #[serde(default)]
    pub bash: Option<i32>,
    #[serde(default)]
    pub allow_open_doors: Option<bool>,
    #[serde(default)]
    pub allow_climb_stairs: Option<bool>,
    #[serde(default)]
    pub avoid_traps: Option<bool>,
    #[serde(default)]
    pub avoid_sharp: Option<bool>,
}
