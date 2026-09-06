//! Canonical headless simulation wiring. The graphical app supplies adapters,
//! not a second copy of the simulation schedule.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_state::state::State;
use bevy_time::Time;
use cdda_components::activity::{ActivityPhase, ActivityProgress};
use cdda_components::actor::{ActionPoints, IsAlive, PlayerData};
use cdda_components::dev::DevPlayer;
use cdda_components::events::{ItemMoveEvent, ItemMoveResult};
use cdda_components::intent::{ActionIntent, ActionOutcomeState, ActionRequestCounter};
use cdda_components::schedule::{
    ActingEntity, GameSet, SimSet, SimulationAction, SimulationActivity, SimulationIngress,
    SimulationRefresh, SimulationTurn,
};
use cdda_components::sim::{GameTime, TurnAdvanced};
use cdda_core_types::sim_id::SimId;

use super::{clock::SimClock, state::AppState};
use crate::activity::plugin::ActivityPlugin;
use crate::actor::{
    bionics::tick_bionics,
    effects::effects_phase,
    healing::healing_phase,
    morale::tick_morale_decay,
    plugin::ActorPlugin,
    temperature::temperature_phase,
    turn::{tick_move_points, TurnQueue},
    vision::update_vision,
};
use crate::ai::plugin::AiPlugin;
use crate::crafting::{plugin::CraftingPlugin, systems::PendingCraft};
use crate::intent::plugin::IntentPlugin;
use crate::inventory::systems::{
    assign_invlets_system, build_inventory_bins, process_item_move_events, InventoryBin,
};
use crate::item::plugin::ItemPlugin;

/// How an outer application requests logical turns. None of these modes changes
/// the duration of a game turn: it is always one simulated second.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SimulationMode {
    /// Reuse player moves across input frames; advance time when moves run out.
    #[default]
    TurnBased,
    /// Only explicit `request_steps` or `step_simulation` calls advance time.
    Manual,
    /// Consume wall time with bounded catch-up, retaining unconsumed elapsed time.
    RealTime,
}

#[derive(Resource, Debug)]
pub struct SimulationControl {
    pub mode: SimulationMode,
    pub paused: bool,
    pub max_steps_per_update: u32,
    pending_steps: u32,
}

impl Default for SimulationControl {
    fn default() -> Self {
        Self {
            mode: SimulationMode::TurnBased,
            paused: false,
            max_steps_per_update: 8,
            pending_steps: 0,
        }
    }
}

impl SimulationControl {
    /// Queue explicit turns (also useful for headless scenarios). Requests are
    /// retained while paused; wall time accumulated during pause is discarded.
    pub fn request_steps(&mut self, count: u32) {
        self.pending_steps = self.pending_steps.saturating_add(count);
    }
}

#[derive(Resource, Default)]
struct ItemMoveWakeCursor(bevy_ecs::message::MessageCursor<ItemMoveEvent>);

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationControl>()
            .init_resource::<SimClock>()
            .init_resource::<GameTime>()
            .init_resource::<TurnQueue>()
            .init_resource::<InventoryBin>()
            .init_resource::<ItemMoveWakeCursor>()
            .add_message::<ItemMoveEvent>()
            .add_message::<ItemMoveResult>()
            .add_message::<TurnAdvanced>()
            .init_schedule(SimulationTurn)
            .init_schedule(SimulationIngress)
            .init_schedule(SimulationRefresh);
        app.configure_sets(
            Update,
            (GameSet::Input, GameSet::Sim, GameSet::Render).chain(),
        );
        app.configure_sets(
            SimulationTurn,
            (
                SimSet::TurnTick,
                SimSet::Effects,
                SimSet::Healing,
                SimSet::Bionics,
                SimSet::Morale,
                SimSet::Temperature,
                SimSet::Vision,
                SimSet::Spawning,
            )
                .chain(),
        );
        app.configure_sets(
            SimulationIngress,
            (SimSet::Activity, SimSet::Inventory).chain(),
        );
        app.configure_sets(
            SimulationRefresh,
            (SimSet::Inventory, SimSet::SpatialUpdate).chain(),
        );
        app.add_plugins((
            ActorPlugin,
            ItemPlugin,
            ActivityPlugin,
            AiPlugin,
            IntentPlugin,
            CraftingPlugin,
        ));
        app.add_systems(
            SimulationTurn,
            (
                tick_move_points.in_set(SimSet::TurnTick),
                effects_phase.in_set(SimSet::Effects),
                healing_phase.in_set(SimSet::Healing),
                tick_bionics.in_set(SimSet::Bionics),
                tick_morale_decay.in_set(SimSet::Morale),
                temperature_phase.in_set(SimSet::Temperature),
                update_vision.in_set(SimSet::Vision),
            ),
        );
        app.add_systems(
            SimulationIngress,
            process_item_move_events.in_set(SimSet::Inventory),
        );
        app.add_systems(
            SimulationRefresh,
            (assign_invlets_system, build_inventory_bins)
                .chain()
                .in_set(SimSet::Inventory),
        );
        app.add_systems(Update, drive_simulation.in_set(GameSet::Sim));
    }
}

