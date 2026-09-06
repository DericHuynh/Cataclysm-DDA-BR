//! Read-only capacity checks for counted solids and unrestricted pockets.
//! Project the proposed edge before mutation, including occupied ancestor pockets.
use super::transfer::TransferError;
use bevy_ecs::prelude::*;
use cdda_components::{
    actor::IsAlive,
    def::{IsDef, ItemLongestSide, ItemVolume, ItemWeight},
    item::*,
};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Load {
    pub volume_ml: u64,
    pub weight_g: u64,
}
impl Load {
    fn add(self, other: Self) -> Result<Self, TransferError> {
        Ok(Self {
            volume_ml: self
                .volume_ml
                .checked_add(other.volume_ml)
                .ok_or(TransferError::InvalidItem)?,
            weight_g: self
                .weight_g
                .checked_add(other.weight_g)
                .ok_or(TransferError::InvalidItem)?,
        })
    }
}
#[derive(Clone, Copy)]
struct Projection {
    item: Entity,
    parent: Entity,
}

fn children_load(
    world: &World,
    entity: Entity,
    projection: Option<Projection>,
    seen: &mut HashSet<Entity>,
) -> Result<Load, TransferError> {
    let mut load = Load::default();
    let children = world
        .get::<ContainerContents>(entity)
        .into_iter()
        .flat_map(|c| c.iter())
        .chain(
            world
                .get::<MountedPockets>(entity)
                .into_iter()
                .flat_map(|c| c.iter()),
        );
    for child in children {
        if projection.is_some_and(|p| p.item == child) {
            continue;
        }
        load = load.add(measure(world, child, projection, seen)?)?;
    }
    if let Some(p) = projection.filter(|p| p.parent == entity) {
        load = load.add(measure(world, p.item, None, seen)?)?;
    }
    Ok(load)
}
fn measure(
    world: &World,
    entity: Entity,
    projection: Option<Projection>,
    seen: &mut HashSet<Entity>,
) -> Result<Load, TransferError> {
    if world.get_entity(entity).is_err() || !seen.insert(entity) {
        return Err(TransferError::InvalidLocation);
    }
    use cdda_components::def::{CountMode, ItemCountMode, ItemPhase, Phase};
    if world
        .get::<ItemPhase>(entity)
        .is_some_and(|p| !matches!(p.0, Phase::Solid))
        || world
            .get::<ItemCountMode>(entity)
            .is_some_and(|c| matches!(c.0, CountMode::Charges { .. }))
    {
        return Err(TransferError::UnsupportedItem);
    }
    let count = u64::from(world.get::<StackCount>(entity).map_or(1, |s| s.get()));
    let own = Load {
        volume_ml: u64::from(world.get::<ItemVolume>(entity).map_or(0, |v| v.0))
            .checked_mul(count)
            .ok_or(TransferError::InvalidItem)?,
        weight_g: u64::from(world.get::<ItemWeight>(entity).map_or(0, |v| v.0))
            .checked_mul(count)
            .ok_or(TransferError::InvalidItem)?,
    };
    let mut contents = children_load(world, entity, projection, seen)?;
    if world.get::<Rigid>(entity).is_some() {
        contents.volume_ml = 0;
    }
    own.add(contents)
}
pub fn item_load(world: &World, item: Entity) -> Result<Load, TransferError> {
    measure(world, item, None, &mut HashSet::new())
}
pub fn contents_load(world: &World, container: Entity) -> Result<Load, TransferError> {
    children_load(world, container, None, &mut HashSet::from([container]))
}
pub(crate) fn parent(world: &World, entity: Entity) -> Option<Entity> {
    world
        .get::<InsideContainer>(entity)
        .map(|v| v.0)
        .or_else(|| world.get::<MountedOn>(entity).map(|v| v.0))
        .or_else(|| world.get::<WieldedBy>(entity).map(|v| v.0))
        .or_else(|| world.get::<WornOn>(entity).map(|v| v.wearer))
}
/// Reject sealed ancestors, but allow moving an unopened sealed item itself.
pub fn check_access(world: &World, start: Option<Entity>) -> Result<(), TransferError> {
    let mut seen = HashSet::new();
    let mut next = start;
    while let Some(e) = next {
        if !seen.insert(e) || world.get_entity(e).is_err() {
            return Err(TransferError::InvalidLocation);
        }
        if world.get::<Sealed>(e).is_some() {
            return Err(TransferError::RestrictedPocket);
        }
        next = parent(world, e);
    }
    Ok(())
}

/// Data-level fit predicate, also usable for unlocated fixture/spawn candidates.
/// Missing dimensions retain legacy zero defaults; weight is always checked.
pub fn validate_capacity(
    world: &World,
    container: Entity,
    item: Entity,
) -> Result<(), TransferError> {
    if world.get_entity(container).is_err()
        || world.get_entity(item).is_err()
        || world.get::<IsDef>(container).is_some()
    {
        return Err(TransferError::InvalidLocation);
    }
    // Validate the payload even when legacy loose storage has no finite limit.
    item_load(world, item)?;
    check_access(world, Some(container))?;
    let mut seen = HashSet::new();
    let mut next = Some(container);
    let projection = Some(Projection {
        item,
        parent: container,
    });
    while let Some(e) = next {
        if e == item || !seen.insert(e) {
            return Err(TransferError::InvalidLocation);
        }
        if world.get::<PocketRestriction>(e).is_some() {
            return Err(TransferError::RestrictedPocket);
        }
        if let Some(pocket) = world.get::<Pocket>(e) {
            if !matches!(pocket.pocket_type, PocketType::Container) {
                return Err(TransferError::RestrictedPocket);
            }
            if e == container {
                let volume = u64::from(world.get::<ItemVolume>(item).map_or(0, |v| v.0));
                if volume < pocket.min_item_volume.0
                    || world
                        .get::<ItemLongestSide>(item)
                        .is_some_and(|v| v.0 > pocket.max_item_length.0)
                {
                    return Err(TransferError::ItemTooLarge);
                }
            }
            let load = children_load(world, e, projection, &mut HashSet::from([e]))?;
            if load.volume_ml > pocket.max_volume.0 {
                return Err(TransferError::PocketFull);
            }
            if load.weight_g > pocket.max_weight.0 {
                return Err(TransferError::TooHeavy);
            }
        } else if let Some(data) = world.get::<Container>(e) {
            if children_load(world, e, projection, &mut HashSet::from([e]))?.volume_ml
                > data.capacity.0
            {
                return Err(TransferError::PocketFull);
            }
        } else if e == container {
            // Loose inventory is a compatibility fallback only for actors without
            // modeled pockets, never an escape hatch around a full bag/pocket.
            if world.get::<IsAlive>(e).is_none()
                || std::iter::once(e)
                    .chain(super::systems::all_items_for_creature(e, world))
                    .any(|holder| {
                        let mut ancestor = Some(holder);
                        let mut seen = HashSet::new();
                        while let Some(a) = ancestor {
                            if a == item {
                                return false;
                            }
                            if !seen.insert(a) {
                                return true;
                            }
                            ancestor = parent(world, a);
                        }
                        world.get::<Pocket>(holder).is_some()
                            || world
                                .get::<MountedPockets>(holder)
                                .is_some_and(|p| p.iter().next().is_some())
                    })
            {
                return Err(TransferError::InvalidLocation);
            }
        }
        next = parent(world, e);
    }
    Ok(())
}
