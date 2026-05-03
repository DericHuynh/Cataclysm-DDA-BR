use crate::raw_defs::cdda_types::StringOrArray;
use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vitamin cost can be a single number (most common) or a map of vitamin_id -> amount.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum VitaminCost {
    /// Simple numeric cost (e.g. `60`).
    Number(u32),
    /// Map of vitamin ID to amount (e.g. `{"vit_C": 1}`).
    Map(HashMap<String, u32>),
}

/// A mutation/trait definition from JSON type `"mutation"`.
///
/// Mutations are genetic or radiation-induced traits that can be gained or lost
/// during gameplay (e.g. "huge", "scales", "tentacles").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationDef {
    /// Unique identifier (e.g. "HUGE", "SCALES", "LEG_TENTACLES").
    pub id: DefId<MutationDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Points cost (positive = bad, negative = good).
    #[serde(default)]
    pub points: Option<i32>,

    /// Category this mutation belongs to.
    #[serde(default)]
    pub category: Vec<DefId<MutationCategoryDef>>,

    /// Prerequisite mutations (string or array).
    #[serde(default)]
    pub prereqs: StringOrArray,

    /// Mutations that cancel this one (string or array).
    #[serde(default)]
    pub cancels: StringOrArray,

    /// Mutations that conflict with this one (string or array).
    #[serde(default)]
    pub conflicts: StringOrArray,

    /// Mutations that replace this one.
    #[serde(default)]
    pub replaces: Vec<DefId<MutationDef>>,

    /// Mutations added by this one (string or array).
    #[serde(default)]
    pub adds: StringOrArray,

    /// Purifiable (can be removed with purifier).
    #[serde(default)]
    pub purifiable: Option<bool>,

    /// Threshold mutation (can't be purified).
    #[serde(default)]
    pub threshold: Option<bool>,

    /// Starting trait (available at character creation).
    #[serde(default)]
    pub starting_trait: Option<bool>,

    /// Mixed breed (can't be in same category as other mutations).
    #[serde(default)]
    pub mixed_breed: Option<bool>,

    /// Whether this mutation is valid for chargen.
    #[serde(default)]
    pub valid: Option<bool>,

    /// Flags (string or array).
    /// CDDA can use a single string like "HERBIVORE_DIET" or an array.
    #[serde(default)]
    pub flags: StringOrArray,

    /// Body part changes.
    #[serde(default)]
    pub body_part_changes: Option<Vec<MutationBodyPart>>,

    /// Active state threshold amount.
    #[serde(default)]
    pub active: Option<bool>,

    /// Cost to activate (in kcal).
    #[serde(default)]
    pub cost: Option<u32>,

    /// Cooldown turns.
    #[serde(default)]
    pub cooldown: Option<u32>,

    /// Hunger requirement.
    #[serde(default)]
    pub hunger: Option<bool>,

    /// Thirst requirement.
    #[serde(default)]
    pub thirst: Option<bool>,

    /// Fatigue requirement.
    #[serde(default)]
    pub fatigue: Option<bool>,

    /// Enchantments granted by this mutation.
    /// CDDA enchantments can be bare strings like "SQUEAKY_ANKLES" or objects.
    #[serde(default)]
    pub enchantments: Option<Vec<crate::raw_defs::cdda_types::RawValue>>,

    /// Modifies body temperature.
    #[serde(default)]
    pub bodytemp_mod: Option<[i32; 2]>,

    /// Social modifiers.
    #[serde(default)]
    pub social_mods: Option<MutationSocial>,

    /// Restricts armor on this body part.
    #[serde(default)]
    pub restricts_armor: Option<Vec<MutationBodyPartSlot>>,

    /// Allows this body part to wear items.
    #[serde(default)]
    pub allows_soft_gear: Option<Vec<String>>,

    /// Armor that is always worn.
    #[serde(default)]
    pub integrated_armor: Option<Vec<String>>,

    /// Passive pseudo items.
    #[serde(default)]
    pub passive_pseudo_items: Option<Vec<String>>,

    /// Provides item group drops.
    #[serde(default)]
    pub drops: Option<Vec<String>>,

    /// Leaks something when damaged.
    #[serde(default)]
    pub leak: Option<String>,

    /// Combat damage bonuses.
    #[serde(default)]
    pub combat_bonus: Option<MutationCombat>,

    /// Encoding of genetic material.
    #[serde(default)]
    pub encoding: Option<String>,

    /// Body part that is changed by this mutation.
    #[serde(default)]
    pub body_part: Option<String>,

    /// Type of mutation.
    #[serde(default)]
    pub mutation_type: Option<String>,

    /// Leads to (string or array).
    #[serde(default)]
    pub leads_to: Option<StringOrArray>,

    /// Prereqs2 (string or array).
    #[serde(default)]
    pub prereqs2: Option<StringOrArray>,

    /// Threshold requirement (string or array).
    #[serde(default)]
    pub threshreq: Option<StringOrArray>,

    /// Changes to mutation (string or array).
    #[serde(default)]
    pub changes_to: Option<StringOrArray>,

    /// Ugliness (can be negative for beauty)
    #[serde(default)]
    pub ugliness: Option<i32>,

    /// Visibility (can be negative for stealth)
    #[serde(default)]
    pub visibility: Option<i32>,

    /// Vitamin cost (number or map).
    #[serde(default)]
    pub vitamin_cost: Option<VitaminCost>,

    /// Types array (string or array).
    #[serde(default)]
    pub types: Option<StringOrArray>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// Body part change from a mutation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationBodyPart {
    /// Body part ID.
    pub part: String,
    /// New type.
    #[serde(default)]
    pub new_type: Option<String>,
    /// New hp.
    #[serde(default)]
    pub hp: Option<u32>,
    /// New encumbrance.
    #[serde(default)]
    pub encumbrance: Option<u32>,
    /// New armor.
    #[serde(default)]
    pub armor: Option<u32>,
    /// New coverage.
    #[serde(default)]
    pub coverage: Option<u32>,
}

