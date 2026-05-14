use crate::core::coords::origins::{Abs, Rel};
use crate::core::coords::scales::{Ms, Om, Omt, Sm, OMT_PER_OM, TILES_PER_OMT, TILES_PER_SM};
use crate::core::coords::z_level::ZLevel;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// A typed coordinate parameterized by **scale** and **origin**.
///
/// Types with different `Scale` or `Origin` parameters do not coerce.
/// The compiler catches mixing them at compile time.
///
/// # Arithmetic
///
/// All horizontal division uses `div_euclid` / `rem_euclid` to correctly
/// handle negative coordinates. Rust's `/` truncates toward zero, which
/// would silently assign negative positions to the wrong submap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos<Scale, Origin> {
    pub x: i32,
    pub y: i32,
    pub z: ZLevel,
    #[serde(skip)]
    #[allow(dead_code)]
    _scale: std::marker::PhantomData<Scale>,
    #[serde(skip)]
    #[allow(dead_code)]
    _origin: std::marker::PhantomData<Origin>,
}

impl<Scale, Origin> Default for Pos<Scale, Origin> {
    fn default() -> Self {
        Self::new(0, 0, ZLevel::new(0))
    }
}

impl<Scale, Origin> Pos<Scale, Origin> {
    pub const fn new(x: i32, y: i32, z: ZLevel) -> Self {
        Self {
            x,
            y,
            z,
            _scale: std::marker::PhantomData,
            _origin: std::marker::PhantomData,
        }
    }

    /// Convert to a different origin type, keeping the same scale and values.
    pub fn with_origin<NewOrigin>(self) -> Pos<Scale, NewOrigin> {
        Pos::new(self.x, self.y, self.z)
    }

    /// Convert to a different scale type, keeping the same origin and values.
    pub fn with_scale<NewScale>(self) -> Pos<NewScale, Origin> {
        Pos::new(self.x, self.y, self.z)
    }
}

// ---------------------------------------------------------------------------
// Coordinate arithmetic — WorldPos ↔ SubmapPos + SubmapLocal
// ---------------------------------------------------------------------------

impl Pos<Ms, Abs> {
    /// Decompose an absolute map-square position into its containing submap
    /// and local offset within that submap.
    ///
    /// Uses `div_euclid`/`rem_euclid` so negative coordinates map to the
    /// correct submap.
    pub fn to_submap(self) -> (Pos<Sm, Abs>, Pos<Ms, Rel>) {
        let sx = self.x.div_euclid(TILES_PER_SM);
        let sy = self.y.div_euclid(TILES_PER_SM);
        let lx = self.x.rem_euclid(TILES_PER_SM);
        let ly = self.y.rem_euclid(TILES_PER_SM);
        (Pos::new(sx, sy, self.z), Pos::new(lx, ly, self.z))
    }

    /// Reconstruct an absolute map-square position from a submap + local offset.
    ///
    /// z passes through unchanged (z does not participate in horizontal scaling).
    pub fn from_submap(sm: Pos<Sm, Abs>, local: Pos<Ms, Rel>) -> Self {
        Pos::new(
            sm.x * TILES_PER_SM + local.x,
            sm.y * TILES_PER_SM + local.y,
            sm.z,
        )
    }

    /// Convert to overmap-terrain scale (1 OMT = 24×24 world tiles).
    ///
    /// z passes through unchanged.
    pub fn to_omt(self) -> Pos<Omt, Abs> {
        Pos::new(
            self.x.div_euclid(TILES_PER_OMT),
            self.y.div_euclid(TILES_PER_OMT),
            self.z,
        )
    }

    /// Construct from an OMT position + local offset within the OMT.
    pub fn from_omt(omt: Pos<Omt, Abs>, local: Pos<Ms, Rel>) -> Self {
        Pos::new(
            omt.x * TILES_PER_OMT + local.x,
            omt.y * TILES_PER_OMT + local.y,
            omt.z,
        )
    }

    /// Convert to overmap scale (1 OM = 180×180 omts).
    pub fn to_om(self) -> Pos<Om, Abs> {
        let tiles_per_om = OMT_PER_OM * TILES_PER_OMT;
        Pos::new(
            self.x.div_euclid(tiles_per_om),
            self.y.div_euclid(tiles_per_om),
            self.z,
        )
    }
}

// ---------------------------------------------------------------------------
// Arithmetic trait impls
// ---------------------------------------------------------------------------

impl<Scale, Origin> Add for Pos<Scale, Origin> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Pos::new(self.x + rhs.x, self.y + rhs.y, self.z.add(rhs.z))
    }
}

impl<Scale, Origin> Sub for Pos<Scale, Origin> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Pos::new(self.x - rhs.x, self.y - rhs.y, self.z.sub(rhs.z))
    }
}

// ---------------------------------------------------------------------------
// Distance helpers
// ---------------------------------------------------------------------------

impl<Scale, Origin> Pos<Scale, Origin> {
    /// 3D Manhattan distance (includes z).
    /// Sum of absolute differences on all three axes.
    pub fn dist_manhattan(self, other: Self) -> u32 {
        (self.x.abs_diff(other.x) + self.y.abs_diff(other.y) + self.z.0.abs_diff(other.z.0) as u32)
            as u32
    }

