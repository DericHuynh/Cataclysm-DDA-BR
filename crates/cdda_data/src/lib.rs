//! # cdda_data — JSON loading, def registry, ACL translation
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
//! ## Anti-Corruption Layer (ACL)
//! After loading and resolving copy-from, raw CDDA types are translated into
//! pure domain types via `crate::translate`. The rest of the game only sees
//! clean types from `cdda_core::templates` and `cdda_core::id`.
//!
//! This crate has **zero Bevy dependencies**.

pub mod loader;
pub mod mod_layer;
pub mod raw_defs;
pub mod raw_types;
pub mod registry;
pub mod resolve;
pub mod schema;

// Re-export raw CDDA types for backward compat during migration
pub use raw_defs::*;
pub use raw_types::*;

pub use loader::Loader;
pub use registry::DefRegistry;
