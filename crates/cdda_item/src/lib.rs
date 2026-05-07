//! # cdda_item — Items, inventory, containers, pockets domain crate
//!
//! Owns item state (`StackCount`, `Spoilable`, etc.), container tags
//! (`Sealed`, `Rigid`), inventory relationships (`InsideContainer`,
//! `WieldedBy`, `WornOn`), and the pocket system.
//!
//! Depends only on `cdda_core` and `bevy_ecs`.

pub mod components;
pub mod plugin;
