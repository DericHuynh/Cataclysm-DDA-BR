//! # cdda_core — Domain types and units
//!
//! The game's domain layer: value objects (Volume, Weight, WorldPos),
//! entity definitions (ItemDef, MonsterDef, TerrainDef), and ID types (DefId<T>).
//!
//! No IO, no Bevy. This is the lowest crate in the dependency graph.
//!
//! This is the lowest crate in the dependency graph.
//! Everything flows upward from here.

pub mod coords;
pub mod damage;
pub mod defs;
pub mod error;
pub mod flags;
pub mod rng;
pub mod stats;
pub mod types;
pub mod units;

// Re-export key types for ergonomic access.
// -- Coords and geometry
pub use coords::{BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos};
pub use coords::{Direction, Facing, ZLevel};
pub use coords::{VehicleMapPos, VehicleMountPos};

// -- Pure utilities
pub use damage::Damage;
pub use error::CoreError;
pub use flags::FlagSet;
pub use stats::Stats;
pub use units::{Energy, Length, Time, Volume, Weight};

// -- Domain definitions (moved from cdda_data)
pub use defs::*;
pub use types::*;
