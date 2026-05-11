//! ECS components for items, inventory, containers, pockets.
//!
//! Extracted from `crate::sim::components` to its own crate.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Query, Resource};
use bevy_reflect::Reflect;
use std::collections::{HashMap, HashSet};

// ===========================================================================
// Item identity
// ===========================================================================

/// Numeric index of the definition entity this runtime item was spawned from.
/// Used by `merge_or_stack` to compare items without needing the string `DefStrId`.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct DefOrigin(pub u32);

// ===========================================================================
// Item state
// ===========================================================================

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct StackCount(u32);

impl StackCount {
    pub fn new(n: u32) -> Self {
        assert!(n >= 1, "StackCount must be >= 1");
        Self(n)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct CurrentCharges(pub i32);

impl Default for CurrentCharges {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct LoadedAmmo(pub i32);

impl Default for LoadedAmmo {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct Spoilable {
    pub rotten: crate::ItemId,
    pub total: crate::Time,
    pub remaining: crate::Time,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct ItemDamage(pub u32);

// ===========================================================================
// Container tags (zero-sized)
// ===========================================================================

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Sealed;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Rigid;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Watertight;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct PreservesTemp;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Fireproof;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct GasTight;

// ===========================================================================
// Relationships — inventory, equipment, attachments
// ===========================================================================

// -- Containment ------------------------------------------------------------

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = ContainerContents)]
pub struct InsideContainer(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = InsideContainer, linked_spawn)]
pub struct ContainerContents(Vec<Entity>);

impl ContainerContents {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Wielding ---------------------------------------------------------------

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = WieldedItems)]
pub struct WieldedBy(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = WieldedBy, linked_spawn)]
pub struct WieldedItems(Vec<Entity>);

impl WieldedItems {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Wearing ----------------------------------------------------------------

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = WornBy)]
pub struct WornOn {
    #[relationship]
    pub wearer: Entity,
    pub slot: Option<String>,
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = WornOn, linked_spawn)]
pub struct WornBy(Vec<Entity>);

impl WornBy {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Pocket attachment ------------------------------------------------------

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = MountedPockets)]
pub struct MountedOn(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = MountedOn, linked_spawn)]
pub struct MountedPockets(Vec<Entity>);

