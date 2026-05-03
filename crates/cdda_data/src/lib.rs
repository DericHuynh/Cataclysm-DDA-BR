//! # cdda_data — JSON loading and def registry
//!
//! Parse CDDA JSON files into typed Rust structs. Resolve `copy-from`
//! inheritance (including `extend`/`delete`/`relative`/`proportional` operations).
//! Expose `DefRegistry` as the single authoritative read-only store of all game
//! definitions.
//!
//! ## Two-pass loading
//! - **Pass 1:** Walk `data/` directories, parse all `.json` files into raw
//!   `serde_json::Value`s keyed by their `"type"` field.
//! - **Pass 2:** Deserialize each raw def into its typed struct, resolving
//!   `copy-from` inheritance chains topologically.
//!
//! This crate has **zero Bevy dependencies**.

pub mod loader;
pub mod mod_layer;
pub mod registry;
pub mod resolve;

// Re-export domain types from cdda_core for ergonomic access
pub use cdda_core::defs::*;
pub use cdda_core::types::*;
pub use cdda_core::units::{Energy, Length, Time, Volume, Weight};

pub use loader::Loader;
pub use registry::DefRegistry;
