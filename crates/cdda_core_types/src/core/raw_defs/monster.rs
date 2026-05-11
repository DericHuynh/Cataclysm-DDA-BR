use crate::core::raw_defs::cdda_types::{
    CddaColor, DeathDrops, DeathFunction, RawValue, Reproduction, StringOrArray, UpgradeInfo,
};
use crate::core::raw_types::{DefId, LocalizedString};
use crate::core::units::{Volume, Weight};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A monster definition from JSON type `"MONSTER"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterDef {
    /// Unique identifier.
    pub id: DefId<MonsterDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Default faction this monster belongs to.
    #[serde(default)]
    pub default_faction: Option<String>,

    /// Body type (e.g. "human", "quadruped", "insect", "bird").
    #[serde(default)]
    pub bodytype: Option<String>,

    /// Species this monster belongs to (e.g. "ZOMBIE", "HUMAN", "FUNGUS").
    /// Can be a single string or array of strings.
    #[serde(default)]
    pub species: StringOrArray,

    /// Physical volume of the monster.
    #[serde(default)]
    pub volume: Option<Volume>,

    /// Physical weight of the monster.
    #[serde(default)]
    pub weight: Option<Weight>,

    /// Hit points.
    #[serde(default = "default_hp")]
    pub hp: i32,

    /// Base movement speed (higher = faster).
    #[serde(default)]
    pub speed: i32,

    /// Materials this monster is made of (e.g. "flesh", "bone", "steel").
    /// Can be a single string or array of strings.
    #[serde(default)]
    pub material: Option<RawValue>,

    /// ASCII symbol on the map.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for map display (can be string or map).
    #[serde(default)]
    pub color: Option<CddaColor>,

    /// Aggression (0 = passive, higher = more aggressive).
    #[serde(default)]
    pub aggression: i32,

    /// Morale (willingness to continue fighting).
    #[serde(default)]
    pub morale: i32,

    /// Melee skill level.
    #[serde(default)]
    pub melee_skill: i32,

    /// Number of melee damage dice.
    #[serde(default)]
    pub melee_dice: i32,

    /// Sides per melee damage die.
    #[serde(default)]
    pub melee_dice_sides: i32,

    /// Additional flat melee damage by type.
    #[serde(default)]
    pub melee_damage: Vec<DamageByType>,

    /// Daytime vision range.
    #[serde(default = "default_vision")]
    pub vision_day: i32,

    /// Nighttime vision range.
    #[serde(default = "default_vision")]
    pub vision_night: i32,

    /// Armor values by damage type.
    #[serde(default)]
    pub armor: Option<ArmorSet>,

    /// Grab strength (for grappling attacks).
    #[serde(default)]
    pub grab_strength: Option<i32>,

    /// Special attacks this monster can perform.
    #[serde(default)]
    pub special_attacks: Vec<SpecialAttack>,

    /// ID of the item group used for death drops.
    #[serde(default)]
    pub death_drops: Option<DeathDrops>,

    /// What the monster burns into when set on fire.
    #[serde(default)]
    pub burn_into: Option<DefId<MonsterDef>>,

    /// What the monster becomes when fungalized.
    #[serde(default)]
    pub fungalize_into: Option<DefId<MonsterDef>>,

    /// Upgrade path (evolution into another monster type).
    /// Can be an object with upgrade info, or `false` to disable.
    #[serde(default)]
    pub upgrades: Option<UpgradeInfo>,

    /// Weakpoint sets for targeting.
    /// Can be a single string or array of strings.
    #[serde(default)]
    pub weakpoint_sets: StringOrArray,

    /// Inline weakpoint definitions.
    /// Array of weakpoint objects with name, coverage, armor_mult, etc.
    #[serde(default)]
    pub weakpoints: Option<Vec<HashMap<String, RawValue>>>,

    /// Proficiency families for dissection.
    /// Can be a single string, array of strings, or array of mixed strings and objects.
    #[serde(default)]
    pub families: Option<RawValue>,

    /// Harvest drop definition (string ID or inline object).
    #[serde(default)]
    pub harvest: Option<RawValue>,

    /// Decay result definition.
    #[serde(default)]
    pub decay: Option<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Categories (e.g. "CLASSIC").
    /// Can be a single string or array of strings.
    #[serde(default)]
    pub categories: StringOrArray,

    /// Pathfinding settings.
    #[serde(default)]
    pub path_settings: Option<PathSettings>,

    /// Aggro character
    #[serde(default)]
    pub aggro_character: Option<bool>,

    /// Baby flags
    #[serde(default)]
    pub baby_flags: Option<Vec<String>>,

    /// Move skills as map of skill name to value (e.g. `{"climb": 8}`).
    #[serde(default)]
    pub move_skills: Option<HashMap<String, RawValue>>,

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
    pub diff: Option<i32>,

    /// Death function
    #[serde(default)]
    pub death_function: Option<DeathFunction>,

    /// Reproduction data
    #[serde(default)]
    pub reproduction: Option<Reproduction>,

    /// Bleed rate
    #[serde(default)]
    pub bleed_rate: Option<i32>,

    /// Dissect result (item group ID string).
    #[serde(default)]
    pub dissect: Option<String>,

    /// Dodge skill
    #[serde(default)]
    pub dodge: Option<i32>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

fn default_hp() -> i32 {
    80
}

fn default_symbol() -> String {
    "Z".to_string()
}

fn default_vision() -> i32 {
    30
}

/// Damage by type (e.g. `{ "damage_type": "cut", "amount": 2 }`).
/// `amount` is f64 because CDDA data sometimes uses `15.0` instead of `15`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DamageByType {
    #[serde(rename = "damage_type")]
    pub damage_type: String,
    pub amount: f64,
}

/// Armor values for different damage types.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArmorSet {
    #[serde(default)]
    pub bash: i32,
    #[serde(default)]
    pub cut: i32,
    #[serde(default)]
    pub stab: i32,
    #[serde(default)]
    pub bullet: i32,
    #[serde(default)]
    pub heat: i32,
    #[serde(default)]
    pub cold: i32,
    #[serde(default)]
    pub electric: i32,
    #[serde(default)]
    pub acid: i32,
    #[serde(default)]
    pub biological: i32,
}

/// A special attack entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SpecialAttack {
    /// Simple pair: ["attack_id", cooldown]
    SimplePair(String, u32),
    /// Object with detailed attack data.
    Object(SpecialAttackObj),
    /// Fallback for any unrecognized special attack format.
    Other(RawValue),
}

/// Detailed special attack object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecialAttackObj {
    /// Attack ID (used by most attacks).
    #[serde(default)]
    pub id: Option<String>,
    /// Attack type (used for "gun", "spell", etc. attacks).
    #[serde(default)]
    pub r#type: Option<String>,
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
    pub damage: Option<HashMap<String, RawValue>>,
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

/// Pathfinding settings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
