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
//!
//! # Coordinate hierarchy (least to most granular)
//!
//! | Type         | Scale | Origin | Unit represents      | Example range         |
//! |--------------|-------|--------|----------------------|-----------------------|
//! | `WorldPos`   | `Ms`  | `Abs`  | 1 map tile           | unbounded             |
//! | `SubmapPos`  | `Sm`  | `Abs`  | 1 submap (12×12)     | world / 12            |
//! | `OmtPos`     | `Omt` | `Abs`  | 1 OMT (24×24)        | world / 24            |
//! | `OmPos`      | `Om`  | `Abs`  | 1 overmap (4320×4320)| world / 4320          |
//! | `SubmapLocal`| `Ms`  | `Rel`  | 1 tile, submap-local | 0..=11 on x and y     |
//! | `BubblePos`  | `Ms`  | `Bubble`| 1 tile, bubble-local | viewport-sized        |

mod direction;
mod origins;
mod pos;
mod scales;
mod z_level;

pub use direction::{Direction, Facing};
pub use origins::{Abs, Bubble, Rel};
pub use pos::Pos;
pub use scales::{Ms, Om, Omt, Sm, OMT_PER_OM, TILES_PER_OMT, TILES_PER_SM};
pub use z_level::ZLevel;

// ---- Type aliases ----

/// Absolute world position in map-square (tile) units.
///
/// This is the **most granular** coordinate type — 1 unit = 1 map tile.
/// It is the "ground truth" coordinate system against which all other
/// coordinate types are defined. All stored positions should use this type.
///
/// **Scale:** [`Ms`] — one unit is one map tile.
/// **Origin:** [`Abs`] — global origin, never shifts as the player moves.
///
/// # Relationships to other types
///
/// | Conversion          | Method                   | Result               |
/// |---------------------|--------------------------|----------------------|
/// | → submap + local    | [`Pos::to_submap`]       | `(SubmapPos, SubmapLocal)` |
/// | → OMT               | [`Pos::to_omt`]          | `OmtPos`             |
/// | → overmap           | [`Pos::to_om`]           | `OmPos`              |
///
/// **See also:** [`SubmapPos`], [`SubmapLocal`], [`OmtPos`], [`OmPos`], [`BubblePos`]
pub type WorldPos = Pos<Ms, Abs>;

/// Absolute submap position.
///
/// Identifies which 12×12-tile submap a position falls in. One unit equals
/// one submap (12 tiles × 12 tiles). To get the position of a specific tile
/// within the submap, pair this with [`SubmapLocal`].
///
/// **Scale:** [`Sm`] — one unit is one submap (12×12 tiles).
/// **Origin:** [`Abs`] — global origin.
///
/// # Relationships
///
/// - `WorldPos = SubmapPos * 12 + SubmapLocal`
/// - 2 submaps span 1 [`OmtPos`] (`OmtPos = SubmapPos / 2` with `div_euclid`)
///
/// **See also:** [`WorldPos`], [`SubmapLocal`], [`OmtPos`]
pub type SubmapPos = Pos<Sm, Abs>;

/// Local position within a submap, in tile units.
///
/// Represents an offset inside a single 12×12 submap. Valid range is
/// `0..=11` on both x and y axes when paired with a well-formed [`WorldPos`],
/// though the type itself does not enforce this at the type level.
///
/// This is the only commonly-used type with [`Rel`] origin that represents
/// a **local** offset (as opposed to vehicle-relative coordinates).
///
/// **Scale:** [`Ms`] — one unit is one map tile.
/// **Origin:** [`Rel`] — relative to the submap's top-left corner.
///
/// **See also:** [`SubmapPos`], [`WorldPos`]
pub type SubmapLocal = Pos<Ms, Rel>;

/// Position within the reality bubble.
///
/// Defines a position relative to the top-left corner of the currently-loaded
/// reality bubble. Used primarily for rendering, FOV calculations, and any
/// operation that needs coordinates relative to the visible viewport.
///
/// **Scale:** [`Ms`] — one unit is one map tile.
/// **Origin:** [`Bubble`] — relative to the reality bubble's top-left.
///
/// # Notes
///
/// - `BubblePos` values are transient — they change as the player moves the
///   bubble. Never store positions in this format.
/// - To convert to/from [`WorldPos`], add or subtract the bubble's current
///   origin offset.
///
/// **See also:** [`WorldPos`], [`SubmapPos`]
pub type BubblePos = Pos<Ms, Bubble>;

/// Overmap terrain position.
///
/// Identifies which 24×24-tile overmap terrain (OMT) square a position
/// falls in. One OMT is 2×2 submaps or 24×24 tiles. This is the scale used
/// for overmap generation and terrain chunk storage.
///
/// **Scale:** [`Omt`] — one unit is one OMT (24×24 tiles).
/// **Origin:** [`Abs`] — global origin.
///
/// # Relationships
///
/// - `WorldPos = OmtPos * 24 + (tile offset within OMT)`
/// - `SubmapPos = OmtPos * 2` (top-left submap of the OMT)
/// - [`OMT_PER_OM`] (180) overmap terrains span 1 overmap
///
/// **See also:** [`WorldPos`], [`SubmapPos`], [`OmPos`]
pub type OmtPos = Pos<Omt, Abs>;

/// Overmap position.
///
/// Identifies which overmap segment a position falls in. One overmap is
/// 180×180 omts = 4320×4320 tiles. This is the coarsest absolute coordinate
/// type in the system, used for inter-overmap travel and world persistence.
///
/// **Scale:** [`Om`] — one unit is one overmap (180×180 omts).
/// **Origin:** [`Abs`] — global origin.
///
/// # Relationships
///
/// - `WorldPos = OmPos * (180 * 24) + (tile offset within overmap)`
/// - `OmtPos = OmPos * 180` (top-left OMT of the overmap)
///
/// **See also:** [`WorldPos`], [`OmtPos`], [`SubmapPos`]
pub type OmPos = Pos<Om, Abs>;

/// Vehicle mount coordinates (facing east, relative to origin).
///
/// Represents a position relative to a vehicle's origin point, used for
/// defining where parts are mounted on the vehicle frame. These coordinates
/// assume the vehicle is facing east (the "canonical" orientation) and do
/// **not** account for vehicle rotation.
///
/// **Scale:** [`Ms`] — one unit is one map tile.
/// **Origin:** [`Rel`] — relative to the vehicle's mount origin.
///
/// **See also:** [`VehicleMapPos`]
pub type VehicleMountPos = Pos<Ms, Rel>;

/// Vehicle map-square coordinates (accounting for facing).
///
/// Like [`VehicleMountPos`], but adjusted for the vehicle's current facing
/// direction. Use this when you need the actual tile position of a vehicle
/// part in world-relative terms, after accounting for rotation.
///
/// **Scale:** [`Ms`] — one unit is one map tile.
/// **Origin:** [`Rel`] — relative to the vehicle's origin, facing-adjusted.
///
/// **See also:** [`VehicleMountPos`]
pub type VehicleMapPos = Pos<Ms, Rel>;
