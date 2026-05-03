//! # Item-group templates
//!
//! Blueprint types for item-group definitions — weighted collections of items
//! (or nested groups) used for loot drops, shop inventories, and spawn tables.

use crate::id::*;

/// How an item group's entries are selected.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemGroupSubtype {
    /// Each entry is rolled independently (each entry can appear).
    Distribution,
    /// Exactly one entry is selected from the weighted pool.
    Collection,
}

/// A single entry in an item group — either a direct item reference or a
/// nested group reference, each with a weight.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemGroupEntry {
    /// Reference to an item with a weight.
    Item(ItemId, u32),
    /// Reference to another item group with a weight.
    Group(ItemGroupId, u32),
}

/// The blueprint for an item-group definition.
///
/// Item groups are the primary mechanism for randomised loot in CDDA: death
/// drops, shop inventories, map-spawned items, and more all use them.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemGroupTemplate {
    /// String identifier (e.g. `"hospital_beds"`).
    pub id: ItemGroupId,
    /// Human-readable name.
    pub name: String,
    /// Whether entries are selected via distribution or collection.
    pub subtype: ItemGroupSubtype,
    /// The weighted entries in this group.
    pub entries: Vec<ItemGroupEntry>,
}
