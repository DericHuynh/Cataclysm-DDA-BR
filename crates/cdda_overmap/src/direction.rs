//! Overmap direction types — mirrors CDDA's `om_direction` and `cube_direction`.
//!
//! Rotation math matches `src/overmap.cpp` lines 2796–2920 exactly.

// ---------------------------------------------------------------------------
// Minimal RNG trait — avoids external dependency on `rand`
// ---------------------------------------------------------------------------

/// Minimal trait for random number generation used by direction randomization.
pub trait Rng {
    /// Returns a random `usize` in the half-open range `[0, max)`.
    fn random_usize(&mut self, max: usize) -> usize;
}

// ---------------------------------------------------------------------------
// OmDirection
// ---------------------------------------------------------------------------

/// Cardinal overmap direction.
///
/// Discriminants match the C++ `om_direction::type` enum:
/// `North = 0`, `East = 1`, `South = 2`, `West = 3`, `Invalid = -1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum OmDirection {
    Invalid = -1,
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl OmDirection {
    /// Number of valid cardinal directions.
    pub const SIZE: usize = 4;

    /// All four cardinal directions in rotation-index order.
    pub const ALL: [OmDirection; 4] = [
        OmDirection::North,
        OmDirection::East,
        OmDirection::South,
        OmDirection::West,
    ];

    // -- name ------------------------------------------------------------------

    /// Returns the human-readable English name (lowercase).
    ///
    /// Matches `om_direction::name()` in `overmap.cpp`.
    pub fn name(self) -> &'static str {
        match self {
            OmDirection::Invalid => "invalid",
            OmDirection::North => "north",
            OmDirection::East => "east",
            OmDirection::South => "south",
            OmDirection::West => "west",
        }
    }

    // -- index conversion ------------------------------------------------------

    /// Create an `OmDirection` from a rotation index.
    ///
    /// Wraps via `idx % SIZE` so that any `usize` maps to a cardinal direction.
    pub fn from_index(idx: usize) -> OmDirection {
        Self::ALL[idx % Self::SIZE]
    }

    /// Returns the rotation index: `North → 0`, `East → 1`, `South → 2`, `West → 3`.
    ///
    /// Returns `0` for `Invalid` (matching C++ behaviour where `invalid = -1` but the
    /// fallback in rotation functions effectively treats it as identity).
    pub fn to_index(self) -> usize {
        match self {
            OmDirection::North => 0,
            OmDirection::East => 1,
            OmDirection::South => 2,
            OmDirection::West => 3,
            OmDirection::Invalid => 0,
        }
    }

    // -- rotation ---------------------------------------------------------------

    /// Rotates a 2D point `(x, y)` by this direction.
    ///
    /// Rotation math (matching `om_direction::rotate(point)` in C++):
    /// - **North**: `( x,  y)` — identity
    /// - **East**:  `(-y,  x)` — 90° clockwise
    /// - **South**: `(-x, -y)` — 180°
    /// - **West**:  `( y, -x)` — 270° clockwise
    /// - **Invalid**: identity (same as North)
    pub fn rotate_point(self, p: (i32, i32)) -> (i32, i32) {
        let (x, y) = p;
        match self {
            OmDirection::Invalid | OmDirection::North => (x, y),
            OmDirection::East => (-y, x),
            OmDirection::South => (-x, -y),
            OmDirection::West => (y, -x),
        }
    }

    /// Rotates a 3D tripoint `(x, y, z)`, preserving the z-coordinate.
    ///
    /// Matches `om_direction::rotate(tripoint)` in C++:
    /// returns `tripoint( rotate( { p.xy() }, dir ), p.z )`.
    pub fn rotate_tripoint(self, p: (i32, i32, i32)) -> (i32, i32, i32) {
        let (rx, ry) = self.rotate_point((p.0, p.1));
        (rx, ry, p.2)
    }

    /// Returns a displacement vector: `rotate((0, -dist), dir)`.
    ///
    /// Matches `om_direction::displace(type dir, int dist)` in C++.
    pub fn displace(self, dist: i32) -> (i32, i32) {
        self.rotate_point((0, -dist))
    }

    // -- arithmetic -------------------------------------------------------------

    /// Adds two directions as rotation indices: `(self + other) % SIZE`.
    ///
    /// Matches `om_direction::add()` / `rotate_internal()` in C++.
    /// If `self` is `Invalid`, returns `Invalid`.
    pub fn add(self, other: OmDirection) -> OmDirection {
        match self {
            OmDirection::Invalid => OmDirection::Invalid,
            _ => OmDirection::from_index(self.to_index() + other.to_index()),
        }
    }

    /// Turn 90° to the left (counter-clockwise).
    ///
    /// Matches `om_direction::turn_left()` (`rotate_internal(dir, -1)`).
    pub fn turn_left(self) -> OmDirection {
        match self {
            OmDirection::Invalid => OmDirection::Invalid,
            _ => OmDirection::from_index((self.to_index() + Self::SIZE - 1) % Self::SIZE),
        }
    }

    /// Turn 90° to the right (clockwise).
    ///
    /// Matches `om_direction::turn_right()` (`rotate_internal(dir, +1)`).
    pub fn turn_right(self) -> OmDirection {
        match self {
            OmDirection::Invalid => OmDirection::Invalid,
            _ => OmDirection::from_index((self.to_index() + 1) % Self::SIZE),
        }
    }

    /// Randomly turn left or right (50/50).
    ///
    /// Matches `om_direction::turn_random()` in C++.
    pub fn turn_random(self, rng: &mut impl Rng) -> OmDirection {
        if rng.random_usize(2) == 0 {
            self.turn_left()
        } else {
            self.turn_right()
        }
    }

    /// Returns the opposite direction (180° rotation).
    ///
    /// Matches `om_direction::opposite()` (`rotate_internal(dir, size/2)`).
    pub fn opposite(self) -> OmDirection {
        match self {
            OmDirection::Invalid => OmDirection::Invalid,
            _ => OmDirection::from_index((self.to_index() + 2) % Self::SIZE),
        }
    }

    /// Returns a uniformly random cardinal direction.
    ///
    /// Matches `om_direction::random()` in C++.
    pub fn random(rng: &mut impl Rng) -> OmDirection {
        OmDirection::from_index(rng.random_usize(Self::SIZE))
    }

    /// Returns `true` if the two directions are parallel (same or opposite).
    ///
    /// Matches `om_direction::are_parallel()` in C++.
    pub fn are_parallel(self, other: OmDirection) -> bool {
        self == other || self == other.opposite()
    }
}

