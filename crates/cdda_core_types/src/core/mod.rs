pub mod coords;
pub mod damage;
pub mod error;
pub mod flags;
pub mod id;
pub mod units;

pub use damage::Damage;
pub use error::CoreError;
pub use flags::FlagSet;
pub use id::{DefCategory, DefId};

/// Absolute world position in map tiles — the "ground truth" coordinate.
/// See [`coords::WorldPos`] for details.
pub use coords::WorldPos;

/// Absolute submap position (1 unit = 12×12 tiles).
/// See [`coords::SubmapPos`] for details.
pub use coords::SubmapPos;

/// Local offset within a submap (0..=11 tiles on x and y).
/// See [`coords::SubmapLocal`] for details.
pub use coords::SubmapLocal;

/// Position relative to the reality bubble's top-left corner.
/// See [`coords::BubblePos`] for details.
pub use coords::BubblePos;

/// Overmap terrain position (1 unit = 24×24 tiles).
/// See [`coords::OmtPos`] for details.
pub use coords::OmtPos;

/// Overmap position (1 unit = 180×180 omts).
/// See [`coords::OmPos`] for details.
pub use coords::OmPos;

/// Generic position type parameterized by scale and origin.
/// See [`coords::Pos`] for details.
pub use coords::Pos;

/// Vehicle mount coordinates (facing east, relative to origin).
/// See [`coords::VehicleMountPos`] for details.
pub use coords::VehicleMountPos;

/// Vehicle map-square coordinates (accounting for facing).
/// See [`coords::VehicleMapPos`] for details.
pub use coords::VehicleMapPos;

pub use coords::{Direction, Facing, ZLevel};
pub use units::{Energy, Length, Time, Volume, Weight};
