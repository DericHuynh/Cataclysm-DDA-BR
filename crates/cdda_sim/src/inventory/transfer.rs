//! Focused, synchronous inventory action boundary.
//!
//! Validates live ownership and costs before changing anything. Legacy
//! ItemMoveEvent/merge paths do not yet use this boundary. Pocket volume/weight
//! restrictions remain deferred; the body pocket is the existing omnibus store.
use std::collections::HashSet;

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, HandCount, IsAlive};
use cdda_components::def::{IsDef, ItemVolume};
use cdda_components::item::{
    ContainerContents, InsideContainer, Invlet, IsPocket, MountedOn, MountedPockets, WieldedBy,
    WieldedItems, WornBy, WornOn, FLOOR_CAP_ML,
};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::sim_id::SimId;

use crate::actor::turn::{AP_COST_PICKUP, AP_COST_WIELD};

#[derive(Debug, Clone, Copy)]
pub enum InventoryAction {
    Pickup,
    Wield,
    Drop,
    Stow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferError {
    IneligibleActor,
    InvalidItem,
    InvalidLocation,
    NotOwned,
    OutOfReach,
    HandsFull,
    FloorFull,
}

#[derive(Clone, Copy)]
enum Location {
    Ground(WorldPos),
    Inside(Entity),
    Wielded(Entity),
    Worn(Entity),
    Mounted(Entity),
}

/// Require exactly one location; stale ground positions never override ownership.
fn location(world: &World, entity: Entity) -> Result<Location, TransferError> {
    let locations = [
        world
            .get::<WorldPosition>(entity)
            .map(|p| Location::Ground(p.get())),
        world
            .get::<InsideContainer>(entity)
            .map(|p| Location::Inside(p.0)),
        world
            .get::<WieldedBy>(entity)
            .map(|p| Location::Wielded(p.0)),
        world
            .get::<WornOn>(entity)
            .map(|p| Location::Worn(p.wearer)),
        world
            .get::<MountedOn>(entity)
            .map(|p| Location::Mounted(p.0)),
    ];
    let mut locations = locations.into_iter().flatten();
    let first = locations.next().ok_or(TransferError::InvalidLocation)?;
    if locations.next().is_some() {
        return Err(TransferError::InvalidLocation);
    }
    Ok(first)
}

/// Find the root of a well-formed live ownership chain, rejecting cycles and
/// ambiguous locations. A root can be an actor or an unowned ground item.
pub fn location_root(world: &World, item: Entity) -> Result<Entity, TransferError> {
    let mut current = item;
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(current) {
            return Err(TransferError::InvalidLocation);
        }
        match location(world, current)? {
            Location::Ground(_) => return Ok(current),
            Location::Inside(parent)
            | Location::Wielded(parent)
            | Location::Worn(parent)
            | Location::Mounted(parent) => current = parent,
        }
    }
}

/// Same-z one-tile Chebyshev reach, safe even at coordinate extremes.
pub fn within_reach(actor: WorldPos, item: WorldPos) -> bool {
    actor.z == item.z && actor.x.abs_diff(item.x) <= 1 && actor.y.abs_diff(item.y) <= 1
}

fn stable_key(world: &World, entity: Entity) -> (bool, u64, u64) {
    let id = world.get::<SimId>(entity);
    (
        id.is_none(),
        id.map(|id| id.0).unwrap_or_default(),
        entity.to_bits(),
    )
}

