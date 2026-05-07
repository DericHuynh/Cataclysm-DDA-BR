//! Inventory system — handle picking up, dropping, and moving items.

use crate::components::WorldPosition;
use crate::events::{ItemMoveEvent, MoveLocation};
use bevy_ecs::prelude::*;
use cdda_core::coords::WorldPos;
use cdda_core::units::*;
use cdda_item::components::{Container, ContainerContents, CurrentCharges, DefOrigin, InsideContainer, ItemDamage, Pocket, StackCount};

/// Pick up an item from the ground into a container entity.
/// Returns an event for the calling system to emit.
pub fn pickup_item(
    commands: &mut Commands,
    collector: Entity,
    item: Entity,
    item_query: &Query<(&WorldPosition, Option<&StackCount>)>,
) -> Option<ItemMoveEvent> {
    if let Ok((pos, stack)) = item_query.get(item) {
        let count = stack.map(|s| s.get()).unwrap_or(1);
        commands
            .entity(item)
            .remove::<WorldPosition>()
            .insert(InsideContainer(collector));
        Some(ItemMoveEvent {
            item,
            from: MoveLocation::Ground(pos.0),
            to: MoveLocation::Container(collector),
            count,
        })
    } else {
        None
    }
}

/// Drop an item from a container to the ground.
pub fn drop_item(
    commands: &mut Commands,
    container: Entity,
    item: Entity,
    drop_pos: WorldPos,
) -> Option<ItemMoveEvent> {
    commands
        .entity(item)
        .remove::<InsideContainer>()
        .insert(WorldPosition(drop_pos));
    Some(ItemMoveEvent {
        item,
        from: MoveLocation::Container(container),
        to: MoveLocation::Ground(drop_pos),
        count: 1,
    })
}

/// Transfer an item from one container to another.
pub fn transfer_item(
    commands: &mut Commands,
    item: Entity,
    from_container: Entity,
    to_container: Entity,
) -> Option<ItemMoveEvent> {
    commands.entity(item).insert(InsideContainer(to_container));
    Some(ItemMoveEvent {
        item,
        from: MoveLocation::Container(from_container),
        to: MoveLocation::Container(to_container),
        count: 1,
    })
}

/// Get the world position of an item, considering it may be in a container.
pub fn effective_position(
    item: Entity,
    positions: &Query<&WorldPosition>,
    containers: &Query<&InsideContainer>,
) -> Option<WorldPos> {
    if let Ok(pos) = positions.get(item) {
        return Some(pos.0);
    }
    let mut current = item;
    loop {
        if let Ok(InsideContainer(parent)) = containers.get(current) {
            if let Ok(pos) = positions.get(*parent) {
                return Some(pos.0);
            }
            current = *parent;
        } else {
            return None;
        }
    }
}

/// Get all items at a world position.
pub fn items_at_position<'a>(
    pos: WorldPos,
    positions: &'a Query<(&WorldPosition, Entity), With<StackCount>>,
) -> Vec<Entity> {
    positions
        .iter()
        .filter(|(p, _)| p.0 == pos)
        .map(|(_, e)| e)
        .collect()
}

/// Get all items inside a container entity.
pub fn items_in_container<'a>(
    container: Entity,
    inside: &'a Query<(Entity, &InsideContainer)>,
) -> Vec<Entity> {
    inside
        .iter()
        .filter(|(_, ic)| ic.0 == container)
        .map(|(e, _)| e)
        .collect()
}

/// Check whether an item can physically fit into a container.
///
/// Checks volume capacity, weight capacity, item length vs pocket
/// restrictions. Returns `true` if no constraints exist or all pass.
pub fn can_fit_in_container(world: &World, container: Entity, item: Entity) -> bool {
    use crate::def_components::{ItemLongestSide, ItemVolume, ItemWeight};

    let item_vol = match world.get::<ItemVolume>(item) {
        Some(v) => Volume::from_milliliters(v.0 as u64),
        None => return true,
    };
    let item_wgt = match world.get::<ItemWeight>(item) {
        Some(w) => Weight::from_grams(w.0 as u64),
        None => Weight::ZERO,
    };

    // Check against Pocket component (runtime containers)
    if let Some(pocket) = world.get::<Pocket>(container) {
        if item_vol > pocket.max_volume {
            return false;
        }
        if item_wgt > pocket.max_weight {
            return false;
        }
        if item_vol < pocket.min_item_volume {
            return false;
        }
        if let Some(longest) = world.get::<ItemLongestSide>(item) {
            let item_len = Length::from_millimeters(longest.0);
            if item_len > pocket.max_item_length {
                return false;
            }
        }
        return true;
    }

    // Check against Container component (simple capacity)
    if let Some(container_data) = world.get::<Container>(container) {
        let current_vol = total_container_volume(world, container);
        return current_vol + item_vol <= container_data.capacity;
    }

    true
}

