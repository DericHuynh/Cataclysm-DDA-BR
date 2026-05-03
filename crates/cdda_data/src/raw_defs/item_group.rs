use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An item group definition from JSON type `"item_group"`.
///
/// Item groups define loot tables: probability-weighted lists of items
/// (or nested groups) that spawn in containers, on monsters, or in mapgen.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemGroupDef {
    /// Unique identifier.
    pub id: DefId<ItemGroupDef>,

    /// Subtype: "distribution" (pick one) or "collection" (pick all).
    #[serde(default = "default_subtype")]
    pub subtype: ItemGroupSubtype,

    /// Whether contained items should have ammo loaded.
    #[serde(default)]
    pub magazine: Option<u32>,

    /// Whether contained items should have ammo loaded.
    #[serde(default)]
    pub ammo: Option<u32>,

    /// Container item type for all entries.
    #[serde(default)]
    pub container: Option<DefId<crate::raw_defs::item::ItemDef>>,

    /// Whether items are sealed in their container.
    #[serde(default)]
    pub sealed: Option<bool>,

    /// Whether this group uses charges.
    #[serde(default)]
    pub charges: Option<u32>,

    /// With ammunition probability.
    #[serde(default)]
    pub with_ammo: Option<u32>,

    /// Entries in this item group.
    #[serde(default)]
    pub entries: Vec<ItemGroupEntry>,

    /// Items (alternative to entries, simpler format).
    #[serde(default)]
    pub items: Option<Vec<serde_json::Value>>,

    /// Groups (alternative to entries for nested groups).
    #[serde(default)]
    pub groups: Option<Vec<serde_json::Value>>,

    /// On overflow behavior
    #[serde(default)]
    pub on_overflow: Option<String>,

    /// Container item
    #[serde(default)]
    pub container_item: Option<String>,
}

fn default_subtype() -> ItemGroupSubtype {
    ItemGroupSubtype::Distribution
}

/// Whether the group uses "distribution" (pick one) or "collection" (pick all) semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ItemGroupSubtype {
    /// Pick one entry (weighted random).
    #[serde(rename = "distribution")]
    Distribution,
    /// Pick all entries.
    #[serde(rename = "collection")]
    Collection,
}

/// An entry in an item group.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ItemGroupEntry {
    /// Simple item reference.
    Item {
        /// Item type ID.
        item: String,
        /// Probability weight (default 100).
        #[serde(default = "default_prob")]
        prob: u32,
        /// Count range [min, max].
        #[serde(default)]
        count: Option<[u32; 2]>,
        /// Charges.
        #[serde(default)]
        charges: Option<[u32; 2]>,
        /// Whether contained.
        #[serde(default)]
        container: Option<String>,
        /// Whether sealed.
        #[serde(default)]
        sealed: Option<bool>,
        /// Whether item is custom-damaged.
        #[serde(default)]
        damage: Option<u32>,
        /// Whether item is custom-damaged.
        #[serde(default)]
        variant: Option<String>,
        /// Custom flags.
        #[serde(default)]
        flags: Option<Vec<String>>,
        /// Contents item group.
        #[serde(default)]
        contents_group: Option<String>,
    },
    /// Nested group reference.
    Group {
        /// Group ID.
        group: String,
        /// Probability weight.
        #[serde(default = "default_prob")]
        prob: u32,
        /// Count range.
        #[serde(default)]
        count: Option<[u32; 2]>,
    },
    /// Distribution of possible entries.
    Distribution {
        /// Distribution entries.
        distribution: Vec<ItemGroupEntry>,
        /// Probability weight.
        #[serde(default = "default_prob")]
        prob: u32,
    },
    /// Fallback for any unrecognized entry format.
    Other(serde_json::Value),
}

fn default_prob() -> u32 {
    100
}
