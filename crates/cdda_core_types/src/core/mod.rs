pub mod coords;
pub mod damage;
pub mod error;
pub mod flags;
pub mod id;
pub mod raw_defs;
pub mod raw_types;
pub mod units;

pub use damage::Damage;
pub use error::CoreError;
pub use flags::FlagSet;
pub use id::{DefCategory, DefId};

pub use coords::{BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos};
pub use coords::{Direction, Facing, ZLevel};
pub use coords::{VehicleMapPos, VehicleMountPos};
pub use units::{Energy, Length, Time, Volume, Weight};
