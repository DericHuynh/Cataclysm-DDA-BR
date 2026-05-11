use serde::{Deserialize, Serialize};

/// Cardinal and intercardinal directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl Direction {
    /// Returns the (dx, dy) offset for this direction.
    pub fn offset(self) -> (i32, i32) {
        match self {
            Direction::N => (0, -1),
            Direction::NE => (1, -1),
            Direction::E => (1, 0),
            Direction::SE => (1, 1),
            Direction::S => (0, 1),
            Direction::SW => (-1, 1),
            Direction::W => (-1, 0),
            Direction::NW => (-1, -1),
        }
    }
}

/// Vehicle / creature facing, expressed as an angle in degrees clockwise from north.
///
/// `Facing(0)` = north, `Facing(90)` = east, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facing(pub i32);

impl Facing {
    /// Create a new `Facing`, normalized to `[0, 360)`.
    pub fn new(degrees: i32) -> Self {
        let normalized = degrees.rem_euclid(360);
        Facing(normalized)
    }

    /// Returns the cardinal `Direction` closest to this facing.
    pub fn to_cardinal(self) -> Direction {
        // 0° = N, 90° = E, 180° = S, 270° = W
        match (self.0 + 23).rem_euclid(360) / 45 {
            0 => Direction::N,
            1 => Direction::NE,
            2 => Direction::E,
            3 => Direction::SE,
            4 => Direction::S,
            5 => Direction::SW,
            6 => Direction::W,
            7 => Direction::NW,
            _ => unreachable!(),
        }
    }

    /// Rotate the facing by `delta` degrees.
    pub fn rotate(self, delta: i32) -> Self {
        Facing::new(self.0 + delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facing_normalize() {
        assert_eq!(Facing::new(360), Facing::new(0));
        assert_eq!(Facing::new(-90), Facing::new(270));
    }

    #[test]
    fn test_facing_to_cardinal() {
        assert_eq!(Facing::new(0).to_cardinal(), Direction::N);
        assert_eq!(Facing::new(90).to_cardinal(), Direction::E);
        assert_eq!(Facing::new(180).to_cardinal(), Direction::S);
        assert_eq!(Facing::new(270).to_cardinal(), Direction::W);
        assert_eq!(Facing::new(45).to_cardinal(), Direction::NE);
    }
}
