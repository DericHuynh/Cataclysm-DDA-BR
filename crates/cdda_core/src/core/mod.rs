pub mod components;
pub mod coords;
pub mod damage;
pub mod error;
pub mod flags;
pub mod id;
pub mod stats;
pub mod units;

// Re-export the most common types at the core level
pub use damage::Damage;
pub use error::CoreError;
pub use flags::FlagSet;
pub use id::{DefCategory, DefId, DefIdx, GenId};
pub use stats::Stats;

// Re-export common unit and coordinate types
pub use coords::{BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos};
pub use coords::{Direction, Facing, ZLevel};
pub use coords::{VehicleMapPos, VehicleMountPos};
pub use units::{Energy, Length, Time, Volume, Weight};
