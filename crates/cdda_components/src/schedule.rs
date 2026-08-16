//! SystemSet labels for cross-crate ordering within the `Update` schedule.
//!
//! # Ordering
//!
//! ```ignore
//! app.configure_sets(Update, (GameSet::Input, GameSet::Sim, GameSet::Render).chain());
//! app.configure_sets(Update, SimSet::ordered().in_set(GameSet::Sim));
//! ```
//!
//! Systems in `cdda_input` → `GameSet::Input`
//! Systems in `cdda_sim`   → `GameSet::Sim` via one of the `SimSet` variants
//! Systems in `cdda_screen`    → `GameSet::Sim` (react to input)
//! Systems in `cdda_render`→ `GameSet::Render`

use bevy_ecs::prelude::SystemSet;

/// Cross-crate system ordering sets for the `Update` schedule.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Raw input processing: `cdda_input::handle_raw_input`.
    Input,
    /// Simulation and UI logic: turn tick, AI, movement, menus.
    Sim,
    /// Rendering: tile drawing, UI overlay drawing.
    Render,
}

/// Fine-grained ordered sets within `GameSet::Sim`.
///
/// Reflects CDDA's turn order: grant AP → declare intents → resolve by
/// speed priority → process multi-turn activities → tick effects/healing/etc.
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
