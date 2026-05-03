use crate::types::{DefId, LocalizedString};
use crate::units::Energy;
use serde::{Deserialize, Serialize};

/// A bionic/CBM definition from JSON type `"bionic"`.
///
/// Bionics are cybernetic implants that can be installed in a character's body,
/// providing special abilities, stat boosts, or other effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub enchantments: Option<Vec<serde_json::Value>>,

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

    /// Encumbrance.
    #[serde(default)]
    pub encumbrance: Option<u32>,

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

    /// Social effects.
    #[serde(default)]
    pub social_mods: Option<serde_json::Value>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Fuel options
    #[serde(default)]
    pub fuel_options: Option<Vec<String>>,

    /// Fake weapon
    #[serde(default)]
    pub fake_weapon: Option<String>,

    /// Fuel efficiency
    #[serde(default)]
    pub fuel_efficiency: Option<f64>,

    /// Protection values
    #[serde(default)]
    pub protec: Option<serde_json::Value>,

    /// Active flags
    #[serde(default)]
    pub active_flags: Option<Vec<String>>,

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

/// A body part slot occupied by a bionic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPartOccupation {
    /// Body part ID.
    pub body_part: Option<String>,

    /// Direct match.
    #[serde(flatten)]
    pub pair: Option<(String, u32)>,
}

// Custom deserialization is needed here because the JSON format can be:
// "occupied_bodyparts": [ [ "torso", 4 ] ]
// or
// "occupied_bodyparts": [ { "body_part": "torso", ... } ]
// We'll handle the first format with a custom visitor.

/// A bionic group definition from JSON type `"bionic_group"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BionicGroupDef {
    /// Unique identifier.
    pub id: DefId<BionicGroupDef>,

    /// Bionics in this group.
    pub bionics: Vec<BionicGroupEntry>,
}

/// An entry in a bionic group.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
