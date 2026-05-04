//! Inventory system — handle picking up, dropping, and moving items.

use crate::components::*;
use crate::events::{ItemMoveEvent, MoveLocation};
use bevy_ecs::prelude::*;
use cdda_core::coords::WorldPos;

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
