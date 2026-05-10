//! # cdda_render — Bevy rendering crate
//!
//! Everything visual: tiles, UI menus, ASCII mode.
//! Reads cdda_core state; never writes it.
//!
//! Re-exports all needed types from cdda_core so that render files
//! can keep their existing `crate::` import paths unchanged.

// Re-export everything from cdda_core so `crate::context::ctx::Ctx` etc. resolve.
pub use cdda_core::*;

// The render module — copied from cdda_core/src/render.
pub mod render;
