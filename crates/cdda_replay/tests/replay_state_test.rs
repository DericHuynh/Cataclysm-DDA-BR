use cdda_input::vocabulary::{ActionSource, GameAction};
use cdda_replay::replay::{ReplaySpeed, ReplayState};
use cdda_replay::session_log::{ActionRecord, SessionLog};

// ---------------------------------------------------------------------------
// Default state
// ---------------------------------------------------------------------------

#[test]
fn default_cursor_is_zero() {
    let state = ReplayState::default();
    assert_eq!(state.cursor, 0);
}

#[test]
fn default_speed_is_fast() {
    let state = ReplayState::default();
    assert!(matches!(state.speed, ReplaySpeed::Fast));
}

#[test]
fn default_not_paused() {
    let state = ReplayState::default();
    assert!(!state.paused);
}

// ---------------------------------------------------------------------------
// is_complete
// ---------------------------------------------------------------------------

#[test]
fn is_complete_on_empty_log() {
    let state = ReplayState::default();
    let log = SessionLog::new(0);
    assert!(state.is_complete(&log));
}

#[test]
fn not_complete_when_actions_remain() {
    let state = ReplayState::default();
    let mut log = SessionLog::new(0);
    log.actions.push(ActionRecord {
        turn: 0,
        action: GameAction::Wait,
        source: ActionSource::Keyboard,
    });
    assert!(!state.is_complete(&log));
}

#[test]
fn is_complete_when_cursor_at_end() {
    let mut state = ReplayState::default();
    let mut log = SessionLog::new(0);
    log.actions.push(ActionRecord {
        turn: 0,
        action: GameAction::Wait,
        source: ActionSource::Keyboard,
    });
    state.cursor = 1;
    assert!(state.is_complete(&log));
}

#[test]
fn is_complete_when_cursor_beyond_end() {
    let mut state = ReplayState::default();
    state.cursor = 999;
    let log = SessionLog::new(0);
    assert!(state.is_complete(&log));
}

#[test]
fn not_complete_with_cursor_partway() {
    let mut state = ReplayState::default();
    state.cursor = 3;
    let mut log = SessionLog::new(0);
    for turn in 0..10 {
        log.actions.push(ActionRecord {
            turn,
            action: GameAction::Wait,
            source: ActionSource::Keyboard,
        });
    }
    assert!(!state.is_complete(&log));
}

// ---------------------------------------------------------------------------
// ReplaySpeed equality
// ---------------------------------------------------------------------------

#[test]
fn replay_speed_variants_are_distinct() {
    assert_ne!(ReplaySpeed::Fast as u8, ReplaySpeed::RealTime as u8);
    assert_ne!(ReplaySpeed::Fast as u8, ReplaySpeed::Step as u8);
    assert_ne!(ReplaySpeed::RealTime as u8, ReplaySpeed::Step as u8);
}

// ---------------------------------------------------------------------------
// Cursor semantics
// ---------------------------------------------------------------------------

#[test]
fn cursor_advances_independently_of_log() {
    let mut state = ReplayState::default();
    let log = SessionLog::new(0);
    // cursor can be incremented even if log is empty (no panic)
    state.cursor += 1;
    assert!(state.is_complete(&log));
}