// ---------------------------------------------------------------------------
// CubeDirection
// ---------------------------------------------------------------------------

/// Six rectilinear directions (faces of a cube).
///
/// Matches C++ `cube_direction` enum in `cube_direction.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CubeDirection {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
    Above = 4,
    Below = 5,
    Last = 6,
}

impl From<CubeDirection> for OmDirection {
    /// Converts a `CubeDirection` to an `OmDirection`.
    ///
    /// Matches `om_direction::from_cube()` in `overmap.cpp`:
    /// - `North`/`East`/`South`/`West` map directly.
    /// - `Above`/`Below`/`Last` become `Invalid`.
    fn from(c: CubeDirection) -> OmDirection {
        match c {
            CubeDirection::North => OmDirection::North,
            CubeDirection::East => OmDirection::East,
            CubeDirection::South => OmDirection::South,
            CubeDirection::West => OmDirection::West,
            CubeDirection::Above | CubeDirection::Below | CubeDirection::Last => OmDirection::Invalid,
        }
    }
}

// ---------------------------------------------------------------------------
// Cardinal offset table
// ---------------------------------------------------------------------------

/// Unit offsets for the four cardinal directions in `OmDirection` index order.
///
/// Matches C++ `four_adjacent_offsets`:
/// `[point::north, point::east, point::south, point::west]` where
/// `point::north = (0, -1)`, `point::east = (1, 0)`,
/// `point::south = (0, 1)`, `point::west = (-1, 0)`.
///
/// Indexed as `four_adjacent_offsets[d.to_index()]`.
pub const FOUR_ADJACENT_OFFSETS: [(i32, i32); 4] = [
    (0, -1),  // north
    (1, 0),   // east
    (0, 1),   // south
    (-1, 0),  // west
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic "RNG" for testing that yields a fixed sequence of values.
    struct StepRng {
        value: usize,
    }

    impl Rng for StepRng {
        fn random_usize(&mut self, max: usize) -> usize {
            let v = self.value;
            self.value = self.value.wrapping_add(1);
            if max == 0 {
                return 0;
            }
            v % max
        }
    }

    #[test]
    fn name_strings() {
        assert_eq!(OmDirection::North.name(), "north");
        assert_eq!(OmDirection::East.name(), "east");
        assert_eq!(OmDirection::South.name(), "south");
        assert_eq!(OmDirection::West.name(), "west");
        assert_eq!(OmDirection::Invalid.name(), "invalid");
    }

    #[test]
    fn index_roundtrip() {
        for (i, &d) in OmDirection::ALL.iter().enumerate() {
            assert_eq!(d.to_index(), i);
            assert_eq!(OmDirection::from_index(i), d);
        }
    }

    #[test]
    fn rotate_point_identity() {
        // North / Invalid: identity
        let p = (3, -7);
        assert_eq!(OmDirection::North.rotate_point(p), p);
        assert_eq!(OmDirection::Invalid.rotate_point(p), p);
    }

    #[test]
    fn rotate_point_east() {
        // East: (-y, x)  — 90° clockwise
        assert_eq!(OmDirection::East.rotate_point((5, 2)), (-2, 5));
    }

    #[test]
    fn rotate_point_south() {
        // South: (-x, -y) — 180°
        assert_eq!(OmDirection::South.rotate_point((5, 2)), (-5, -2));
    }

    #[test]
    fn rotate_point_west() {
        // West: (y, -x) — 270° clockwise
        assert_eq!(OmDirection::West.rotate_point((5, 2)), (2, -5));
    }

    #[test]
    fn rotate_tripoint_preserves_z() {
        assert_eq!(
            OmDirection::East.rotate_tripoint((1, 2, 42)),
            (-2, 1, 42)
        );
    }

    #[test]
    fn displace_vectors() {
        // displace(dir, dist) = rotate((0, -dist), dir)
        assert_eq!(OmDirection::North.displace(3), (0, -3));
        assert_eq!(OmDirection::East.displace(3), (3, 0));
        assert_eq!(OmDirection::South.displace(3), (0, 3));
        assert_eq!(OmDirection::West.displace(3), (-3, 0));
    }

    #[test]
    fn add_directions() {
        assert_eq!(OmDirection::North.add(OmDirection::East), OmDirection::East);
        assert_eq!(OmDirection::East.add(OmDirection::East), OmDirection::South);
        assert_eq!(OmDirection::South.add(OmDirection::South), OmDirection::North); // 2+2=4%4=0
        assert_eq!(OmDirection::West.add(OmDirection::West), OmDirection::South); // 3+3=6%4=2
        // Invalid propagates
        assert_eq!(OmDirection::Invalid.add(OmDirection::North), OmDirection::Invalid);
        assert_eq!(OmDirection::North.add(OmDirection::Invalid), OmDirection::North);
        //  ^ because Invalid.to_index() == 0, so North + Invalid == North + 0 == North
    }

    #[test]
    fn turn_left_right() {
        assert_eq!(OmDirection::North.turn_left(), OmDirection::West);
        assert_eq!(OmDirection::North.turn_right(), OmDirection::East);
        assert_eq!(OmDirection::East.turn_left(), OmDirection::North);
        assert_eq!(OmDirection::East.turn_right(), OmDirection::South);
        assert_eq!(OmDirection::South.turn_left(), OmDirection::East);
        assert_eq!(OmDirection::South.turn_right(), OmDirection::West);
        assert_eq!(OmDirection::West.turn_left(), OmDirection::South);
        assert_eq!(OmDirection::West.turn_right(), OmDirection::North);
        // Invalid stays Invalid
        assert_eq!(OmDirection::Invalid.turn_left(), OmDirection::Invalid);
        assert_eq!(OmDirection::Invalid.turn_right(), OmDirection::Invalid);
    }

    #[test]
    fn opposite() {
        assert_eq!(OmDirection::North.opposite(), OmDirection::South);
        assert_eq!(OmDirection::East.opposite(), OmDirection::West);
        assert_eq!(OmDirection::South.opposite(), OmDirection::North);
        assert_eq!(OmDirection::West.opposite(), OmDirection::East);
        assert_eq!(OmDirection::Invalid.opposite(), OmDirection::Invalid);
    }

    #[test]
    fn are_parallel() {
        assert!(OmDirection::North.are_parallel(OmDirection::North));
        assert!(OmDirection::North.are_parallel(OmDirection::South));
        assert!(!OmDirection::North.are_parallel(OmDirection::East));
        assert!(!OmDirection::North.are_parallel(OmDirection::West));
    }

    #[test]
    fn turn_random() {
        let mut rng = StepRng { value: 0 };
        // value=0 → turn_left, value=1 → turn_right, value=2 → turn_left, …
        assert_eq!(OmDirection::North.turn_random(&mut rng), OmDirection::West); // left
        assert_eq!(OmDirection::North.turn_random(&mut rng), OmDirection::East); // right
        assert_eq!(OmDirection::North.turn_random(&mut rng), OmDirection::West); // left
    }

    #[test]
    fn random_direction() {
        let mut rng = StepRng { value: 0 };
        assert_eq!(OmDirection::random(&mut rng), OmDirection::North); // 0 % 4
        assert_eq!(OmDirection::random(&mut rng), OmDirection::East); // 1 % 4
        assert_eq!(OmDirection::random(&mut rng), OmDirection::South); // 2 % 4
        assert_eq!(OmDirection::random(&mut rng), OmDirection::West); // 3 % 4
        assert_eq!(OmDirection::random(&mut rng), OmDirection::North); // 4 % 4
    }

    #[test]
    fn four_adjacent_offsets_order() {
        // Index must match OmDirection::to_index()
        assert_eq!(FOUR_ADJACENT_OFFSETS[OmDirection::North.to_index()], (0, -1));
        assert_eq!(FOUR_ADJACENT_OFFSETS[OmDirection::East.to_index()], (1, 0));
        assert_eq!(FOUR_ADJACENT_OFFSETS[OmDirection::South.to_index()], (0, 1));
        assert_eq!(FOUR_ADJACENT_OFFSETS[OmDirection::West.to_index()], (-1, 0));
    }

    #[test]
    fn cube_to_om_conversion() {
        assert_eq!(OmDirection::from(CubeDirection::North), OmDirection::North);
        assert_eq!(OmDirection::from(CubeDirection::East), OmDirection::East);
        assert_eq!(OmDirection::from(CubeDirection::South), OmDirection::South);
        assert_eq!(OmDirection::from(CubeDirection::West), OmDirection::West);
        assert_eq!(OmDirection::from(CubeDirection::Above), OmDirection::Invalid);
        assert_eq!(OmDirection::from(CubeDirection::Below), OmDirection::Invalid);
        assert_eq!(OmDirection::from(CubeDirection::Last), OmDirection::Invalid);
    }

    #[test]
    fn discriminant_values_match_cpp() {
        // C++: invalid=-1, north=0, east=1, south=2, west=3
        assert_eq!(OmDirection::Invalid as i32, -1);
        assert_eq!(OmDirection::North as i32, 0);
        assert_eq!(OmDirection::East as i32, 1);
        assert_eq!(OmDirection::South as i32, 2);
        assert_eq!(OmDirection::West as i32, 3);
    }
}
