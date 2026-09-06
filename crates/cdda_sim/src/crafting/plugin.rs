use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;

use super::input::process_pending_craft;
use super::systems::{complete_craft, CraftOutcome, CraftRevision, PendingCraft, RecipeIndex};
use crate::activity::systems::tick_crafting;
use cdda_components::messages::CraftCompleted;
use cdda_components::schedule::{SimSet, SimulationActivity, SimulationIngress};

pub struct CraftingPlugin;

impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        // Register CraftCompleted message — replaces the old CRAFT_COMPLETE_HOOK.
        // tick_crafting writes this message; we read it here.
        app.add_message::<CraftCompleted>();

        app.init_resource::<CraftRevision>();
        app.init_resource::<PendingCraft>();
        app.init_resource::<RecipeIndex>();

        // Translate legacy UI selection before budget arbitration. This phase
        // never consumes ingredients or advances work.
        app.add_systems(
            SimulationIngress,
            process_pending_craft.in_set(SimSet::Activity),
        );

        // Craft completion: reads CraftCompleted messages emitted by
        // tick_crafting and spawns the result item.
        app.add_systems(
            SimulationActivity,
            process_craft_completions
                .in_set(SimSet::Activity)
                .after(tick_crafting),
        );
    }
}

/// Read `CraftCompleted` messages and call `complete_craft` for each.
///
/// This is an exclusive system because `complete_craft` needs `&mut World` to
/// spawn result items and manipulate inventory.  Craft completions are rare
/// (not every frame), so the scheduling cost is negligible.
fn process_craft_completions(
    world: &mut World,
    mut cursor: Local<bevy_ecs::message::MessageCursor<CraftCompleted>>,
) {
    let completed: Vec<(Entity, Entity)> = {
        cursor
            .read(world.resource::<bevy_ecs::message::Messages<CraftCompleted>>())
            .map(|c| (c.crafter, c.craft_entity))
            .collect()
    };
    for (player, craft_e) in &completed {
        let outcome = match complete_craft(world, *player, *craft_e) {
            Ok(item) => CraftOutcome::Completed { item },
            Err(reason) => CraftOutcome::Failed { reason },
        };
        world.resource_mut::<CraftRevision>().last_result = Some(outcome);
        world.resource_mut::<CraftRevision>().revision += 1;
    }
}
