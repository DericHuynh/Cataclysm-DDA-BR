//! Integration tests for [`cdda_core::rng::SeededRng`].
//!
//! Verifies determinism, seed storage, and range/boolean/float generation
//! contracts.

use cdda_core_types::rng::SeededRng;

#[test]
fn rng_deterministic_same_seed() {
    let mut a = SeededRng::new(42);
    let mut b = SeededRng::new(42);

    let seq_a: Vec<u32> = (0..20).map(|_| a.gen_range(0, 100)).collect();
    let seq_b: Vec<u32> = (0..20).map(|_| b.gen_range(0, 100)).collect();

    assert_eq!(seq_a, seq_b);
}

#[test]
fn rng_different_seed_different() {
    let mut a = SeededRng::new(42);
    let mut b = SeededRng::new(99);

    // Almost certainly different sequences
    let seq_a: u32 = a.gen_range(0, 100);
    let seq_b: u32 = b.gen_range(0, 100);
    assert_ne!(
        seq_a, seq_b,
        "extremely unlikely that two different seeds produce the same first value"
    );
}

#[test]
fn rng_seed_stored() {
    let rng = SeededRng::new(12345);
    assert_eq!(rng.seed(), 12345);
}

#[test]
fn rng_gen_range_bounds() {
    let mut rng = SeededRng::new(7);
    for _ in 0..1000 {
        let v = rng.gen_range(1, 6);
        assert!(
            (1..=6).contains(&v),
            "gen_range(1, 6) produced {v} which is outside [1, 6]"
        );
    }
}

#[test]
fn rng_gen_bool_p1() {
    let mut rng = SeededRng::new(7);
    for _ in 0..100 {
        assert!(rng.gen_bool(1.0));
    }
}

#[test]
fn rng_gen_bool_p0() {
    let mut rng = SeededRng::new(7);
    for _ in 0..100 {
        assert!(!rng.gen_bool(0.0));
    }
}

#[test]
fn rng_gen_f64_range() {
    let mut rng = SeededRng::new(7);
    for _ in 0..1000 {
        let v = rng.gen_f64();
        assert!(
            (0.0..1.0).contains(&v),
            "gen_f64() produced {v} which is outside [0.0, 1.0)"
        );
    }
}
