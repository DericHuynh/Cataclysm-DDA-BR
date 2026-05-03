//! # Coordinate system
//!
//! Every coordinate has two independent type parameters:
//! - **Scale** — what one unit represents: `Ms` (map square), `Sm` (submap = 12×12),
//!   `Omt` (overmap terrain = 24×24), `Om` (overmap = 180×180 omts).
//! - **Origin** — what (0, 0) means: `Abs` (global), `Bubble` (reality bubble),
//!   `Rel` (generic relative — vehicle mounts, submap-local offsets).
//!
//! Types do not coerce. Compiler errors on type mismatch prevent the coordinate
//! confusion bugs that have plagued CDDA for years.
//!
//! All horizontal division uses `div_euclid` / `rem_euclid`, never `/` and `%`.

mod scales;
mod origins;
mod pos;
mod z_level;
mod direction;

pub use scales::{Ms, Sm, Omt, Om};
pub use origins::{Abs, Bubble, Rel};
pub use pos::Pos;
pub use z_level::ZLevel;
pub use direction::{Direction, Facing};

// ---- Type aliases ----

/// Absolute map-square position (the "world position").
pub type WorldPos = Pos<Ms, Abs>;

/// Which 12×12 submap.
pub type SubmapPos = Pos<Sm, Abs>;

/// Offset within a submap, 0..=11 on x and y.
pub type SubmapLocal = Pos<Ms, Rel>;

/// Position within the reality bubble.
pub type BubblePos = Pos<Ms, Bubble>;

/// Overmap terrain position (1 unit = 24×24 tiles).
pub type OmtPos = Pos<Omt, Abs>;

/// Overmap position (1 unit = 180×180 omts).
pub type OmPos = Pos<Om, Abs>;

/// Vehicle mount coordinates (facing east, relative to origin).
pub type VehicleMountPos = Pos<Ms, Rel>;

/// Vehicle map-square coordinates (accounting for facing).
pub type VehicleMapPos = Pos<Ms, Rel>;
