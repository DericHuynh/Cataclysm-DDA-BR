//! # cdda_defs_raw — Raw JSON def structs (the typed AST layer)
//!
//! Each file under `src/raw_defs/` mirrors one `"type"` value that appears in
//! `data/core/*.json`. These structs are the typed AST output of the data
//! pipeline's first pass: JSON → `RawFoo` struct → resolved `Foo` struct →
//! `DefRegistry` → Bevy `DefWorld`.
//!
//! ## Layer 1.5
//!
//! `cdda_defs_raw` sits between `cdda_core_types` (value types, units, IDs)
//! and `cdda_data` (the resolver + registry). It is intentionally Bevy-free
//! and logic-free: it only deserializes JSON into typed Rust.
//!
//! ## Module layout
//!
//! Every `data/core/<type>.json` has a sibling `raw_defs::<type>` module here.
//! New def types go through the same six touch points (see
//! `crates/cdda_data/AGENTS.md`): add a `raw_defs/<type>.rs`, register in
//! `def_kinds.rs`, add a `DefRegistry` field, add a `def_world.rs` builder,
//! regenerate the schema in `data/schemas/`.
//!
//! ## Bevy deps
//!
//! None. This crate is pure serde + schemars.

#![allow(clippy::all)]

pub mod raw_defs;
pub mod raw_types;

// Re-export the shared enums from cdda_core_types so that consumers of
// `cdda_defs_raw::raw_defs` continue to see `LocalizedString` and the like
// without an extra `cdda_core_types` import.
pub use crate::raw_types::*;
