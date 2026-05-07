use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// A z-coordinate with checked arithmetic.
///
/// Range `[-10, 10]` is enforced on construction.
/// CDDA confirms: "z-coordinates do not scale along with the horizontal dimensions."
///
/// Z is always absolute — it does not participate in horizontal scale conversions.
/// When converting between coordinate scales, z passes through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ZLevel(pub i8);

impl ZLevel {
    pub const MIN: i8 = -10;
    pub const MAX: i8 = 10;

    /// Create a new `ZLevel`, clamping to the valid range `[-10, 10]`.
    pub const fn new(z: i8) -> Self {
        if z < Self::MIN {
            ZLevel(Self::MIN)
        } else if z > Self::MAX {
            ZLevel(Self::MAX)
        } else {
            ZLevel(z)
        }
    }

    /// Checked addition — returns `None` if the result would exceed the range.
    pub fn checked_add(self, rhs: i8) -> Option<Self> {
        let result = self.0.checked_add(rhs)?;
        if result < Self::MIN || result > Self::MAX {
            None
        } else {
            Some(ZLevel(result))
        }
    }

    /// Checked subtraction — returns `None` if the result would exceed the range.
    pub fn checked_sub(self, rhs: i8) -> Option<Self> {
        let result = self.0.checked_sub(rhs)?;
        if result < Self::MIN || result > Self::MAX {
            None
        } else {
            Some(ZLevel(result))
        }
    }
}

impl Add for ZLevel {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        ZLevel::new(self.0 + rhs.0)
    }
}

impl Sub for ZLevel {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        ZLevel::new(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zlevel_construction() {
        assert_eq!(ZLevel::new(0).0, 0);
        assert_eq!(ZLevel::new(5).0, 5);
        assert_eq!(ZLevel::new(-3).0, -3);
    }

    #[test]
    fn test_zlevel_clamping() {
        assert_eq!(ZLevel::new(-20).0, -10);
        assert_eq!(ZLevel::new(20).0, 10);
    }

    #[test]
    fn test_checked_add_within_bounds() {
        let z = ZLevel::new(5);
        assert_eq!(z.checked_add(3), Some(ZLevel::new(8)));
    }

    #[test]
    fn test_checked_add_overflow() {
        let z = ZLevel::new(9);
        assert_eq!(z.checked_add(2), None);
    }

    #[test]
    fn test_checked_sub_underflow() {
        let z = ZLevel::new(-9);
        assert_eq!(z.checked_sub(2), None);
    }
}
