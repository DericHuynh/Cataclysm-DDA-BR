//! Integration tests for [`cdda_core::units`]: Volume, Weight, Length, Time, Energy.
//!
//! These tests exercise the public API of each unit type: construction,
//! arithmetic, ordering, and display formatting.

use cdda_core_types::core::units::{Energy, Length, Time, Volume, Weight};

// ─── Volume ─────────────────────────────────────────────────────────────────

#[test]
fn from_liters_conversion() {
    assert_eq!(Volume::from_liters(2).as_milliliters(), 2000);
}

#[test]
fn from_milliliters_storage() {
    assert_eq!(Volume::from_milliliters(500).as_milliliters(), 500);
}

#[test]
fn volume_addition() {
    assert_eq!(Volume(100) + Volume(200), Volume(300));
}

#[test]
fn volume_subtraction_saturating() {
    assert_eq!(Volume(100) - Volume(200), Volume::ZERO);
}

#[test]
fn volume_ordering() {
    assert!(Volume(100) < Volume(200));
}

#[test]
fn volume_display() {
    assert_eq!(format!("{}", Volume(250)), "250 ml");
    assert_eq!(format!("{}", Volume(1500)), "1.5 L");
}

// ─── Weight ─────────────────────────────────────────────────────────────────

#[test]
fn from_kilograms() {
    assert_eq!(Weight::from_kilograms(1.5).as_grams(), 1500);
}

#[test]
fn from_grams_storage() {
    assert_eq!(Weight::from_grams(500).as_grams(), 500);
}

#[test]
fn weight_addition() {
    assert_eq!(Weight(100) + Weight(200), Weight(300));
}

#[test]
fn weight_subtraction_saturating() {
    assert_eq!(Weight(100) - Weight(500), Weight::ZERO);
}

#[test]
fn weight_ordering() {
    assert!(Weight(1000) > Weight(500));
}

#[test]
fn weight_display() {
    assert_eq!(format!("{}", Weight(500)), "500 g");
    assert_eq!(format!("{}", Weight(2000)), "2 kg");
}

// ─── Length ─────────────────────────────────────────────────────────────────

#[test]
fn from_meters() {
    assert_eq!(Length::from_meters(2).as_millimeters(), 2000);
}

#[test]
fn from_centimeters() {
    assert_eq!(Length::from_centimeters(50).as_millimeters(), 500);
}

#[test]
fn length_addition() {
    assert_eq!(Length(1000) + Length(500), Length(1500));
}

#[test]
fn length_subtraction_saturating() {
    assert_eq!(Length(100) - Length(200), Length::ZERO);
}

#[test]
fn length_ordering() {
    assert!(Length::from_meters(1) > Length::from_centimeters(50));
}

#[test]
fn length_display() {
    assert_eq!(format!("{}", Length(5)), "5 mm");
    assert_eq!(format!("{}", Length(1500)), "1.5 m");
}

// ─── Time ───────────────────────────────────────────────────────────────────

#[test]
fn time_from_turns() {
    assert_eq!(Time::from_turns(3600).as_turns(), 3600);
}

#[test]
fn time_addition() {
    assert_eq!(Time(100) + Time(200), Time(300));
}

#[test]
fn time_subtraction() {
    assert_eq!(Time(500) - Time(200), Time(300));
}

#[test]
fn time_ordering() {
    assert!(Time::from_turns(3600) > Time::from_turns(600));
}

#[test]
fn time_display() {
    assert_eq!(format!("{}", Time(3661)), "1 h 1 m 1 s");
    assert_eq!(format!("{}", Time(0)), "0 m");
}

// ─── Energy ─────────────────────────────────────────────────────────────────

#[test]
fn energy_from_joules() {
    assert_eq!(Energy::from_joules(1000).as_joules(), 1000);
}

#[test]
fn energy_addition() {
    assert_eq!(Energy(500) + Energy(500), Energy(1000));
}

#[test]
fn energy_subtraction_saturating() {
    assert_eq!(Energy(100) - Energy(200), Energy::ZERO);
}

#[test]
fn energy_ordering() {
    assert!(Energy::from_joules(1000) > Energy::from_joules(500));
}
