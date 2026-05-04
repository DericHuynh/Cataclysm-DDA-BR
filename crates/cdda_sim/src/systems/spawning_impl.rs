//! Spawning utilities — create gameplay entities from definition entities.
//!
//! Uses Bevy 0.18 `EntityCloner` with `linked_cloning` to clone the def
//! entity (and its linked-spawn children, e.g. container pockets) in one
//! operation. No manual component enumeration — any component with
//! `#[derive(Clone)]` on a def entity automatically propagates to spawns.

use crate::components::*;
use crate::def_components::*;
use bevy_ecs::entity::EntityCloner;
use bevy_ecs::prelude::*;
use cdda_core::coords::WorldPos;

/// Spawn a gameplay item entity by cloning a definition entity.
///
/// Clones all Clone-deriving components except explicitly denied ones.
/// `linked_cloning(true)` recursively clones linked-spawn children.
pub fn spawn_item_from_def(
    world: &mut World,
    def_entity: Entity,
    pos: WorldPos,
    count: u32,
) -> Entity {
    let mut builder = EntityCloner::build_opt_out(world);
    builder.deny::<IsDef>();
    builder.deny::<DefStrId>();
    builder.deny::<ItemPrice>();
    builder.deny::<ItemFlagList>();
    builder.linked_cloning(true);
    let mut cloner = builder.finish();

    let new_entity = cloner.spawn_clone(world, def_entity);

    world.entity_mut(new_entity)
        .insert(StackCount::new(count))
        .insert(CurrentCharges(0))
        .insert(WorldPosition(pos));

    new_entity
}

/// Spawn a gameplay creature entity by cloning a monster definition entity.
pub fn spawn_creature_from_def(
    world: &mut World,
    def_entity: Entity,
    pos: WorldPos,
    faction: cdda_core::FactionId,
) -> Entity {
    let mut builder = EntityCloner::build_opt_out(world);
    builder.deny::<IsDef>();
    builder.deny::<DefStrId>();
    builder.linked_cloning(true);
    let mut cloner = builder.finish();

    let new_entity = cloner.spawn_clone(world, def_entity);

    // Get hp from cloned MonsterStats to seed Health
    let hp = world.get::<MonsterStats>(new_entity)
        .map(|s| s.hp)
        .unwrap_or(100);

    world.entity_mut(new_entity)
        .insert(IsAlive)
        .insert(Solid)
        .insert(WorldPosition(pos))
        .insert(Health { current: hp, max: hp })
        .insert(Faction { id: faction });

    new_entity
}

/// Convert a definition-level `BodyPartId` to the ECS `BodyPartSlot`.
pub fn body_part_id_to_slot(id: cdda_core::BodyPartId) -> BodyPartSlot {
    match (id.0).0 {
        0 => BodyPartSlot::Head,
        1 => BodyPartSlot::Eyes,
        2 => BodyPartSlot::Mouth,
        3 => BodyPartSlot::Torso,
        4 => BodyPartSlot::ArmLeft,
        5 => BodyPartSlot::ArmRight,
        6 => BodyPartSlot::HandLeft,
        7 => BodyPartSlot::HandRight,
        8 => BodyPartSlot::LegLeft,
        9 => BodyPartSlot::LegRight,
        10 => BodyPartSlot::FootLeft,
        11 => BodyPartSlot::FootRight,
        _ => BodyPartSlot::Torso,
    }
}
