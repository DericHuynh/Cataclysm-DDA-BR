//! Spawning phase — spawn new entities from SpawnEvents.
//!
//! Processes `SpawnEvent` messages to create monster and item entities
//! from definition templates. Delegates to `spawning_impl` for the
//! actual `EntityCloner`-based instantiation.
//!
//! # Design note
//!
//! These functions take a `def_entity: Entity` directly rather than an ID.
//! The caller (event processing or tests) is responsible for resolving
//! MonsterId/ItemId to definition entities via the `DefinitionWorld`.
//! This keeps the spawning functions ID-agnostic and directly testable
//! with manually constructed definition entities.

use crate::components::*;
use bevy_ecs::prelude::*;
use cdda_core::coords::WorldPos;
use cdda_core::{FactionId, ItemGroupId};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Spawn a monster from a definition entity at the given position.
///
/// Delegates to `spawning_impl::spawn_creature_from_def` which uses
/// `EntityCloner` to clone the def entity and inserts runtime
/// components: `IsAlive`, `Solid`, `WorldPosition`, `Health`, `Faction`,
/// and initial `MovePoints`.
pub fn spawn_monster(
    world: &mut World,
    def_entity: Entity,
    position: WorldPos,
    faction: FactionId,
) -> Entity {
    super::spawning_impl::spawn_creature_from_def(world, def_entity, position, faction)
}

/// Spawn an item from a definition entity at the given position.
///
/// Delegates to `spawning_impl::spawn_item_from_def` which uses
/// `EntityCloner` to clone the def entity and inserts `WorldPosition`,
/// `StackCount(count)`, and `CurrentCharges(0)`.
pub fn spawn_item(world: &mut World, def_entity: Entity, position: WorldPos, count: u32) -> Entity {
    super::spawning_impl::spawn_item_from_def(world, def_entity, position, count)
}

/// Spawn one or more items from an item group definition.
///
/// Item groups are weighted lists of items. The group definition
/// determines how many items to spawn and which templates to use.
/// Returns the list of spawned entity IDs.
///
/// STUB: Not yet implemented.
pub fn spawn_from_group(
    world: &mut World,
    group_id: ItemGroupId,
    position: WorldPos,
) -> Vec<Entity> {
    let _ = (world, group_id, position);
    todo!("spawn from item group: lookup group in DefinitionWorld, roll weighted items, spawn each via spawn_item")
}

/// Spawning phase — process all queued `SpawnEvent` messages.
///
/// Reads the `SpawnEvent` message buffer, resolves each event's
/// template_id to a def entity, and calls `spawn_monster` / `spawn_item`.
///
/// STUB: no-op until spawning implemented
pub fn spawning_phase(world: &mut World) {
    let _ = world;
}
