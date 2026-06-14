//! Context actions — the authoritative list of available actions for the
//! current UI context.
//!
//! Populated via `OnEnter` state transitions (firing instantly, once per
//! screen entry) and `Changed<T>` resource watchers for dynamic updates.
//! Renderers read the resource to display key hints — no per-frame systems.
//!
//! The canonical type now lives in `cdda_components::context`. This module
//! re-exports it under its old name so `use cdda_context::actions::ContextActions;`
//! paths continue to compile (and the same struct is now a Bevy Resource
//! regardless of which path the user imports it from).

pub use cdda_components::context::{ContextAction, ContextActions};