fn simulation_enabled(world: &World) -> bool {
    !world.resource::<SimulationControl>().paused
        && world
            .get_resource::<State<AppState>>()
            .map_or(true, |state| *state.get() == AppState::InGame)
}

/// Advance one world turn, then arbitrate action and activity work by live AP.
/// Highest AP wins with stable identity tie-breaks. Activity-specific rules own
/// spending and any finishing remainder. A pass with no committed action/work
/// parks the actor. Each actor is bounded to 64 selections per dispatch and keeps
/// unused AP. Pause gates world, ingress, work and refresh schedules.
pub fn step_simulation(world: &mut World) -> bool {
    if !simulation_enabled(world) {
        return false;
    }
    world.run_schedule(SimulationTurn);
    dispatch_commands(world, false);
    true
}

fn dispatch_commands(world: &mut World, wait_for_player: bool) {
    world.run_schedule(SimulationIngress);
    run_action_budget(world, wait_for_player);
    world.run_schedule(SimulationRefresh);
}

fn is_player(world: &World, actor: Entity) -> bool {
    world.get::<PlayerData>(actor).is_some() || world.get::<DevPlayer>(actor).is_some()
}

fn player_has_moves(world: &mut World) -> bool {
    world
        .query_filtered::<&ActionPoints, (With<IsAlive>, Or<(With<PlayerData>, With<DevPlayer>)>)>()
        .iter(world)
        .any(|ap| ap.current > 0)
}

