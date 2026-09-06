//! Calendar / game-time tests — GameTime resource, turn tracking, and
//! hours-elapsed calculation.

use cdda_sim::runtime::state::GameTime;
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Default state
// ---------------------------------------------------------------------------

#[test]
fn game_time_default() {
    let time = GameTime::default();
    assert_eq!(time.turn, 0);
}

// ---------------------------------------------------------------------------
// Turn advancement
// ---------------------------------------------------------------------------

#[test]
fn game_time_advance() {
    let mut test = TestBed::new();
    test.insert_resource(GameTime::default());

    test.resource_mut::<GameTime>().advance();

    let time = test.resource::<GameTime>();
    assert_eq!(time.turn, 1);
}

#[test]
fn game_time_multiple_advances() {
    let mut test = TestBed::new();
    test.insert_resource(GameTime::default());

    for _ in 0..3 {
        test.resource_mut::<GameTime>().advance();
    }

    let time = test.resource::<GameTime>();
    assert_eq!(time.turn, 3);
}

// ---------------------------------------------------------------------------
// Hours elapsed
// ---------------------------------------------------------------------------

#[test]
fn hours_elapsed_zero() {
    let time = GameTime { turn: 0 };
    assert_eq!(time.hours_elapsed(), 0);
}

#[test]
fn hours_elapsed_one_day() {
    let time = GameTime {
        turn: GameTime::TURNS_PER_DAY,
    };
    // 86400 one-second turns / 3600 = 24
    assert_eq!(time.hours_elapsed(), 24);
}

#[test]
fn hours_elapsed_partial_day() {
    let time = GameTime { turn: 36000 };
    // 36000 one-second turns / 3600 = 10
    assert_eq!(time.hours_elapsed(), 10);
}
