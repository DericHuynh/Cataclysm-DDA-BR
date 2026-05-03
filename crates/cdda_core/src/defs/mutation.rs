use crate::types::{DefId, LocalizedString};
use serde::{Deserialize, Serialize};

/// A mutation/trait definition from JSON type `"mutation"`.
///
/// Mutations are genetic or radiation-induced traits that can be gained or lost
/// during gameplay (e.g. "huge", "scales", "tentacles").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationDef {
    /// Unique identifier (e.g. "HUGE", "SCALES", "LEG_TENTACLES").
    pub id: DefId<MutationDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    pub description: LocalizedString,

    /// Points cost (positive = bad, negative = good).
    #[serde(default)]
    pub points: Option<i32>,

    /// Category this mutation belongs to.
    #[serde(default)]
    pub category: Vec<DefId<MutationCategoryDef>>,

    /// Prerequisite mutations.
    #[serde(default)]
    pub prereqs: Vec<String>,

    /// Mutations that cancel this one.
    #[serde(default)]
    pub cancels: Vec<String>,

    /// Mutations that conflict with this one.
    #[serde(default)]
    pub conflicts: Vec<String>,

    /// Mutations that replace this one.
    #[serde(default)]
    pub replaces: Vec<DefId<MutationDef>>,

    /// Mutations added by this one.
    #[serde(default)]
    pub adds: Vec<String>,

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

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

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
    #[serde(default)]
    pub enchantments: Option<Vec<serde_json::Value>>,

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

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Leads to
    #[serde(default)]
    pub leads_to: Option<Vec<String>>,

    /// Prereqs2
    #[serde(default)]
    pub prereqs2: Option<Vec<String>>,

    /// Threshold requirement
    #[serde(default)]
    pub threshreq: Option<Vec<String>>,

    /// Changer to mutation
    #[serde(default)]
    pub changes_to: Option<Vec<String>>,

    /// Ugliness
    #[serde(default)]
    pub ugliness: Option<u32>,

    /// Visibility
    #[serde(default)]
    pub visibility: Option<u32>,

    /// Vitamin cost
    #[serde(default)]
    pub vitamin_cost: Option<serde_json::Value>,

    /// Types array
    #[serde(default)]
    pub types: Option<Vec<String>>,

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

/// Body part change from a mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationSocial {
    #[serde(default)]
    pub intimidation: Option<i32>,
    #[serde(default)]
    pub persuasion: Option<i32>,
    #[serde(default)]
    pub lie: Option<i32>,
}

/// Body part slot restriction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationBodyPartSlot {
    /// Body part ID.
    pub part: String,
    /// Allow items of this type.
    pub allowed: Option<bool>,
}

/// Combat bonuses from a mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationCombat {
    /// Melee damage bonus.
    #[serde(default)]
    pub melee_damage: Option<crate::damage::Damage>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub mutagen: Option<DefId<crate::defs::item::ItemDef>>,

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitGroupDef {
    /// Unique identifier.
    pub id: DefId<TraitGroupDef>,

    /// Subtype: distribution or collection.
    #[serde(default)]
    pub subtype: String,

    /// Entries.
    #[serde(default)]
    pub entries: Vec<TraitGroupEntry>,

    /// Alternative entries format.
    #[serde(default)]
    pub traits: Option<Vec<serde_json::Value>>,
}

/// An entry in a trait group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TraitGroupEntry {
    /// Simple: "trait_id"
    Simple(String),
    /// Object with probability.
    Obj {
        trait_: String,
        #[serde(default = "default_prob")]
        prob: u32,
    },
}

fn default_prob() -> u32 {
    100
}
