//! Integration tests for [`cdda_core::stats::Stats`].
//!
//! Exercises construction, default values, equality, and field access.

use cdda_core::Stats;

#[test]
fn stats_default() {
    let s = Stats::default();
    assert_eq!(s.strength, 8);
    assert_eq!(s.dexterity, 8);
    assert_eq!(s.intelligence, 8);
    assert_eq!(s.perception, 8);
}

#[test]
fn stats_custom_values() {
    let s = Stats::new(10, 12, 8, 14);
    assert_eq!(s.strength, 10);
    assert_eq!(s.dexterity, 12);
    assert_eq!(s.intelligence, 8);
    assert_eq!(s.perception, 14);
}

#[test]
fn stats_new_zero() {
    let s = Stats::new(0, 0, 0, 0);
    assert_eq!(s.strength, 0);
    assert_eq!(s.dexterity, 0);
    assert_eq!(s.intelligence, 0);
    assert_eq!(s.perception, 0);
}

#[test]
fn stats_eq_same() {
    let a = Stats::new(10, 10, 10, 10);
    let b = Stats::new(10, 10, 10, 10);
    assert_eq!(a, b);
}

#[test]
fn stats_eq_different() {
    let a = Stats::new(10, 10, 10, 10);
    let b = Stats::new(10, 10, 10, 11);
    assert_ne!(a, b);
}

#[test]
fn stats_field_access() {
    let s = Stats::new(7, 9, 11, 13);
    assert_eq!(s.strength, 7);
    assert_eq!(s.dexterity, 9);
    assert_eq!(s.intelligence, 11);
    assert_eq!(s.perception, 13);
}
