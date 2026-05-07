//! ECS components for items, inventory, containers, pockets.
//!
//! Extracted from `cdda_sim::components` to its own crate.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

// ===========================================================================
// Item identity
// ===========================================================================

/// Numeric index of the definition entity this runtime item was spawned from.
/// Used by `merge_or_stack` to compare items without needing the string `DefStrId`.
#[derive(Component, Debug, Clone, Copy)]
pub struct DefOrigin(pub u32);

// ===========================================================================
// Item state
// ===========================================================================

#[derive(Component, Debug, Clone, Copy)]
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

#[derive(Component, Debug, Clone, Copy)]
pub struct CurrentCharges(pub i32);

impl Default for CurrentCharges {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct LoadedAmmo(pub i32);

impl Default for LoadedAmmo {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Component, Debug, Clone)]
pub struct Spoilable {
    pub rotten: cdda_core::ItemId,
    pub total: cdda_core::Time,
    pub remaining: cdda_core::Time,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemDamage(pub u32);

// ===========================================================================
// Container tags (zero-sized)
// ===========================================================================

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Sealed;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Rigid;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Watertight;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PreservesTemp;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Fireproof;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct GasTight;

// ===========================================================================
// Relationships — inventory, equipment, attachments
// ===========================================================================

// -- Containment ------------------------------------------------------------

#[derive(Component)]
#[relationship(relationship_target = ContainerContents)]
pub struct InsideContainer(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = InsideContainer, linked_spawn)]
pub struct ContainerContents(Vec<Entity>);

impl ContainerContents {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Wielding ---------------------------------------------------------------

#[derive(Component)]
#[relationship(relationship_target = WieldedItems)]
pub struct WieldedBy(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = WieldedBy, linked_spawn)]
pub struct WieldedItems(Vec<Entity>);

impl WieldedItems {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Wearing ----------------------------------------------------------------

#[derive(Component)]
#[relationship(relationship_target = WornBy)]
pub struct WornOn {
    #[relationship]
    pub wearer: Entity,
    pub slot: Option<String>,
}

#[derive(Component)]
#[relationship_target(relationship = WornOn, linked_spawn)]
pub struct WornBy(Vec<Entity>);

impl WornBy {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Pocket attachment ------------------------------------------------------

#[derive(Component)]
#[relationship(relationship_target = MountedPockets)]
pub struct MountedOn(pub Entity);

#[derive(Component)]
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

#[derive(Component, Debug, Clone)]
pub struct Pocket {
    pub max_volume: cdda_core::Volume,
    pub max_weight: cdda_core::Weight,
    pub max_item_length: cdda_core::Length,
    pub min_item_volume: cdda_core::Volume,
    pub pocket_type: PocketType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PocketType {
    Container,
    Magazine,
    MagazineWell,
    Holster,
    Special,
}

#[derive(Component, Debug, Clone)]
pub struct PocketRestriction {
    pub allowed_flags: Vec<String>,
    pub allowed_items: Vec<cdda_core::ItemId>,
    pub ammo_type: Option<String>,
    pub item_category: Option<String>,
    pub max_item_volume: cdda_core::Volume,
}

#[derive(Component, Debug, Clone)]
pub struct AttachmentSlot {
    pub slot_type: AttachmentType,
    pub max_volume: cdda_core::Volume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentType {
    Molle,
    Belt,
    Clip,
    Velcro,
    Universal,
}

// ===========================================================================
// Container entity
// ===========================================================================

#[derive(Component, Debug, Clone)]
pub struct Container {
    pub capacity: cdda_core::Volume,
}
