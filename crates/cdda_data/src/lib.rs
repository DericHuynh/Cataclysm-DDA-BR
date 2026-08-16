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

pub mod assets;
pub mod bridge;
pub mod def_kinds;
pub mod def_registry_resource;
pub mod def_world;
pub mod flags;
pub mod interner;
pub mod json_asset;
pub mod loader;
pub mod mod_info;
pub mod mod_layer;
pub mod patch;
pub mod populate_flags;
pub mod raw_values;
pub mod registry;
pub mod resolve;
pub mod roundtrip;
pub mod schema;
pub mod schema_gen;

pub use loader::Loader;
pub use registry::DefRegistry;
