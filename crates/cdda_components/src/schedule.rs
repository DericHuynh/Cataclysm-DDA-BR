//! Cross-crate schedule labels. `GameSet` orders outer `Update` adapters:
//! Input → Sim (logical schedule driver) → Render. `SimSet` orders systems
//! INSIDE `SimulationTurn`; attaching a SimSet to Update does not run it as
//! simulation work. `cdda_sim::runtime::SimulationPlugin` owns both chains.

use bevy_ecs::prelude::SystemSet;
use bevy_ecs::schedule::ScheduleLabel;

/// One logical simulation turn. Run by `cdda_sim::runtime::SimulationPlugin`,
/// never once implicitly per render frame. Shared with world/replay adapters.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationTurn;

/// One selected actor's action, repeated within its world-turn AP budget.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationAction;

/// Selected actor while SimulationAction runs. Absent for isolated system tests.
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

/// Fine-grained ordered sets within `SimulationTurn`.
///
/// Current phase order: grant AP → declare intents → resolve by speed priority
/// → activities → effects/healing/etc. Repeated AP-budget action scheduling is
/// a separate pending extension, not implied by these phase labels.
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
