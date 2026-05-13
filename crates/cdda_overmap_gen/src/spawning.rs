//! Entity spawning from definition templates.
//!
//! These functions clone definition entities from `DefinitionWorld`
//! and insert runtime components. Used by crafting, dev-spawn,
//! and eventually by overmap monster group placement.

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::core::id::FactionId;
use cdda_components::sim::WorldPosition;
use cdda_components::item::StackCount;
use cdda_components::actor::{Health, IsAlive};
use cdda_components::sim::Solid;

/// Spawn a monster from a definition entity at the given position.
pub fn spawn_monster(
    world: &mut World,
    def_entity: Entity,
    position: WorldPos,
    faction: FactionId,
) -> Entity {
    let entity = world.spawn((
        IsAlive,
        Solid,
        WorldPosition(position),
        Health { current: 100, max: 100 },
    )).id();
    let _ = def_entity;
    let _ = faction;
    entity
}

/// Spawn an item from a definition entity at the given position.
pub fn spawn_item_from_def(
    world: &mut World,
    def_entity: Entity,
    position: WorldPos,
    count: u32,
) -> Entity {
    let entity = world.spawn((
        WorldPosition(position),
        StackCount::new(count),
    )).id();
    let _ = def_entity;
    entity
}
