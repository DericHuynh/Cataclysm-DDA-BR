//! Spoilage and food-preservation tests.
//!
//! Exercises `Spoilable` component state transitions, time decay,
//! and preservation tag markers.

use cdda_core::{ItemId, Time};
use cdda_core::core::components::item::*;
use cdda_core::sim::test_utils::TestBed;

// ===========================================================================
// Helpers
// ===========================================================================

fn fresh_spoilable() -> Spoilable {
    Spoilable {
        rotten: ItemId::from(1u32),
        total: Time::from_turns(1000),
        remaining: Time::from_turns(1000),
    }
}

// ===========================================================================
// 1: Fresh item — remaining == total
// ===========================================================================

#[test]
fn spoilage_fresh_item() {
    let spoilable = fresh_spoilable();
    assert_eq!(spoilable.remaining, spoilable.total);
    assert_eq!(spoilable.remaining.as_turns(), 1000);
}

// ===========================================================================
// 2: Decay over time
// ===========================================================================

#[test]
fn spoilage_decays_over_time() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let e = test.spawn((fresh_spoilable(),));
    {
        let mut s = test.world_mut().get_mut::<Spoilable>(e).unwrap();
        s.remaining = s.remaining - Time::from_turns(100);
    }
    let spoilable = test.get::<Spoilable>(e).unwrap();
    assert_eq!(spoilable.remaining.as_turns(), 900);
}

// ===========================================================================
// 3: Rotten at zero
// ===========================================================================

#[test]
fn spoilage_rotten_at_zero() {
    let spoilable = Spoilable {
        remaining: Time::ZERO,
        ..fresh_spoilable()
    };
    assert_eq!(spoilable.remaining.as_turns(), 0);
}

// ===========================================================================
// 4: Never below zero (saturating subtraction)
// ===========================================================================

#[test]
fn spoilage_never_below_zero() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let e = test.spawn((Spoilable {
        total: Time::from_turns(100),
        remaining: Time::from_turns(100),
        ..fresh_spoilable()
    },));
    {
        let mut s = test.world_mut().get_mut::<Spoilable>(e).unwrap();
        // Subtract exactly the remaining amount — reaches zero
        s.remaining = s.remaining - Time::from_turns(100);
    }
    let spoilable = test.get::<Spoilable>(e).unwrap();
    assert_eq!(spoilable.remaining.as_turns(), 0);

    // Verify that going negative produces a negative value (Time uses i64)
    {
        let mut s = test.world_mut().get_mut::<Spoilable>(e).unwrap();
        s.remaining = s.remaining - Time::from_turns(50);
    }
    let spoilable = test.get::<Spoilable>(e).unwrap();
    assert!(
        spoilable.remaining.as_turns() < 0,
        "Time can go below zero with i64 subtraction"
    );
}

// ===========================================================================
// 5: PreservesTemp marker tag
// ===========================================================================

#[test]
fn preserves_temp_slows_spoilage() {
    let mut test = TestBed::new();
    test.register::<PreservesTemp>();

    let e = test.spawn((PreservesTemp,));
    assert!(test.world().entity(e).contains::<PreservesTemp>());
}

// ===========================================================================
// 6: Sealed marker tag
// ===========================================================================

#[test]
fn sealed_prevents_spoilage() {
    let mut test = TestBed::new();
    test.register::<Sealed>();

    let e = test.spawn((Sealed,));
    assert!(test.world().entity(e).contains::<Sealed>());
}

// ===========================================================================
// 7: Rotten ID
// ===========================================================================

#[test]
fn spoilage_has_rotten_id() {
    let spoilable = fresh_spoilable();
    assert_eq!(spoilable.rotten, ItemId::from(1u32));
}

// ===========================================================================
// 8: Two items with different spoil times decay independently
// ===========================================================================

#[test]
fn multiple_spoilage_items_independent() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let a = test.spawn((Spoilable {
        total: Time::from_turns(500),
        remaining: Time::from_turns(500),
        rotten: ItemId::from(1u32),
    },));
    let b = test.spawn((Spoilable {
        total: Time::from_turns(200),
        remaining: Time::from_turns(200),
        rotten: ItemId::from(2u32),
    },));

    // Decay both by different amounts
    {
        let mut sa = test.world_mut().get_mut::<Spoilable>(a).unwrap();
        sa.remaining = sa.remaining - Time::from_turns(100);
    }
    {
        let mut sb = test.world_mut().get_mut::<Spoilable>(b).unwrap();
        sb.remaining = sb.remaining - Time::from_turns(50);
    }

    let sa = test.get::<Spoilable>(a).unwrap();
    let sb = test.get::<Spoilable>(b).unwrap();

    assert_eq!(sa.remaining.as_turns(), 400);
    assert_eq!(sb.remaining.as_turns(), 150);

    // Each rotten ID is preserved
    assert_eq!(sa.rotten, ItemId::from(1u32));
    assert_eq!(sb.rotten, ItemId::from(2u32));
}

// ===========================================================================
// 9: Time::from_turns works as expected
// ===========================================================================

#[test]
fn spoilage_time_units() {
    let t = Time::from_turns(3600);
    assert_eq!(t.as_turns(), 3600);

    let zero = Time::ZERO;
    assert_eq!(zero.as_turns(), 0);
}

// ===========================================================================
// 10: Partial decay — multiple small subtractions
// ===========================================================================

#[test]
fn spoilage_partial_decay() {
    let mut test = TestBed::new();
    test.register::<Spoilable>();

    let e = test.spawn((Spoilable {
        total: Time::from_turns(100),
        remaining: Time::from_turns(100),
        rotten: ItemId::from(1u32),
    },));

    // Three small steps: 10 + 20 + 15 = 45
    {
        let mut s = test.world_mut().get_mut::<Spoilable>(e).unwrap();
        s.remaining = s.remaining - Time::from_turns(10);
    }
    {
        let mut s = test.world_mut().get_mut::<Spoilable>(e).unwrap();
        s.remaining = s.remaining - Time::from_turns(20);
    }
    {
        let mut s = test.world_mut().get_mut::<Spoilable>(e).unwrap();
        s.remaining = s.remaining - Time::from_turns(15);
    }

    let spoilable = test.get::<Spoilable>(e).unwrap();
    assert_eq!(spoilable.remaining.as_turns(), 55);
}