/// Calculate the total volume of all items inside a container.
///
/// Iterates the `ContainerContents` relationship and sums each item's
/// `ItemVolume` multiplied by `StackCount`.
pub fn total_container_volume(world: &World, container: Entity) -> Volume {
    use crate::def_components::ItemVolume;
    let mut total = Volume::ZERO;
    if let Some(contents) = world.get::<ContainerContents>(container) {
        for child in contents.iter() {
            if let Some(vol) = world.get::<ItemVolume>(child) {
                let count = world.get::<StackCount>(child).map(|s| s.get()).unwrap_or(1);
                total = total + Volume::from_milliliters(vol.0 as u64 * count as u64);
            }
        }
    }
    total
}

/// Calculate the total weight of all items inside a container.
///
/// Iterates the `ContainerContents` relationship and sums each item's
/// `ItemWeight` multiplied by `StackCount`.
pub fn total_container_weight(world: &World, container: Entity) -> Weight {
    use crate::def_components::ItemWeight;
    let mut total = Weight::ZERO;
    if let Some(contents) = world.get::<ContainerContents>(container) {
        for child in contents.iter() {
            if let Some(wgt) = world.get::<ItemWeight>(child) {
                let count = world.get::<StackCount>(child).map(|s| s.get()).unwrap_or(1);
                total = total + Weight::from_grams(wgt.0 as u64 * count as u64);
            }
        }
    }
    total
}

/// Attempt to merge an incoming item with an existing stack of the same type.
///
/// If `target` and `incoming` have the same definition ID and damage level,
/// the incoming item's `StackCount` is added to the target and the incoming
/// entity is despawned. Returns `true` if the merge succeeded.
pub fn merge_or_stack(world: &mut World, target: Entity, incoming: Entity) -> bool {
    use crate::def_components::{DefStrId, ItemName};

    // Phase 1: type identity check — try DefOrigin first (fast, numeric)
    let same_type = match (
        world.get::<DefOrigin>(target),
        world.get::<DefOrigin>(incoming),
    ) {
        (Some(t), Some(i)) => t.0 == i.0,
        _ => {
            // Fallback: compare DefStrId (for manual test items without DefOrigin)
            match (
                world.get::<DefStrId>(target),
                world.get::<DefStrId>(incoming),
            ) {
                (Some(t), Some(i)) => t.0 == i.0,
                _ => {
                    // Last resort: compare ItemName
                    match (
                        world.get::<ItemName>(target),
                        world.get::<ItemName>(incoming),
                    ) {
                        (Some(t), Some(i)) => t.0 == i.0,
                        _ => return false,
                    }
                }
            }
        }
    };
    if !same_type {
        return false;
    }

    let incoming_count = world
        .get::<StackCount>(incoming)
        .map(|s| s.get())
        .unwrap_or(1);
    let target_count = world
        .get::<StackCount>(target)
        .map(|s| s.get())
        .unwrap_or(1);
    let incoming_charges = world
        .get::<CurrentCharges>(incoming)
        .map(|c| c.0)
        .unwrap_or(0);
    let target_charges = world
        .get::<CurrentCharges>(target)
        .map(|c| c.0)
        .unwrap_or(0);
    let incoming_damage = world.get::<ItemDamage>(incoming).map(|d| d.0).unwrap_or(0);
    let target_damage = world.get::<ItemDamage>(target).map(|d| d.0).unwrap_or(0);

    // Don't merge items with different damage levels
    if incoming_damage != target_damage {
        return false;
    }

    // Phase 2: mutation — all immutable borrows are dropped
    world
        .entity_mut(target)
        .insert(StackCount::new(target_count + incoming_count));
    world
        .entity_mut(target)
        .insert(CurrentCharges(target_charges + incoming_charges));
    world.despawn(incoming);
    true
}
