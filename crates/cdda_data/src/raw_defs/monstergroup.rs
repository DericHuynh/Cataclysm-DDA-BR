use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A monster group definition from JSON type `"monstergroup"`.
///
/// Defines a weighted group of monsters used for spawning in various locations.
/// Groups can reference individual monsters or other groups recursively.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterGroupDef {
    /// Unique identifier (e.g. "GROUP_CIVILIANS_UPGRADE", "GROUP_ZOMBIE").
    pub id: DefId<MonsterGroupDef>,

    /// List of monsters or sub-groups in this group.
    #[serde(default)]
    pub monsters: Vec<MonsterGroupEntry>,

    /// Default monster to spawn when no entry matches conditions.
    #[serde(default)]
    pub default_monster: Option<String>,

    /// Whether this group contains only animals.
    #[serde(default)]
    pub is_animal: Option<bool>,

    /// If set, this group replaces another monster group in the JSON inheritance system.
    #[serde(default)]
    pub replace_monster_group: Option<String>,

    /// If set, creates a new monster group with this ID and copies monsters from the original.
    #[serde(default)]
    pub new_monster_group: Option<String>,
}

/// An entry in a monster group, either referencing a specific monster or a sub-group.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MonsterGroupEntry {
    /// An entry referencing a specific monster type.
    Monster {
        /// Monster type ID (e.g. "mon_zombie").
        monster: String,
        /// Relative weight (probability) of this monster within the group.
        #[serde(default)]
        weight: Option<u32>,
        /// Cost multiplier for spawn cost calculations.
        #[serde(default)]
        cost_multiplier: Option<u32>,
        /// Pack size range [min, max] for spawning multiple at once.
        #[serde(default)]
        pack_size: Option<Vec<u32>>,
        /// Starting time (hour or time string like "45 days") when this monster can start spawning.
        #[serde(default)]
        starts: Option<serde_json::Value>,
        /// Ending time (hour or time string like "180 days") when this monster stops spawning.
        #[serde(default)]
        ends: Option<serde_json::Value>,
        /// Conditions required for spawning (e.g. ["SPRING", "SUMMER"]).
        #[serde(default)]
        conditions: Option<Vec<String>>,
        /// Additional spawn multiplier configuration (flexible JSON).
        #[serde(default)]
        spawn_multiplier: Option<serde_json::Value>,
        /// Result modifications applied to spawned monster (flexible JSON).
        #[serde(default)]
        result: Option<Vec<serde_json::Value>>,
    },
    /// An entry referencing a sub-group.
    Group {
        /// Sub-group ID (e.g. "GROUP_FERAL").
        group: String,
        /// Relative weight (probability) of this sub-group within the group.
        #[serde(default)]
        weight: Option<u32>,
        /// Cost multiplier for spawn cost calculations.
        #[serde(default)]
        cost_multiplier: Option<u32>,
        /// Pack size range [min, max] for spawning multiple at once.
        #[serde(default)]
        pack_size: Option<Vec<u32>>,
    },
}