impl MountedPockets {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Pocket identity --------------------------------------------------------

/// Marker placed on every pocket entity.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct IsPocket;

/// On a pocket entity: the character who owns this pocket.
///
/// This is intentionally a **one-way** component (not a bidirectional
/// relationship).  The reverse lookup (finding all pockets for a creature)
/// uses `MountedPockets` instead.  Keeping `PocketOf` one-way avoids the
/// complexity of synchronizing two relationship halves for a simple
/// ownership pointer.
///
/// Follows the chain: item → InsideContainer(pocket) → PocketOf(player).
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct PocketOf(pub Entity);

// ===========================================================================
// Pocket system
// ===========================================================================

#[derive(Component, Debug, Clone, Reflect)]
pub struct Pocket {
    pub max_volume: crate::Volume,
    pub max_weight: crate::Weight,
    pub max_item_length: crate::Length,
    pub min_item_volume: crate::Volume,
    #[reflect(ignore)]
    pub pocket_type: PocketType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum PocketType {
    #[default]
    Container,
    Magazine,
    MagazineWell,
    Holster,
    Special,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct PocketRestriction {
    pub allowed_flags: Vec<String>,
    #[reflect(ignore)]
    pub allowed_items: Vec<crate::ItemId>,
    pub ammo_type: Option<String>,
    pub item_category: Option<String>,
    pub max_item_volume: crate::Volume,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct AttachmentSlot {
    #[reflect(ignore)]
    pub slot_type: AttachmentType,
    pub max_volume: crate::Volume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum AttachmentType {
    #[default]
    Molle,
    Belt,
    Clip,
    Velcro,
    Universal,
}

// ===========================================================================
// Container entity
// ===========================================================================

#[derive(Component, Debug, Clone, Reflect)]
pub struct Container {
    pub capacity: crate::Volume,
}

// ===========================================================================
/// Display / render hints

/// CDDA type-string ID used for tileset sprite lookup.
///
/// Added to items that have a known CDDA type. Distinct from `DefStrId`
/// which lives on definition entities only.
///
/// The render crate queries this to find the right `TileInfo` in
/// `TileRegistry`. When no tile is registered for the ID renders fall
/// back to `crate::core::components::def::ItemSymbol` if present.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ItemTypeId(pub String);

// ===========================================================================
// Tool qualities on runtime items
// ===========================================================================

/// Tool qualities present on a runtime item entity.
///
/// Populated during def-to-runtime cloning: `build_def_world` inserts this
/// on the def entity from the JSON `qualities` field, and `EntityCloner`
/// carries it forward to runtime spawns.
///
/// Each entry is `(quality_id, level)` — e.g. `("CUT", 2)`.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ItemQualities(pub Vec<(String, i32)>);

impl ItemQualities {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ===========================================================================
// Inventory system components and resources
// ===========================================================================

/// Hard cap on total item volume (mL) that may rest on one floor tile.
pub const FLOOR_CAP_ML: u32 = 400_000;

/// The set of characters available for inventory-letter assignment.
/// 62 chars: a-z, A-Z, 0-9.
pub const INVLET_CHARS: &[char; 62] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

/// Assigned inventory letter on an item entity.
///
/// Present only while the item is in a creature's inventory.
/// Removed on drop / transfer out of inventory.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct Invlet(pub char);

// ===========================================================================
// InvletFavorites — per-def-origin invlet preferences
// ===========================================================================

/// Stores the player's preferred inventory letters per item type.
///
/// When an item of a given `DefOrigin` is picked up, the system tries
/// to assign one of the favourite invlets for that type.
#[derive(Component, Debug, Clone, Reflect)]
pub struct InvletFavorites {
    favorites: HashMap<u32, HashSet<char>>,
}

impl Default for InvletFavorites {
    fn default() -> Self {
        Self {
            favorites: HashMap::new(),
        }
    }
}

impl InvletFavorites {
    /// Record that `invlet` is a preferred letter for items of `def_origin`.
    pub fn set(&mut self, def_origin: u32, invlet: char) {
        self.favorites.entry(def_origin).or_default().insert(invlet);
    }

    /// Forget `invlet` as a preferred letter for items of `def_origin`.
    pub fn erase(&mut self, def_origin: u32, invlet: char) {
        if let Some(set) = self.favorites.get_mut(&def_origin) {
            set.remove(&invlet);
        }
    }

    /// All favourite invlet characters for this definition.
    pub fn invlets_for(&self, def_origin: u32) -> Vec<char> {
        self.favorites
            .get(&def_origin)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }
}

// ===========================================================================
// Inventory component
// ===========================================================================

/// Per-creature inventory state.
///
/// Tracks invlet → entity mappings and pending invlet assignments.
/// **Item ownership** is expressed via the `InsideContainer(creature)`
/// relationship, not through the `items` Vec.
///
/// # Query patterns
///
/// To iterate all items in a creature's inventory:
/// ```ignore
/// fn system(
///     creature: Entity,
///     contents: Query<&ContainerContents>,
///     items: Query<&StackCount>,
/// ) {
///     if let Ok(cc) = contents.get(creature) {
///         for item_entity in cc.iter() {
///             let count = items.get(item_entity).map(|s| s.get()).unwrap_or(1);
///         }
///     }
/// }
/// ```
#[derive(Component, Debug, Clone, Reflect)]
pub struct Inventory {
    /// invlet character → item entity in this inventory.
    pub invlets: HashMap<char, Entity>,
    /// Entities that have been added but not yet assigned an invlet.
    pub needs_invlet: HashSet<Entity>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            invlets: HashMap::new(),
            needs_invlet: HashSet::new(),
        }
    }
}

impl Inventory {
    /// Number of items tracked in this inventory.
    pub fn len(&self) -> usize {
        self.invlets.len() + self.needs_invlet.len()
    }

    /// True when no items are in the inventory.
    pub fn is_empty(&self) -> bool {
        self.invlets.is_empty() && self.needs_invlet.is_empty()
    }

    /// All item entities currently in this inventory.
    pub fn item_entities(&self) -> Vec<Entity> {
        let mut v: Vec<Entity> = self.invlets.values().copied().collect();
        v.extend(self.needs_invlet.iter().copied());
        v
    }

    /// Queue `item` for invlet assignment on the next `assign_invlets_system` run.
    pub fn mark_needs_invlet(&mut self, item: Entity) {
        self.needs_invlet.insert(item);
    }

    /// Find an unassigned invlet character, or None if all are taken.
    pub fn allocate_invlet(&self) -> Option<char> {
        INVLET_CHARS
            .iter()
            .copied()
            .find(|c| !self.invlets.contains_key(c))
    }
}

// ===========================================================================
// InventoryBin — cached item-type lookup
// ===========================================================================

/// Cached bins of inventory items keyed by `DefOrigin`.
///
/// Built by `build_inventory_bins` each frame. Provides fast `count_of`
/// and `charges_of` queries without iterating the entire inventory.
///
/// In CDDA-master this is the `itype_bin` inside `inventory`.
#[derive(Debug, Clone, Default, Resource)]
pub struct InventoryBin {
    /// `DefOrigin.0` → list of item entities of that type.
    pub bins: HashMap<u32, Vec<Entity>>,
}

impl InventoryBin {
    /// Total stack count for items of this definition origin.
    pub fn count_of(&self, def_origin: u32, counts: &Query<&StackCount>) -> u32 {
        self.bins.get(&def_origin).map_or(0, |entities| {
            entities
                .iter()
                .map(|e| counts.get(*e).map(|s| s.get()).unwrap_or(1))
                .sum()
        })
    }

    /// Total charges across all items of this definition origin.
    pub fn charges_of(&self, def_origin: u32, charges: &Query<&CurrentCharges>) -> i32 {
        self.bins.get(&def_origin).map_or(0, |entities| {
            entities
                .iter()
                .map(|e| charges.get(*e).map(|c| c.0).unwrap_or(0))
                .sum()
        })
    }

    /// Checks whether the inventory has at least `qty` items of the given origin.
    pub fn has_amount(&self, def_origin: u32, qty: u32, counts: &Query<&StackCount>) -> bool {
        self.count_of(def_origin, counts) >= qty
    }

    /// Checks whether the inventory has at least `qty` charges of the given origin.
    pub fn has_charges(&self, def_origin: u32, qty: i32, charges: &Query<&CurrentCharges>) -> bool {
        self.charges_of(def_origin, charges) >= qty
    }
}

// ===========================================================================
// InventoryFocus — focused row in the inventory screen
// ===========================================================================

/// Tracks which item row (by sorted position) is focused in the inventory screen.
///
/// `panel`: 0 = pocket list (left), 1 = wielded panel (top-right).
/// Written by `inventory_screen_input`, read by `cdda_render` to highlight rows.
#[derive(Resource, Debug, Clone, Default)]
pub struct InventoryFocus {
    pub index: usize,
    pub panel: usize,
}

// ===========================================================================
// InProgressCraft — partially-crafted item entity
// ===========================================================================

/// Marks a partially-completed craft.
///
/// Spawned instead of the final item when the player starts a recipe.
/// Components are consumed immediately; the result item is withheld until
/// `ap_spent >= ap_total`. The entity lives in the player's inventory
/// (`InsideContainer`) or on the ground (`WorldPosition`) — picking it up
/// resumes crafting automatically each turn.
#[derive(Component, Debug, Clone, Reflect)]
pub struct InProgressCraft {
    /// The recipe definition entity (read-only after construction).
    pub recipe_entity: Entity,
    /// String ID of the result item def.
    pub result_id: String,
    /// Display name of the result item.
    pub result_name: String,
    /// How many of the result will be produced.
    pub result_count: u32,
    /// Total AP required at speed=100 (= `recipe_turns * 100`).
    pub ap_total: i32,
    /// AP already invested in this craft.
    pub ap_spent: i32,
}

impl InProgressCraft {
    pub fn is_complete(&self) -> bool {
        self.ap_spent >= self.ap_total
    }

    pub fn progress_pct(&self) -> u32 {
        if self.ap_total == 0 {
            return 100;
        }
        ((self.ap_spent as f32 / self.ap_total as f32) * 100.0).min(100.0) as u32
    }

    pub fn display_name(&self) -> String {
        format!("[crafting] {} ({}%)", self.result_name, self.progress_pct())
    }
}
