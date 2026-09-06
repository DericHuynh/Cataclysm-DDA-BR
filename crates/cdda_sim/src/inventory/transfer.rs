//! Focused, synchronous inventory action boundary.
//!
//! Validates live ownership, projected capacity and costs before changing anything.
//! Native intents and the legacy whole-stack adapter share this commit boundary.
use std::collections::HashSet;

use super::capacity;
use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, HandCount, IsAlive};
use cdda_components::def::IsDef;
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
    Transfer { container: Entity },
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
    PocketFull,
    TooHeavy,
    ItemTooLarge,
    RestrictedPocket,
    InvalidCount,
    UnsupportedItem,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Location {
    Ground(WorldPos),
    Inside(Entity),
    Wielded(Entity),
    Worn(Entity),
    Mounted(Entity),
}

/// Require exactly one location; stale ground positions never override ownership.
pub(super) fn location(world: &World, entity: Entity) -> Result<Location, TransferError> {
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

/// Validate an item's exclusive ownership chain up to a known actor. Headless
/// actors need no ground position for crafting; every descendant still needs
/// exactly one valid location. Work is bounded by nesting depth.
pub(crate) fn belongs_to(world: &World, item: Entity, actor: Entity) -> bool {
    let mut current = item;
    let mut seen = HashSet::new();
    while current != actor {
        if !seen.insert(current) {
            return false;
        }
        current = match location(world, current) {
            Ok(
                Location::Inside(parent)
                | Location::Wielded(parent)
                | Location::Worn(parent)
                | Location::Mounted(parent),
            ) => parent,
            _ => return false,
        };
    }
    world.get_entity(actor).is_ok()
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
        || world
            .get::<cdda_components::activity::ActivityProgress>(actor)
            .is_some()
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
    capacity::check_access(world, capacity::parent(world, item))?;
    let root = location_root(world, item)?;
    let owned = root == actor;
    let reachable_unowned = world.get::<IsAlive>(root).is_none()
        && matches!(location(world, root)?, Location::Ground(pos) if within_reach(actor_pos, pos));
    let reachable_ground = matches!(source, Location::Ground(pos) if within_reach(actor_pos, pos));
    let (destination, cost) = match action {
        InventoryAction::Pickup => {
            if !reachable_ground {
                return Err(TransferError::OutOfReach);
            }
            (
                Location::Inside(select_storage(world, actor, item)?),
                AP_COST_PICKUP,
            )
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
            let item_volume = capacity::item_load(world, item)?.volume_ml;
            let mut floor = world.query_filtered::<(Entity, &WorldPosition), (Without<IsAlive>, Without<IsDef>, Without<IsPocket>)>();
            let mut floor_volume = 0u64;
            for (entity, pos) in floor.iter(world) {
                if pos.get() == actor_pos {
                    if !matches!(location(world, entity)?, Location::Ground(_)) {
                        return Err(TransferError::InvalidLocation);
                    }
                    floor_volume = floor_volume
                        .checked_add(capacity::item_load(world, entity)?.volume_ml)
                        .ok_or(TransferError::FloorFull)?;
                }
            }
            if floor_volume
                .checked_add(item_volume)
                .is_none_or(|v| v > u64::from(FLOOR_CAP_ML))
            {
                return Err(TransferError::FloorFull);
            }
            (Location::Ground(actor_pos), AP_COST_PICKUP)
        }
        InventoryAction::Stow => {
            if !owned || !matches!(source, Location::Wielded(owner) if owner == actor) {
                return Err(TransferError::NotOwned);
            }
            (
                Location::Inside(select_storage(world, actor, item)?),
                AP_COST_WIELD,
            )
        }
        InventoryAction::Transfer { container } => {
            if (!owned && !reachable_unowned) || matches!(source, Location::Mounted(_)) {
                return Err(TransferError::NotOwned);
            }
            if matches!(source, Location::Inside(old) if old == container) {
                return Err(TransferError::InvalidLocation);
            }
            let destination_root = location_root(world, container)?;
            if destination_root != actor {
                let Location::Ground(pos) = location(world, destination_root)? else {
                    return Err(TransferError::InvalidLocation);
                };
                if world.get::<IsAlive>(destination_root).is_some() || !within_reach(actor_pos, pos)
                {
                    return Err(TransferError::NotOwned);
                }
            }
            (Location::Inside(container), AP_COST_PICKUP)
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

    if let Location::Inside(parent) = destination {
        capacity::validate_capacity(world, parent, item)?;
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

fn select_storage(world: &mut World, actor: Entity, item: Entity) -> Result<Entity, TransferError> {
    let mut q = world.query::<(Entity, &cdda_components::item::Pocket)>();
    let mut candidates: Vec<_> = q
        .iter(world)
        .map(|(e, _)| e)
        .filter(|&e| {
            if location_root(world, e) != Ok(actor) {
                return false;
            }
            let mut parent = Some(e);
            while let Some(p) = parent {
                if p == item {
                    return false;
                }
                parent = capacity::parent(world, p);
            }
            true
        })
        .collect();
    candidates.sort_by_key(|&e| stable_key(world, e));
    let has_storage = !candidates.is_empty();
    for e in candidates {
        if capacity::validate_capacity(world, e, item).is_ok() {
            return Ok(e);
        }
    }
    if !has_storage {
        capacity::validate_capacity(world, actor, item)?;
        return Ok(actor);
    }
    Err(TransferError::PocketFull)
}

/// Compatibility adapter: stale source/count claims never relocate a whole stack.
/// A living actor must be inferable from the source or requested destination.
pub fn apply_legacy_move(
    world: &mut World,
    request: &cdda_components::events::ItemMoveEvent,
) -> Result<(), TransferError> {
    use cdda_components::{events::MoveLocation, item::StackCount};
    let actual = location(world, request.item)?;
    let matches = match (actual, &request.from) {
        (Location::Ground(a), MoveLocation::Ground(b)) => a == *b,
        (Location::Inside(a), MoveLocation::Container(b))
        | (Location::Wielded(a), MoveLocation::Wielded(b))
        | (Location::Worn(a), MoveLocation::Worn(b)) => a == *b,
        _ => false,
    };
    if !matches {
        return Err(TransferError::InvalidLocation);
    }
    if request.count != world.get::<StackCount>(request.item).map_or(1, |s| s.get()) {
        return Err(TransferError::InvalidCount);
    }
    let source_root = location_root(world, request.item)?;
    let actor = if world.get::<IsAlive>(source_root).is_some() {
        source_root
    } else {
        let parent = match request.to {
            MoveLocation::Container(e) | MoveLocation::Wielded(e) => e,
            _ => return Err(TransferError::IneligibleActor),
        };
        location_root(world, parent)?
    };
    let action = match request.to {
        MoveLocation::Container(container) => InventoryAction::Transfer { container },
        MoveLocation::Wielded(e) if e == actor => InventoryAction::Wield,
        MoveLocation::Ground(pos)
            if world
                .get::<WorldPosition>(actor)
                .is_some_and(|p| p.get() == pos) =>
        {
            InventoryAction::Drop
        }
        _ => return Err(TransferError::InvalidLocation),
    };
    apply_inventory_action(world, actor, request.item, action)
}
