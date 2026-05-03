use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Game time measured in turns.
///
/// Each turn is approximately 1 second of in-game time.
/// The calendar system builds on this base unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Time(pub i64);

impl Time {
    pub const ZERO: Time = Time(0);

    pub const fn from_turns(turns: i64) -> Self {
        Time(turns)
    }

    pub fn as_turns(&self) -> i64 {
        self.0
    }
}

impl Add for Time {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Time(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Time(self.0 - rhs.0)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
