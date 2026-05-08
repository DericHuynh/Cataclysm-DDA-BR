//! Equipment system — wielding, wearing, and managing items on a creature.
//!
//! Uses relationships (`WieldedBy`/`WieldedItems`, `WornOn`/`WornBy`) for
//! bidirectional tracking of equipped items. Mutations go through
//! `commands.insert()` so that Bevy hooks fire and keep the relationship
//! target in sync — never query as `&mut` and modify the entity field.

use bevy_ecs::prelude::*;
use crate::actor::components::*;
use crate::item::components::*;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during equipment operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquipError {
    /// Creature is already wielding something.
    AlreadyWielding(Entity),
    /// Creature has no free hands to wield a two-handed item.
    NoFreeHands,
    /// A worn item already occupies this slot.
    SlotOccupied(String),
    /// Item exceeds creature's carry capacity.
    ItemTooHeavy,
    /// Item is too large for the available space.
    ItemTooLarge,
    /// Item cannot be equipped (missing required flags/structure).
    NotEquippable,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Wield an item into a creature's active hand slot.
///
/// Removes `InsideContainer` (if stored), inserts `WieldedBy(creature)`.
/// Returns `Err` if the creature already wields something or the item
/// cannot be wielded.
pub fn wield_item(
    world: &mut World,
    creature: Entity,
    item: Entity,
) -> Result<(), EquipError> {
    let _ = (world, creature, item);
    todo!("wield item logic: check WieldedItems, remove InsideContainer, insert WieldedBy")
}

/// Unwield the creature's currently held item.
///
/// Removes `WieldedBy`, inserts `InsideContainer(creature)` or
/// `WorldPosition(ground_pos)` as fallback.
/// Returns the unwielded item entity on success.
pub fn unwield(world: &mut World, creature: Entity) -> Result<Entity, EquipError> {
    let _ = (world, creature);
    todo!("unwield logic: find wielded item, remove WieldedBy, return entity")
}

/// Wear an item on a specific equipment slot (or auto-assign a slot).
///
/// Inserts `WornOn { wearer: creature, slot }`.
/// Returns `Err` if slot is occupied or item is not wearable.
pub fn wear_item(
    world: &mut World,
    creature: Entity,
    item: Entity,
    slot: Option<String>,
) -> Result<(), EquipError> {
    let _ = (world, creature, item, slot);
    todo!("wear item on slot: check WornBy, validate slot, insert WornOn")
}

/// Take off a worn item.
///
/// Removes `WornOn`, inserts `InsideContainer(creature)` or drops to ground.
pub fn take_off(
    world: &mut World,
    creature: Entity,
    item: Entity,
) -> Result<(), EquipError> {
    let _ = (world, creature, item);
    todo!("remove worn item: find WornOn, remove it, insert InsideContainer")
}

/// List all available (unoccupied) equipment slots for a creature.
///
/// Examines the creature's body parts and compares against currently
/// worn items to determine which slots are free.
pub fn available_slots(world: &World, creature: Entity) -> Vec<String> {
    let _ = (world, creature);
    todo!("list equipment slots: query CreatureBodyParts, filter by worn items")
}