/// Social modifiers from a mutation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationSocial {
    #[serde(default)]
    pub intimidation: Option<i32>,
    #[serde(default)]
    pub persuasion: Option<i32>,
    #[serde(default)]
    pub lie: Option<i32>,
}

/// Body part slot restriction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationBodyPartSlot {
    /// Body part ID.
    pub part: String,
    /// Allow items of this type.
    pub allowed: Option<bool>,
}

/// Combat bonuses from a mutation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationCombat {
    /// Melee damage bonus.
    #[serde(default)]
    pub melee_damage: Option<cdda_core::damage::Damage>,
    /// Melee attack bonus.
    #[serde(default)]
    pub attack: Option<u32>,
    /// Melee defense bonus.
    #[serde(default)]
    pub defense: Option<u32>,
    /// Dodge bonus.
    #[serde(default)]
    pub dodge: Option<i32>,
    /// Move cost modifier.
    #[serde(default)]
    pub move_cost: Option<i32>,
}

/// Mutation category definition from JSON type `"mutation_category"`.
///
/// Defines a group/category of mutations (e.g. "LIZARD", "BEAST", "PLANT").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MutationCategoryDef {
    /// Unique identifier (e.g. "LIZARD", "BEAST", "PLANT").
    pub id: DefId<MutationCategoryDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Display name for the threshold.
    pub threshold_name: Option<LocalizedString>,

    /// Description.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Display category for UI.
    #[serde(default)]
    pub category: Option<String>,

    /// Mutagen item type.
    #[serde(default)]
    pub mutagen: Option<DefId<crate::raw_defs::item::ItemDef>>,

    /// Mutagen item type (group).
    #[serde(default)]
    pub mutagen_group: Option<String>,

    /// Ivy poison item from this category.
    #[serde(default)]
    pub iv: Option<String>,

    /// Mutagenic liquid item.
    #[serde(default)]
    pub mutagenic: Option<String>,

    /// Blood analysis message.
    #[serde(default)]
    pub memorial_message: Option<String>,

    /// Preferred body part for mutations.
    #[serde(default)]
    pub preferred_part: Option<String>,
}

/// A trait group definition from JSON type `"trait_group"`.
///
/// Defines weighted groups of traits/mutations for random selection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraitGroupDef {
    /// Unique identifier.
    pub id: DefId<TraitGroupDef>,

    /// Subtype: distribution or collection.
    #[serde(default)]
    pub subtype: String,

    /// Entries (subtype: "distribution" or "collection").
    #[serde(default)]
    pub entries: Vec<TraitGroupEntry>,

    /// Alternative entries format (array of trait/group objects).
    #[serde(default)]
    pub traits: Option<Vec<TraitGroupEntry>>,
}

/// An entry in a trait group.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TraitGroupEntry {
    /// Simple: "trait_id"
    Simple(String),
    /// Object with probability and optional trait field.
    Obj {
        /// Trait/mutation ID.
        #[serde(rename = "trait")]
        trait_: String,
        /// Probability weight (default 100).
        #[serde(default = "default_prob")]
        prob: u32,
    },
    /// Object with group reference (subgroups).
    Group {
        /// Reference to another trait group.
        group: String,
        /// Probability weight.
        #[serde(default = "default_prob")]
        prob: u32,
    },
    /// Object with collection of traits.
    Collection {
        /// Sub-entries.
        #[serde(default)]
        collection: Vec<TraitGroupEntry>,
        /// Probability weight.
        #[serde(default = "default_prob")]
        prob: u32,
    },
    /// Object with distribution of traits.
    Distribution {
        /// Sub-entries.
        #[serde(default)]
        distribution: Vec<TraitGroupEntry>,
        /// Probability weight.
        #[serde(default = "default_prob")]
        prob: u32,
    },
}

fn default_prob() -> u32 {
    100
}
