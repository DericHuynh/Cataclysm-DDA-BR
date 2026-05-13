//! Seeded deterministic RNG matching CDDA master's `rng()` behavior.
//!
//! Uses an LCG: `state = state * 1103515245 + 12345` with the top bits taken
//! as output. This is a Bevy [`Resource`] so systems can access it via
//! `Res<XorShiftRng>` or `ResMut<XorShiftRng>`.
//!
//! ## Methods
//!
//! | Method | CDDA equivalent | Returns |
//! |---|---|---|
//! | `range_i32(lo, hi)` | `rng(lo, hi)` | integer in `[lo, hi]` inclusive |
//! | `range_f32(lo, hi)` | `rng_float(lo, hi)` | float in `[lo, hi)` |
//! | `one_in(n)` | `one_in(n)` | `true` with probability `1/n` |
//! | `roll_remainder(expected)` | `roll_remainder(expected)` | integer with fractional roll |
//! | `x_in_y(x, y)` | `x_in_y(x, y)` | `true` with probability `x/y` |

use bevy_ecs::prelude::*;

use crate::direction::Rng;

/// Deterministic seeded PRNG using the LCG from CDDA master.
///
/// # Mutation
/// Mutate freely via `&mut self` — this is a continuously-updating value,
/// not discrete state. Change detection on the [`Resource`] fires every
/// frame the RNG is advanced, which is expected.
#[derive(Resource, Debug, Clone)]
pub struct XorShiftRng {
    state: u64,
}

impl XorShiftRng {
    /// Create a new RNG from a seed. The seed is incremented by 1 internally
    /// to avoid the absorbing state of 0 in LCGs.
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    /// Raw next u32. Advances the LCG and returns the top 32 bits.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12_345);
        (self.state >> 16) as u32
    }

    /// Random integer in `[lo, hi]` inclusive. Matches CDDA `rng(lo, hi)`.
    #[inline]
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }
        let range = (hi - lo + 1) as u32;
        lo + (self.next_u32() % range) as i32
    }

    /// Random float in `[lo, hi)`. Matches CDDA `rng_float(lo, hi)`.
    #[inline]
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let t = self.next_f64();
        // fma: lo + t * (hi - lo)
        (lo as f64 + t * (hi as f64 - lo as f64)) as f32
    }

    /// Random f64 in `[0.0, 1.0)`.
    #[inline]
    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / ((u32::MAX as f64) + 1.0)
    }

    /// Returns `true` with probability `1/n`. Matches CDDA `one_in(n)`.
    ///
    /// Returns `false` if `n <= 0`.
    #[inline]
    pub fn one_in(&mut self, n: i32) -> bool {
        if n <= 0 {
            return false;
        }
        self.range_i32(0, n - 1) == 0
    }

    /// Rolls the fractional part of `expected`. Returns the integer count.
    ///
    /// For example, `roll_remainder(2.7)` returns `2` with 30% probability
    /// and `3` with 70% probability. Matches CDDA `roll_remainder(expected)`.
    #[inline]
    pub fn roll_remainder(&mut self, expected: f64) -> i32 {
        let mut ret = expected as i32;
        let frac = expected - ret as f64;
        if frac > 0.0 && self.next_f64() < frac {
            ret += 1;
        }
        ret
    }

    /// Returns `true` with probability `x/y`. Matches CDDA `x_in_y(x, y)`.
    ///
    /// Returns `false` if `y <= 0` or `x <= 0`. Returns `true` if `x >= y`.
    #[inline]
    pub fn x_in_y(&mut self, x: i32, y: i32) -> bool {
        if y <= 0 || x <= 0 {
            return false;
        }
        if x >= y {
            return true;
        }
        self.range_i32(0, y - 1) < x
    }
}

impl Rng for XorShiftRng {
    fn random_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u32() as usize) % max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_seed() {
        let mut a = XorShiftRng::new(42);
        let mut b = XorShiftRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn deterministic_different_seed() {
        let mut a = XorShiftRng::new(42);
        let mut b = XorShiftRng::new(99);
        let mut any_different = false;
        for _ in 0..100 {
            if a.next_u32() != b.next_u32() {
                any_different = true;
                break;
            }
        }
        assert!(any_different, "different seeds should produce different sequences");
    }

    #[test]
    fn range_i32_bounds() {
        let mut rng = XorShiftRng::new(1);
        for _ in 0..1000 {
            let v = rng.range_i32(0, 9);
            assert!(v >= 0 && v <= 9, "v={v} out of [0,9]");
        }
    }

    #[test]
    fn range_i32_single_value() {
        let mut rng = XorShiftRng::new(1);
        assert_eq!(rng.range_i32(5, 5), 5);
    }

    #[test]
    fn one_in_deterministic() {
        // With seed 7, one_in(1000) should consistently produce the same result
        let mut a = XorShiftRng::new(7);
        let mut b = XorShiftRng::new(7);
        let results_a: Vec<bool> = (0..100).map(|_| a.one_in(1000)).collect();
        let results_b: Vec<bool> = (0..100).map(|_| b.one_in(1000)).collect();
        assert_eq!(results_a, results_b);
    }

    #[test]
    fn one_in_zero() {
        let mut rng = XorShiftRng::new(1);
        assert!(!rng.one_in(0));
    }

    #[test]
    fn one_in_one() {
        let mut rng = XorShiftRng::new(1);
        assert!(rng.one_in(1));
    }

    #[test]
    fn x_in_y_always() {
        let mut rng = XorShiftRng::new(1);
        assert!(rng.x_in_y(5, 5));
        assert!(rng.x_in_y(10, 5));
    }

    #[test]
    fn x_in_y_never() {
        let mut rng = XorShiftRng::new(1);
        assert!(!rng.x_in_y(0, 5));
        assert!(!rng.x_in_y(1, 0));
    }

    #[test]
    fn roll_remainder_integer() {
        let mut rng = XorShiftRng::new(1);
        assert_eq!(rng.roll_remainder(3.0), 3);
    }

    #[test]
    fn roll_remainder_fractional() {
        let mut rng = XorShiftRng::new(1);
        // 2.999 should almost always return 3
        let results: Vec<i32> = (0..1000).map(|_| rng.roll_remainder(2.999)).collect();
        let threes = results.iter().filter(|&&r| r == 3).count();
        // With 0.999 fractional, almost all should be 3
        assert!(threes > 900, "expected mostly 3s, got {threes}/1000");
    }

    #[test]
    fn range_f32_bounds() {
        let mut rng = XorShiftRng::new(1);
        for _ in 0..1000 {
            let v = rng.range_f32(1.0, 5.0);
            assert!(v >= 1.0 && v < 5.0, "v={v} out of [1.0, 5.0)");
        }
    }

    #[test]
    fn zero_seed() {
        // Seed 0 is internally bumped to 1; seed 1 is bumped to 2.
        // These should produce different sequences.
        let mut a = XorShiftRng::new(0);
        let mut b = XorShiftRng::new(1);
        assert_ne!(a.next_u32(), b.next_u32());
    }
}
