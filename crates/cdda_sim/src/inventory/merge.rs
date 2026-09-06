//! Explicit colocated-stack consolidation. Never a hidden inventory transfer.
use super::transfer::{location, Location};
use bevy_ecs::prelude::*;
use cdda_catalog::inventory::ItemDefinitionRef;
use cdda_components::{
    actor::IsAlive,
    def::{DefStrId, IsDef, ItemVolume, ItemWeight},
    item::*,
};

pub fn merge_stacks(world: &mut World, target: Entity, incoming: Entity) -> bool {
    if target == incoming {
        return false;
    }
    let (Ok(a), Ok(b)) = (location(world, target), location(world, incoming)) else {
        return false;
    };
    if a != b || !matches!(a, Location::Ground(_) | Location::Inside(_)) {
        return false;
    }
    if super::capacity::check_access(world, super::capacity::parent(world, target)).is_err() {
        return false;
    }
    // Objects with independent state/lifetimes must retain their entities.
    for e in [target, incoming] {
        if world.get::<IsDef>(e).is_some()
            || world.get::<IsAlive>(e).is_some()
            || world.get::<IsPocket>(e).is_some()
            || world.get::<MountedPockets>(e).is_some()
            || world.get::<ContainerContents>(e).is_some()
            || world.get::<Container>(e).is_some()
            || world.get::<Pocket>(e).is_some()
            || world.get::<WieldedItems>(e).is_some()
            || world.get::<WornBy>(e).is_some()
            || world.get::<LoadedAmmo>(e).is_some()
            || world.get::<Spoilable>(e).is_some()
            || world.get::<InProgressCraft>(e).is_some()
        {
            return false;
        }
    }
    let same_type = match (
        world.get::<DefOrigin>(target),
        world.get::<DefOrigin>(incoming),
    ) {
        (Some(a), Some(b)) => a.0 == b.0,
        _ => {
            matches!((world.get::<DefStrId>(target), world.get::<DefStrId>(incoming)), (Some(a), Some(b)) if a.0 == b.0)
        }
    };
    if !same_type {
        return false;
    }
    if let (Some(a), Some(b)) = (
        world.get::<DefStrId>(target),
        world.get::<DefStrId>(incoming),
    ) {
        if a.0 != b.0 {
            return false;
        }
    }
    match (
        world.get::<ItemDefinitionRef>(target),
        world.get::<ItemDefinitionRef>(incoming),
    ) {
        (Some(a), Some(b)) if std::sync::Arc::ptr_eq(&a.0, &b.0) => {}
        (None, None) => {}
        _ => return false,
    }
    if world.get::<ItemVolume>(target).map(|v| v.0)
        != world.get::<ItemVolume>(incoming).map(|v| v.0)
        || world.get::<ItemWeight>(target).map(|v| v.0)
            != world.get::<ItemWeight>(incoming).map(|v| v.0)
        || world.get::<ItemDamage>(target).map_or(0, |v| v.0)
            != world.get::<ItemDamage>(incoming).map_or(0, |v| v.0)
    {
        return false;
    }
    let count = world
        .get::<StackCount>(target)
        .map_or(1, |v| v.get())
        .checked_add(world.get::<StackCount>(incoming).map_or(1, |v| v.get()));
    let charges = world
        .get::<CurrentCharges>(target)
        .map_or(0, |v| v.0)
        .checked_add(world.get::<CurrentCharges>(incoming).map_or(0, |v| v.0));
    let (Some(count), Some(charges)) = (count, charges) else {
        return false;
    };
    // Dimensions are equal and both stacks already occupy the same location:
    // aggregate volume/weight is conserved; no capacity is newly consumed.
    world
        .entity_mut(target)
        .insert(StackCount::new(count).expect("checked positive sum"));
    if world.get::<CurrentCharges>(target).is_some()
        || world.get::<CurrentCharges>(incoming).is_some()
    {
        world.entity_mut(target).insert(CurrentCharges(charges));
    }
    world.despawn(incoming);
    true
}
