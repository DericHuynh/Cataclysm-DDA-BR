//! Canonical headless simulation wiring. The graphical app supplies adapters,
//! not a second copy of the simulation schedule.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_state::state::State;
use bevy_time::Time;
use cdda_components::activity::{ActivityPhase, ActivityProgress};
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::events::ItemMoveEvent;
use cdda_components::intent::{ActionIntent, ActionOutcomeState, ActionRequestCounter};
use cdda_components::schedule::{ActingEntity, GameSet, SimSet, SimulationAction, SimulationTurn};
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
    /// Wait for declared actions; ongoing activities advance one turn per update.
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
            .add_message::<TurnAdvanced>()
            .init_schedule(SimulationTurn);
        app.configure_sets(
            Update,
            (GameSet::Input, GameSet::Sim, GameSet::Render).chain(),
        );
        app.configure_sets(
            SimulationTurn,
            (
                SimSet::TurnTick,
                SimSet::Activity,
                SimSet::Effects,
                SimSet::Healing,
                SimSet::Bionics,
                SimSet::Morale,
                SimSet::Temperature,
                SimSet::Vision,
                SimSet::Spawning,
                SimSet::Inventory,
                SimSet::SpatialUpdate,
            )
                .chain(),
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
                (
                    process_item_move_events,
                    assign_invlets_system,
                    build_inventory_bins,
                )
                    .chain()
                    .in_set(SimSet::Inventory),
            ),
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

/// Advance exactly one world turn using production systems and persistent
/// system state. After world processing, the AP-budget action loop lets each
/// actor act repeatedly while its budget lasts: the highest-AP eligible actor
/// is selected, `SimulationAction` runs (AI declare → collect → resolve for
/// that actor), and selection repeats until no eligible actor remains. Actors
/// that could not complete an action (rejected, planless) are not re-selected
/// within the same turn, and activities/pending actors are excluded — their
/// per-turn tick already consumed budget. Returns false without mutations when
/// paused/outside a running game. Headless apps may omit `AppState`.
pub fn step_simulation(world: &mut World) -> bool {
    if !simulation_enabled(world) {
        return false;
    }
    world.run_schedule(SimulationTurn);
    run_action_budget(world);
    true
}

/// Highest AP first (SimId then Entity as stable tie-breaks, matching intent
/// collection order); skips actors already given a chance this turn.
fn select_actor(world: &mut World, spent: &[Entity]) -> Option<Entity> {
    let mut candidates: Vec<_> = world
        .query_filtered::<(Entity, &ActionPoints, Option<&SimId>), (With<IsAlive>, Without<ActivityProgress>)>()
        .iter(world)
        .filter(|(entity, ap, _)| ap.current > 0 && !spent.contains(entity))
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
/// pass produced no committed action is parked for the rest of the turn.
fn run_action_budget(world: &mut World) {
    let mut spent: Vec<Entity> = Vec::new();
    for _ in 0..64 {
        let Some(actor) = select_actor(world, &spent) else {
            return;
        };
        world.insert_resource(ActingEntity(actor));
        let before = world.resource::<ActionRequestCounter>().last();
        world.run_schedule(SimulationAction);
        world.remove_resource::<ActingEntity>();
        let committed = world.resource::<ActionRequestCounter>().last() != before
            && world
                .get::<cdda_components::intent::ActionOutcome>(actor)
                .is_some_and(|outcome| outcome.state == ActionOutcomeState::Completed);
        if !committed {
            spent.push(actor);
        }
    }
}

fn has_turn_work(world: &mut World) -> bool {
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
        .next()
        .is_some()
        || world
            .query::<&ActivityProgress>()
            .iter(world)
            .any(|activity| activity.phase != ActivityPhase::Done)
        || world.resource::<PendingCraft>().0.is_some()
        || item_moves
}

/// Outer driver runs after input commands have been applied and before display
/// extraction. All simulation systems live in `SimulationTurn`, not `Update`.
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
            u32::from(explicit == 0 && limit > 0 && has_turn_work(world))
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
