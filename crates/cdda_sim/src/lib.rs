//! # cdda_sim — Game simulation
//!
//! The single workspace crate that owns every game-logic subsystem plus the
//! runtime harness (`AppState`, `TestBed`). Submodule layout:
//!
//! * `runtime` — `AppState`, `GameTime`, `TestBed`.
//! * `actor` — creature turn scheduling, movement, bionics, effects,
//!   healing, temperature, morale, vision.
//! * `ai` — monster/NPC decision making.
//! * `activity` — multi-turn player activities.
//! * `combat` — damage, hit/miss, melee, ranged.
//! * `crafting` — recipe lookup, component consumption, progress.
//! * `equipment` — wielding, wearing, encumbrance.
//! * `inventory` — stacks, invlets, binned lookups, item movement.
//! * `item` — `ItemPlugin` type registration.
//! * `noise` — 3D simplex noise matching CDDA master.
//!
//! Consumers reach into `cdda_sim::<area>::…` directly. There are no
//! deprecation shim crates — the consolidation migration is complete.
//!
//! ## Bevy deps
//! `bevy_ecs`, `bevy_reflect`, `bevy_app`, `bevy_state`, plus the
//! `cdda_core_types` and `cdda_components` workspace crates.

pub mod activity;
pub mod actor;
pub mod ai;
pub mod combat;
pub mod crafting;
pub mod equipment;
pub mod inventory;
pub mod item;
pub mod noise;
pub mod runtime;
