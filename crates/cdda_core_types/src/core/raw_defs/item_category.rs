use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An item category definition from JSON type `"ITEM_CATEGORY"`.
///
/// Defines a category for organizing items in inventory screens and
/// sorting loot (e.g. weapons, ammo, food, tools).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemCategoryDef {
    /// Unique identifier (e.g. "CC_WEAPON", "ammo", "food").
    pub id: DefId<ItemCategoryDef>,

    /// Header name displayed in the inventory category listing.
    #[serde(default)]
    pub name_header: Option<LocalizedString>,

    /// Noun name used for referring to a single item of this category.
    #[serde(default)]
    pub name_noun: Option<LocalizedString>,

    /// Sort rank for ordering categories in lists.
    #[serde(default)]
    pub sort_rank: Option<i32>,

    /// Whether items in this category are grouped under a default subcategory.
    #[serde(default)]
    pub group_under_default: Option<bool>,

    /// Priority for sorting within the category.
    #[serde(default)]
    pub priority: Option<i32>,
}
