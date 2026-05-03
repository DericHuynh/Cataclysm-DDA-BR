use crate::raw_defs::cdda_types::RawValue;
use crate::raw_types::{DefId, LocalizedString};
use cdda_core::units::Energy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A bionic/CBM definition from JSON type `"bionic"`.
///
/// Bionics are cybernetic implants that can be installed in a character's body,
/// providing special abilities, stat boosts, or other effects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BionicDef {
    /// Unique identifier (e.g. "bio_adrenaline", "bio_tools").
    pub id: DefId<BionicDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    pub description: LocalizedString,

    /// Body parts occupied by this bionic and their capacity usage.
    #[serde(default)]
    pub occupied_bodyparts: Vec<BodyPartOccupation>,

    /// Power cost to activate.
    #[serde(default)]
    pub act_cost: Option<String>,

    /// Power cost per turn while active.
    #[serde(default)]
    pub react_cost: Option<String>,

    /// Power cost to trigger.
    #[serde(default)]
    pub trigger_cost: Option<String>,

    /// Activation time.
    #[serde(default)]
    pub time: Option<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Enchantments granted.
    /// CDDA enchantments can be bare strings like "THERMAL_VISION_GOOD" or objects.
    #[serde(default)]
    pub enchantments: Option<Vec<crate::raw_defs::cdda_types::RawValue>>,

    /// Activated EOC effects.
    #[serde(default)]
    pub activated_eocs: Option<Vec<String>>,

    /// Deactivated EOC effects.
    #[serde(default)]
    pub deactivated_eocs: Option<Vec<String>>,

    /// Processing EOC effects (every turn while active).
    #[serde(default)]
    pub processed_eocs: Option<Vec<String>>,

    /// Mutation conflicts.
    #[serde(default)]
    pub mutation_conflicts: Vec<String>,

    /// Fake item created by this bionic.
    #[serde(default)]
    pub fake_item: Option<String>,

    /// Passive pseudo items provided.
    #[serde(default)]
    pub passive_pseudo_items: Option<Vec<String>>,

    /// Fuel type for this bionic.
    #[serde(default)]
    pub fuel_type: Option<String>,

    /// Fuel capacity.
    #[serde(default)]
    pub fuel_capacity: Option<String>,

    /// Power capacity in Joules.
    #[serde(default)]
    pub capacity: Option<Energy>,

    /// Coverage overrides.
    #[serde(default)]
    pub coverage: Option<u32>,

    /// Encumbrance per body part as raw value (format: [["head", 1]]).
    #[serde(default)]
    pub encumbrance: Option<Vec<RawValue>>,

    /// Difficulty of installation.
    #[serde(default)]
    pub difficulty: Option<u32>,

    /// Installation requirement IDs.
    #[serde(default)]
    pub installable: Option<bool>,

    /// Canceled by mutations.
    #[serde(default)]
    pub canceled_mutations: Vec<String>,

    /// Whether this is an upgrade bionic.
    #[serde(default)]
    pub upgraded_bionic: Option<DefId<BionicDef>>,

    /// Which body part this bionic is installed in.
    #[serde(default)]
    pub body_part: Option<String>,

    /// Whether this bionic is active.
    #[serde(default)]
    pub active: Option<bool>,

    /// Bionic groups for spawning.
    #[serde(default)]
    pub group: Option<String>,

    /// Social effects (key=social_score, value=modifier).
    #[serde(default)]
    pub social_modifiers: Option<HashMap<String, i32>>,

    /// Fuel options
    #[serde(default)]
    pub fuel_options: Option<Vec<String>>,

    /// Fake weapon
    #[serde(default)]
    pub fake_weapon: Option<String>,

    /// Fuel efficiency
    #[serde(default)]
    pub fuel_efficiency: Option<f64>,

    /// Protection values per body part as raw value (format: [["arm_r", {...}]]).
    #[serde(default)]
    pub protec: Option<Vec<RawValue>>,

    /// Active flags
    #[serde(default)]
    pub active_flags: Option<Vec<String>>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A body part slot occupied by a bionic.
///
/// CDDA format: either `["torso", 4]` (pair) or `{"body_part": "torso", ...}` (object).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BodyPartOccupation {
    /// Simple pair: ["body_part", capacity]
    Pair(String, u32),
    /// Object form: {"body_part": "torso", ...}
    Object {
        body_part: String,
        #[serde(default)]
        coverage: Option<u32>,
    },
}

/// A bionic group definition from JSON type `"bionic_group"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BionicGroupDef {
    /// Unique identifier.
    pub id: DefId<BionicGroupDef>,

    /// Bionics in this group.
    pub bionics: Vec<BionicGroupEntry>,
}

/// An entry in a bionic group.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BionicGroupEntry {
    /// Simple: "bionic_id"
    Simple(String),
    /// Object with probability.
    Obj {
        id: String,
        #[serde(default = "default_prob")]
        prob: u32,
    },
}

fn default_prob() -> u32 {
    100
}
