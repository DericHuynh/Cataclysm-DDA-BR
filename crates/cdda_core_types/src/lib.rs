//! # cdda_core_types — Pure value types for CDDA
//!
//! Zero `bevy_ecs` dependency. Coords, units, IDs, damage, flags,
//! RNG, and raw definition types. Used by both `cdda_core` and
//! downstream crates.

pub mod core;
pub mod rng;
pub mod sim_id;
pub mod wyrand;