    /// 2D Manhattan distance (horizontal only, ignores z).
    /// Use this when z-level differences are irrelevant (e.g. pathfinding on one floor).
    pub fn dist_manhattan_2d(self, other: Self) -> u32 {
        (self.x.abs_diff(other.x) + self.y.abs_diff(other.y)) as u32
    }

    /// 3D Chebyshev (max-axis) distance including z.
    /// The maximum of |dx|, |dy|, |dz|.
    pub fn dist_chebyshev(self, other: Self) -> u32 {
        self.x
            .abs_diff(other.x)
            .max(self.y.abs_diff(other.y))
            .max(self.z.0.abs_diff(other.z.0) as u32) as u32
    }

    /// 2D Chebyshev distance (horizontal only, ignores z).
    pub fn dist_chebyshev_2d(self, other: Self) -> u32 {
        self.x.abs_diff(other.x).max(self.y.abs_diff(other.y)) as u32
    }

    /// 3D squared Euclidean distance (includes z).
    /// Useful for range comparisons without sqrt.
    pub fn dist_sq(self, other: Self) -> u64 {
        let dx = self.x as i64 - other.x as i64;
        let dy = self.y as i64 - other.y as i64;
        let dz = self.z.0 as i64 - other.z.0 as i64;
        (dx * dx + dy * dy + dz * dz) as u64
    }

    /// 2D squared Euclidean distance (horizontal only, ignores z).
    pub fn dist_sq_2d(self, other: Self) -> u64 {
        let dx = self.x as i64 - other.x as i64;
        let dy = self.y as i64 - other.y as i64;
        (dx * dx + dy * dy) as u64
    }
}

// ---------------------------------------------------------------------------
// Conversions between related coordinate types
// ---------------------------------------------------------------------------

impl Pos<Sm, Abs> {
    /// Convert SubmapPos to tile-scale WorldPos (top-left corner of the submap).
    pub fn to_worldpos(self) -> Pos<Ms, Abs> {
        Pos::new(self.x * TILES_PER_SM, self.y * TILES_PER_SM, self.z)
    }
}

impl Pos<Omt, Abs> {
    /// Convert OmtPos to tile-scale WorldPos (top-left corner of the OMT).
    pub fn to_worldpos(self) -> Pos<Ms, Abs> {
        Pos::new(self.x * TILES_PER_OMT, self.y * TILES_PER_OMT, self.z)
    }

    /// Convert OmtPos to SubmapPos (top-left submap of the OMT).
    pub fn to_submappos(self) -> Pos<Sm, Abs> {
        Pos::new(self.x * 2, self.y * 2, self.z)
    }
}

impl Pos<Om, Abs> {
    /// Convert OmPos to tile-scale WorldPos (top-left corner of the overmap).
    pub fn to_worldpos(self) -> Pos<Ms, Abs> {
        let tiles_per_om = OMT_PER_OM * TILES_PER_OMT;
        Pos::new(self.x * tiles_per_om, self.y * tiles_per_om, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::WorldPos;

    #[test]
    fn test_worldpos_to_submap_positive() {
        let wp = WorldPos::new(13, 25, ZLevel::new(0));
        let (sm, local) = wp.to_submap();
        assert_eq!(sm.x, 1);
        assert_eq!(sm.y, 2);
        assert_eq!(local.x, 1);
        assert_eq!(local.y, 1);
        assert_eq!(sm.z, ZLevel::new(0));
    }

    #[test]
    fn test_worldpos_to_submap_negative() {
        // -1 should go to submap -1, local 11 (not submap 0, local -1)
        let wp = WorldPos::new(-1, -1, ZLevel::new(0));
        let (sm, local) = wp.to_submap();
        assert_eq!(sm.x, -1);
        assert_eq!(sm.y, -1);
        assert_eq!(local.x, 11);
        assert_eq!(local.y, 11);
    }

    #[test]
    fn test_submap_roundtrip() {
        let wp = WorldPos::new(42, -17, ZLevel::new(3));
        let (sm, local) = wp.to_submap();
        let reconstructed = WorldPos::from_submap(sm, local);
        assert_eq!(reconstructed, wp);
    }

    #[test]
    fn test_worldpos_to_omt() {
        let wp = WorldPos::new(50, 50, ZLevel::new(0));
        let omt = wp.to_omt();
        assert_eq!(omt.x, 2); // 50 / 24 = 2 (euclid)
        assert_eq!(omt.y, 2);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = WorldPos::new(0, 0, ZLevel::new(0));
        let b = WorldPos::new(3, 4, ZLevel::new(0));
        assert_eq!(a.dist_manhattan(b), 7);
    }

    #[test]
    fn test_3d_distance() {
        let a = WorldPos::new(0, 0, ZLevel::new(0));
        let b = WorldPos::new(3, 4, ZLevel::new(5));
        assert_eq!(a.dist_chebyshev(b), 5);
    }
}
