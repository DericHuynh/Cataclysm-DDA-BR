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
/// The canonical simulation phase order is expressed here as a chain so that
/// new systems only need `in_set(SimSet::Combat)` (for example) instead of
/// manually chaining `.after()` calls off specific function names.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSet {
    TurnTick,
    Ai,
    Movement,
    Combat,
    Effects,
    Healing,
    Bionics,
    Morale,
    Temperature,
    Vision,
    Spawning,
    Inventory,
    SpatialUpdate,
}
