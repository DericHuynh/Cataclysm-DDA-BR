//! Legacy menu ingress: translate a pending selection into a native intent.
//! Validation and ingredient consumption occur only when the actor is selected.
use super::systems::{find_dev_player, CraftOutcome, CraftRevision, PendingCraft};
use bevy_ecs::prelude::*;
use cdda_components::intent::ActionIntent;

pub fn process_pending_craft(world: &mut World) {
    let Some(recipe) = world.resource_mut::<PendingCraft>().0.take() else {
        return;
    };
    let failure = match find_dev_player(world) {
        Some(player)
            if world
                .get::<cdda_components::actor::IsAlive>(player)
                .is_none()
                || world
                    .get::<cdda_components::actor::ActionPoints>(player)
                    .is_none() =>
        {
            "Crafter cannot act"
        }
        Some(player) if world.get::<ActionIntent>(player).is_none() => {
            world
                .entity_mut(player)
                .insert(ActionIntent::StartCraft { recipe });
            return;
        }
        Some(_) => "Crafter already has a pending action",
        None => "Crafter no longer exists",
    };
    let mut revision = world.resource_mut::<CraftRevision>();
    revision.last_result = Some(CraftOutcome::Failed {
        reason: failure.into(),
    });
    revision.revision += 1;
}
