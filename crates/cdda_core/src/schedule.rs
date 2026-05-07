//! SystemSet labels for cross-crate ordering within the `Update` schedule.
//!
//! # Ordering
//!
//! ```ignore
//! app.configure_sets(Update, (GameSet::Input, GameSet::Sim, GameSet::Render).chain());
//! ```
//!
//! Systems in `cdda_input` → `GameSet::Input`
//! Systems in `cdda_sim`   → `GameSet::Sim`
//! Systems in `cdda_ui`    → `GameSet::Sim` (react to input)
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
