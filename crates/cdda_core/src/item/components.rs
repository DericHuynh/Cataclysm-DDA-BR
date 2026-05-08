//! ECS components for items, inventory, containers, pockets.
//!
//! Extracted from `crate::sim::components` to its own crate.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_reflect::Reflect;

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
// Display / render hints
// ===========================================================================

/// CDDA type-string ID used for tileset sprite lookup.
///
/// Added to items that have a known CDDA type. Distinct from `DefStrId`
/// which lives on definition entities only.
///
/// The render crate queries this to find the right `TileInfo` in
/// `TileRegistry`. When no tile is registered for the ID renders fall
/// back to `crate::sim::def_components::ItemSymbol` if present.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ItemTypeId(pub String);
