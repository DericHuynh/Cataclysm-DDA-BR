//! Cross-crate schedule labels. `GameSet` orders outer `Update` adapters:
//! Input → Sim (logical schedule driver) → Render. `SimSet` orders systems
//! inside the logical turn, action and activity schedules. Attaching a SimSet
//! to Update does not run simulation work. SimulationPlugin owns dispatch.

use bevy_ecs::prelude::SystemSet;
use bevy_ecs::schedule::ScheduleLabel;

/// One logical simulation turn. Run by `cdda_sim::runtime::SimulationPlugin`,
/// never once implicitly per render frame. Shared with world/replay adapters.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationTurn;

/// Pending commands, dispatched with available budget before actor selection.
/// May run between player commands without advancing world time.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationIngress;

/// Derived inventory/spatial state after committed actions and activity work.
/// Runs even when a player command uses the current turn's remaining moves.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationRefresh;

/// One selected actor's action, repeated within its world-turn AP budget.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationAction;

/// One selected actor’s activity work, sharing the action-point scheduler.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationActivity;

/// Selected actor while SimulationAction or SimulationActivity runs. Absent for isolated system tests.
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy)]
pub struct ActingEntity(pub bevy_ecs::entity::Entity);

/// Cross-crate system ordering sets for the `Update` schedule.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Raw input processing: `cdda_input::handle_raw_input`.
    Input,
    /// Outer driver for the headless logical simulation schedule.
    Sim,
    /// Rendering: tile drawing, UI overlay drawing.
    Render,
}

/// Ordered sets within logical simulation schedules. World phases grant AP
/// and update the world; the budget driver then dispatches selected actors
/// through IntentDeclare → IntentResolve or Activity work/completion.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Grant action points to all living actors.
    TurnTick,
    /// Player input + AI decisions → insert `ActionIntent` components.
    IntentDeclare,
    /// Sort intents by AP, resolve one-by-one with precondition validation.
    IntentResolve,
    /// Multi-turn activities (crafting, aiming, reading, etc.).
    Activity,
    /// Status effects tick.
    Effects,
    /// Natural healing.
    Healing,
    /// Bionic power drain.
    Bionics,
    /// Morale decay.
    Morale,
    /// Body temperature updates.
    Temperature,
    /// Vision / line-of-sight updates.
    Vision,
    /// Creature/item spawning.
    Spawning,
    /// Inventory bin maintenance.
    Inventory,
    /// Spatial index maintenance.
    SpatialUpdate,
}