/// Apply one complete item action and charge AP exactly once. Every error is
/// no-cost and mutation-free. Zero AP is accepted for isolated legacy callers;
/// the production budget driver selects only actors with positive AP.
pub fn apply_inventory_action(
    world: &mut World,
    actor: Entity,
    item: Entity,
    action: InventoryAction,
) -> Result<(), TransferError> {
    if world.get::<IsAlive>(actor).is_none()
        || world.get::<IsDef>(actor).is_some()
        || !world
            .get::<ActionPoints>(actor)
            .is_some_and(|ap| ap.current >= 0)
    {
        return Err(TransferError::IneligibleActor);
    }
    let Location::Ground(actor_pos) = location(world, actor)? else {
        return Err(TransferError::InvalidLocation);
    };
    if item == actor
        || world.get_entity(item).is_err()
        || world.get::<IsAlive>(item).is_some()
        || world.get::<IsPocket>(item).is_some()
        || world.get::<IsDef>(item).is_some()
    {
        return Err(TransferError::InvalidItem);
    }
    let source = location(world, item)?;
    let root = location_root(world, item)?;
    let owned = root == actor;
    let reachable_ground = matches!(source, Location::Ground(pos) if within_reach(actor_pos, pos));
    let (destination, cost) = match action {
        InventoryAction::Pickup => {
            if !reachable_ground {
                return Err(TransferError::OutOfReach);
            }
            (Location::Inside(actor), AP_COST_PICKUP)
        }
        InventoryAction::Wield => {
            if !reachable_ground && !(owned && matches!(source, Location::Inside(_))) {
                return Err(TransferError::NotOwned);
            }
            let hands = world
                .get::<HandCount>(actor)
                .map_or(0, |hands| hands.0 as usize);
            let mut wielded = world.query::<&WieldedBy>();
            if wielded.iter(world).filter(|owner| owner.0 == actor).count() >= hands {
                return Err(TransferError::HandsFull);
            }
            (Location::Wielded(actor), AP_COST_WIELD)
        }
        InventoryAction::Drop => {
            if !owned || matches!(source, Location::Mounted(_)) {
                return Err(TransferError::NotOwned);
            }
            let item_volume = world.get::<ItemVolume>(item).map_or(0, |v| u64::from(v.0));
            let mut floor = world.query::<(Entity, &WorldPosition, Option<&ItemVolume>)>();
            let floor_volume: u64 = floor
                .iter(world)
                .filter(|(entity, pos, _)| *entity != actor && pos.get() == actor_pos)
                .map(|(_, _, volume)| volume.map_or(0, |v| u64::from(v.0)))
                .sum();
            if floor_volume.saturating_add(item_volume) > u64::from(FLOOR_CAP_ML) {
                return Err(TransferError::FloorFull);
            }
            (Location::Ground(actor_pos), AP_COST_PICKUP)
        }
        InventoryAction::Stow => {
            if !owned || !matches!(source, Location::Wielded(owner) if owner == actor) {
                return Err(TransferError::NotOwned);
            }
            let destination = world
                .get::<MountedPockets>(actor)
                .into_iter()
                .flat_map(|pockets| pockets.iter())
                .filter(|pocket| world.get::<IsPocket>(*pocket).is_some())
                .min_by_key(|pocket| stable_key(world, *pocket))
                .unwrap_or(actor);
            if location_root(world, destination)? != actor {
                return Err(TransferError::InvalidLocation);
            }
            (Location::Inside(destination), AP_COST_WIELD)
        }
    };
    // The new parent must not be the item or one of its descendants. Checking
    // the full forward chain also fails closed on preexisting graph corruption.
    if let Location::Inside(parent) | Location::Wielded(parent) = destination {
        let mut current = parent;
        let mut seen = HashSet::new();
        loop {
            if current == item || !seen.insert(current) {
                return Err(TransferError::InvalidLocation);
            }
            match location(world, current)? {
                Location::Ground(_) => break,
                Location::Inside(p)
                | Location::Wielded(p)
                | Location::Worn(p)
                | Location::Mounted(p) => current = p,
            }
        }
    }

    // Everything above is read-only; commits and reverse links are immediate.
    let mut entity = world.entity_mut(item);
    entity.remove::<(WorldPosition, InsideContainer, WieldedBy, WornOn, MountedOn)>();
    match destination {
        Location::Ground(pos) => {
            entity.insert(WorldPosition::new(pos));
        }
        Location::Inside(parent) => {
            entity.insert(InsideContainer(parent));
        }
        Location::Wielded(parent) => {
            entity.insert(WieldedBy(parent));
        }
        _ => unreachable!("focused actions never equip or mount"),
    }
    if matches!(destination, Location::Ground(_)) {
        // A dropped bag's contents also leave inventory; retain their graph but
        // clear stale inventory letters throughout the subtree.
        let mut pending = vec![item];
        let mut seen = HashSet::new();
        while let Some(entity) = pending.pop() {
            if !seen.insert(entity) {
                continue;
            }
            if let Some(children) = world.get::<ContainerContents>(entity) {
                pending.extend(children.iter());
            }
            if let Some(children) = world.get::<MountedPockets>(entity) {
                pending.extend(children.iter());
            }
            if let Some(children) = world.get::<WieldedItems>(entity) {
                pending.extend(children.iter());
            }
            if let Some(children) = world.get::<WornBy>(entity) {
                pending.extend(children.iter());
            }
            if let Ok(mut entity) = world.get_entity_mut(entity) {
                entity.remove::<Invlet>();
            }
        }
    }
    world
        .get_mut::<ActionPoints>(actor)
        .expect("validated actor")
        .spend(cost);
    Ok(())
}
