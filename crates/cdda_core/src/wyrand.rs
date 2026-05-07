//! Per-entity WyRand RNG for deterministic simulation.
//!
//! Based on Wang Yi's wyhash v4.2 — public domain.

use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WyRand {
    state: u64,
}

impl WyRand {
    pub fn from_seed(world_seed: u64, sim_id: u64) -> Self {
        let mut rng = WyRand { state: 0 };
        rng.state = splitmix64(world_seed.wrapping_add(sim_id));
        rng
    }

    pub fn gen(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0xa0761d6478bd642f);
        let t = u128::from(self.state).wrapping_mul(u128::from(self.state ^ 0xe7037ed1a0b428db));
        (t.wrapping_shr(64) ^ t) as u64
    }

    /// Returns a float in [0.0, 1.0).
    pub fn gen_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / (1u64 << 53) as f64;
        (self.gen() >> 11) as f64 * SCALE
    }

    pub fn gen_range_u32(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let range = (max - min + 1) as u64;
        let limit = u64::MAX - u64::MAX % range;
        loop {
            let x = self.gen();
            if x < limit {
                return min + (x % range) as u32;
            }
        }
    }

    pub fn gen_bool(&mut self, p: f64) -> bool {
        self.gen_f64() < p
    }

    pub fn fork(&mut self) -> Self {
        WyRand { state: self.gen() }
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = WyRand::from_seed(42, 1);
        let mut b = WyRand::from_seed(42, 1);
        for _ in 0..100 {
            assert_eq!(a.gen(), b.gen());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = WyRand::from_seed(42, 1);
        let mut b = WyRand::from_seed(42, 2);
        let mut same = true;
        for _ in 0..20 {
            if a.gen() != b.gen() {
                same = false;
                break;
            }
        }
        assert!(!same);
    }

    #[test]
    fn fork_produces_different_state() {
        let mut parent = WyRand::from_seed(99, 0);
        let a = parent.fork();
        let b = parent.fork();
        assert_ne!(a.state, b.state);
    }

    #[test]
    fn gen_f64_in_range() {
        let mut rng = WyRand::from_seed(1, 0);
        for _ in 0..5000 {
            let v = rng.gen_f64();
            assert!(v >= 0.0 && v < 1.0, "got {v}");
        }
    }

    #[test]
    fn gen_range_exact_match() {
        let mut rng = WyRand::from_seed(1, 0);
        for _ in 0..100 {
            assert_eq!(rng.gen_range_u32(5, 5), 5);
        }
    }

    #[test]
    fn gen_range_in_bounds() {
        let mut rng = WyRand::from_seed(1, 0);
        for _ in 0..1000 {
            let v = rng.gen_range_u32(10, 20);
            assert!(v >= 10 && v <= 20, "got {v}");
        }
    }
}
