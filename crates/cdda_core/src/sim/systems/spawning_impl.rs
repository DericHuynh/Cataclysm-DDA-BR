//! Spawning utilities — create gameplay entities from definition entities.
//!
//! Uses Bevy 0.18 `EntityCloner` with `linked_cloning` to clone the def
//! entity (and its linked-spawn children, e.g. container pockets) in one
//! operation. No manual component enumeration — any component with
//! `#[derive(Clone)]` on a def entity automatically propagates to spawns.

use crate::sim::components::{Solid, WorldPosition};
use crate::sim::def_components::*;
use bevy_ecs::entity::EntityCloner;
use bevy_ecs::prelude::*;
use crate::actor::components::{BodyPartDef, BodyPartSlot, Faction, Health, IsAlive};
use crate::coords::WorldPos;
use crate::item::components::{CurrentCharges, DefOrigin, StackCount};

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
    // ItemFlagList is now a type alias for Flags; denied at the sim level
    builder.linked_cloning(true);
    let mut cloner = builder.finish();

    let new_entity = cloner.spawn_clone(world, def_entity);

    // Store def origin for fast numeric identity checks (merge_or_stack, etc.)
    let origin = def_entity.index().index();

    world
        .entity_mut(new_entity)
        .insert(DefOrigin(origin))
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
    faction: crate::FactionId,
) -> Entity {
    let mut builder = EntityCloner::build_opt_out(world);
    builder.deny::<IsDef>();
    builder.deny::<DefStrId>();
    builder.linked_cloning(true);
    let mut cloner = builder.finish();

    let new_entity = cloner.spawn_clone(world, def_entity);

    // Get hp from cloned MonsterStats to seed Health
    let hp = world
        .get::<MonsterStats>(new_entity)
        .map(|s| s.hp)
        .unwrap_or(100);

    world
        .entity_mut(new_entity)
        .insert(IsAlive)
        .insert(Solid)
        .insert(WorldPosition(pos))
        .insert(Health {
            current: hp,
            max: hp,
        })
        .insert(Faction { id: faction });

    new_entity
}

/// Spawn body part instances for a creature by cloning body part defs.
pub fn spawn_body_parts_for_creature(
    world: &mut World,
    def_world: &crate::sim::def_world::DefinitionWorld,
    body_part_ids: &[&str],
) -> Vec<Entity> {
    use std::collections::HashMap;
    let mut instances = Vec::new();
    let mut counters: HashMap<String, u32> = HashMap::new();

    for &def_id_str in body_part_ids {
        let def_entity = match def_world.entity_by_str(def_id_str) {
            Some(e) => e,
            None => continue,
        };

        // Increment counter for slot naming
        let count = counters.entry(def_id_str.to_string()).or_insert(0);
        *count += 1;
        let slot = format!("{}_{}", def_id_str, count);

        // Clone the def entity into a body part instance
        let mut builder = EntityCloner::build_opt_out(world);
        builder.deny::<crate::sim::def_components::IsDef>();
        builder.deny::<crate::sim::def_components::DefStrId>();
        builder.linked_cloning(true);
        let mut cloner = builder.finish();
        let instance = cloner.spawn_clone(world, def_entity);

        world
            .entity_mut(instance)
            .insert(BodyPartSlot(slot))
            .insert(BodyPartDef(def_entity));
        instances.push(instance);
    }

    instances
}
