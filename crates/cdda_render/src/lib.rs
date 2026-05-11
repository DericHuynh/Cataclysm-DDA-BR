//! # cdda_render — Bevy rendering crate
//!
//! Everything visual: tiles, UI menus, ASCII mode.
//! Reads cdda_core state; never writes it.
//!
//! # cdda_render — Bevy rendering crate
//!
//! Everything visual: tiles, UI menus, ASCII mode.
//! Reads cdda_core and cdda_context state; never writes it.

// Re-export everything from cdda_core — this includes context, input, etc.
pub use cdda_core::*;

// The render module.
pub mod render;
