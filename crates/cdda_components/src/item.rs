//! ECS components for items, inventory, containers, pockets.
//!
//! Extracted from `crate::sim::components` to its own crate.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_reflect::Reflect;

// ===========================================================================
// Item identity
// ===========================================================================

/// Session-local item-type token, matching ItemTypeRegistry for native spawns.
/// Legacy callers may supply opaque values. Never serialize it as a definition
/// identity; native persistence must resolve the stable item key.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct DefOrigin(pub u32);

// ===========================================================================
// Item state
// ===========================================================================

/// Stack count for items that can be stacked.
///
/// # Zero contract
/// A `StackCount(0)` is meaningless — the entity should be despawned
/// rather than kept around with a zero count. Use `new()` which rejects
/// zero values, and when subtracting results in zero, despawn the entity.
///
/// The inner field is private — create via `StackCount::new()` or by
/// deserializing the component directly (for save/load).
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct StackCount(u32);

impl StackCount {
    /// Create a new `StackCount` with the given value.
    ///
    /// Returns `Err` if `n == 0` (zero-count entities should be despawned,
    /// not kept alive).
    pub fn new(n: u32) -> Result<Self, &'static str> {
        if n == 0 {
            Err("StackCount must be >= 1, zero-count entities should be despawned")
        } else {
            Ok(Self(n))
        }
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

/// Exterior volume excludes contents; nested contents still contribute weight.
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

// ===========================================================================
// Pocket system
// ===========================================================================

/// Total capacity limits. The inventory boundary validates projected stack and
/// nested contents loads; this data component alone does not enforce placement.
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
    /// Flag indices (from `ItemFlagRegistry`) that are allowed in this pocket.
    /// Items must have at least one of these flags set to be placed in the pocket.
    pub allowed_flags: Vec<u16>,
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

/// CDDA type-string ID used for tileset sprite lookup and crafting matching.
///
/// Added to items that have a known CDDA type. Interned via `ItemTypeRegistry`.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct ItemType(pub crate::tokens::ItemTypeId);

// ===========================================================================
// Tool qualities on runtime items
// ===========================================================================

/// Interned identifier for a tool quality (e.g. "CUT", "BOIL", "HAMMER").
///
/// Stored in `ItemQualities` and `RecipeQualities` components instead of
/// the raw string.  Resolve back to a string via `QualityRegistry::resolve()`
/// (in the `cdda_data` crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct QualityId(pub u16);

/// Tool qualities present on a runtime item entity.
///
/// Populated during def-to-runtime cloning: `build_def_world` interns
/// quality names via `QualityRegistry` and stores the resulting IDs.
///
/// Each entry is `(QualityId, level)` — e.g. `(QualityId(3), 2)` for "CUT" at level 2.
/// Resolve back to a string via `QualityRegistry::resolve()` for display.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ItemQualities(pub Vec<(QualityId, i32)>);

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
