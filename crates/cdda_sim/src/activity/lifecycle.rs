//! Validating activity lifecycle operations. Progress and type tags are one lifetime.
use bevy_ecs::prelude::*;
use cdda_components::activity::*;
use cdda_components::item::InProgressCraft;

/// Interrupt work without deleting its saved craft item or refunding spent work.
/// Resuming a craft validates ownership and restores its remaining work.
pub fn interrupt_activity(world: &mut World, actor: Entity) -> bool {
    if world.get::<ActivityProgress>(actor).is_none()
        && world.get::<Crafting>(actor).is_none()
        && world.get::<Aiming>(actor).is_none()
        && world.get::<Reading>(actor).is_none()
        && world.get::<Waiting>(actor).is_none()
        && world.get::<Reloading>(actor).is_none()
        && world.get::<Interacting>(actor).is_none()
    {
        return false;
    }
    world.entity_mut(actor).remove::<(
        ActivityProgress,
        Crafting,
        Aiming,
        Reading,
        Waiting,
        Reloading,
        Interacting,
    )>();
    true
}

/// Reject ambiguous activity state before any per-type system can spend AP.
/// An inaccessible or vanished craft is interrupted and its item is retained.
pub(crate) fn ready(world: &mut World, actor: Entity) -> bool {
    let Some(progress) = world.get::<ActivityProgress>(actor) else {
        return false;
    };
    if progress.phase == ActivityPhase::Suspended {
        return false;
    }
    let kinds = [
        world.get::<Crafting>(actor).is_some(),
        world.get::<Aiming>(actor).is_some(),
        world.get::<Reading>(actor).is_some(),
        world.get::<Waiting>(actor).is_some(),
        world.get::<Reloading>(actor).is_some(),
        world.get::<Interacting>(actor).is_some(),
    ];
    if kinds.into_iter().filter(|present| *present).count() != 1 {
        return false;
    }
    if let Some(craft) = world.get::<Crafting>(actor) {
        let item = craft.craft_entity;
        if world.get::<InProgressCraft>(item).is_none()
            || crate::crafting::systems::validate_craft_access(world, actor, item).is_err()
        {
            interrupt_activity(world, actor);
            return false;
        }
    }
    true
}