/// Highest AP first (SimId then Entity as stable tie-breaks, matching intent
/// collection order); skips actors parked during this dispatch.
fn select_actor(world: &mut World, spent: &[Entity], players_only: bool) -> Option<Entity> {
    let mut candidates: Vec<_> = world
        .query_filtered::<(Entity, &ActionPoints, Option<&SimId>), With<IsAlive>>()
        .iter(world)
        .filter(|(entity, ap, _)| {
            ap.current > 0
                && !spent.contains(entity)
                && (!players_only || is_player(world, *entity))
        })
        .map(|(entity, ap, id)| {
            (
                std::cmp::Reverse(ap.current),
                id.is_none(),
                id.map(|id| id.0).unwrap_or_default(),
                entity.to_bits(),
                entity,
            )
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next().map(|(.., entity)| entity)
}

/// Repeated action selection until budgets are exhausted. A completed action
/// may leave budget, so successful actors are re-selected; an actor whose
/// pass produced no committed action is parked for this dispatch. In turn-based
/// mode a new external player command can resume the budget on a later frame.
fn run_action_budget(world: &mut World, wait_for_player: bool) {
    let mut spent: Vec<Entity> = Vec::new();
    let actor_count = world
        .query_filtered::<Entity, With<IsAlive>>()
        .iter(world)
        .count();
    let mut selections = std::collections::HashMap::<Entity, u8>::new();
    for _ in 0..actor_count.saturating_mul(64) {
        // Master completes the avatar input loop before processing creatures.
        // A player parked with spare moves is awaiting another external command;
        // resume here next frame, without another world tick or AP grant.
        let players_only = wait_for_player && player_has_moves(world);
        let Some(actor) = select_actor(world, &spent, players_only) else {
            return;
        };
        world.insert_resource(ActingEntity(actor));
        let before = world.resource::<ActionRequestCounter>().last();
        let ap_before = world.get::<ActionPoints>(actor).unwrap().current;
        let progress_before = world
            .get::<ActivityProgress>(actor)
            .map(|p| (p.phase, p.moves_left));
        let control = world
            .get::<ActionIntent>(actor)
            .is_some_and(ActionIntent::is_activity_control);
        if world.get::<ActivityProgress>(actor).is_some() && !control {
            if crate::activity::lifecycle::ready(world, actor) {
                world.run_schedule(SimulationActivity);
            }
        } else {
            world.run_schedule(SimulationAction);
        }
        world.remove_resource::<ActingEntity>();
        let committed = world.resource::<ActionRequestCounter>().last() != before
            && world
                .get::<cdda_components::intent::ActionOutcome>(actor)
                .is_some_and(|outcome| outcome.state == ActionOutcomeState::Completed);
        let advanced = world
            .get::<ActionPoints>(actor)
            .is_some_and(|ap| ap.current < ap_before)
            || progress_before
                != world
                    .get::<ActivityProgress>(actor)
                    .map(|p| (p.phase, p.moves_left));
        let count = selections.entry(actor).or_default();
        *count += 1;
        if (!committed && !advanced) || *count >= 64 {
            spent.push(actor);
        }
    }
}

fn has_turn_work(world: &mut World, players_only: bool) -> bool {
    let item_moves = world.resource_scope(|world, mut cursor: Mut<ItemMoveWakeCursor>| {
        cursor
            .0
            .read(world.resource::<Messages<ItemMoveEvent>>())
            .count()
            > 0
    });
    world
        .query_filtered::<Entity, (With<ActionIntent>, With<IsAlive>)>()
        .iter(world)
        .any(|actor| !players_only || is_player(world, actor))
        || world
            .query_filtered::<(Entity, &ActivityProgress), With<IsAlive>>()
            .iter(world)
            .any(|(actor, activity)| {
                (!players_only || is_player(world, actor))
                    && matches!(
                        activity.phase,
                        ActivityPhase::Pending | ActivityPhase::Active
                    )
            })
        || world.resource::<PendingCraft>().0.is_some()
        || item_moves
}

/// Outer driver runs after input commands have been applied and before display
/// extraction. Command dispatch can resume a player budget without ticking time.
pub fn drive_simulation(world: &mut World) {
    if !simulation_enabled(world) {
        world.resource_mut::<SimClock>().reset();
        return;
    }
    let (mode, limit, explicit) = {
        let mut control = world.resource_mut::<SimulationControl>();
        let limit = control.max_steps_per_update;
        let explicit = control.pending_steps.min(limit);
        control.pending_steps -= explicit;
        (control.mode, limit, explicit)
    };
    let automatic = match mode {
        SimulationMode::RealTime => {
            let elapsed = world
                .get_resource::<Time>()
                .map(|t| t.delta())
                .unwrap_or_default();
            let mut clock = world.resource_mut::<SimClock>();
            clock.advance(elapsed);
            clock.take_steps(limit - explicit)
        }
        SimulationMode::TurnBased => {
            world.resource_mut::<SimClock>().reset();
            let player_ready = player_has_moves(world);
            if explicit == 0 && limit > 0 && has_turn_work(world, player_ready) {
                if !player_ready {
                    world.run_schedule(SimulationTurn);
                }
                dispatch_commands(world, true);
            }
            0
        }
        SimulationMode::Manual => {
            world.resource_mut::<SimClock>().reset();
            0
        }
    };
    for _ in 0..explicit + automatic {
        if !step_simulation(world) {
            break;
        }
    }
}
