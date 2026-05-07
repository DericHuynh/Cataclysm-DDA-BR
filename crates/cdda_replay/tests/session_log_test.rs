use cdda_input::{ActionSource, GameAction};
use cdda_replay::session_log::{ActionRecord, SessionLog};

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

#[test]
fn new_log_stores_seed() {
    let log = SessionLog::new(42);
    assert_eq!(log.world_seed, 42);
}

#[test]
fn new_log_is_empty() {
    let log = SessionLog::new(0);
    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
}

#[test]
fn default_log_has_seed_zero() {
    let log = SessionLog::default();
    assert_eq!(log.world_seed, 0);
}

// ---------------------------------------------------------------------------
// Mutation
// ---------------------------------------------------------------------------

#[test]
fn push_action_increments_len() {
    let mut log = SessionLog::new(1);
    log.actions.push(ActionRecord {
        turn: 0,
        action: GameAction::Wait,
        source: ActionSource::Keyboard,
    });
    assert_eq!(log.len(), 1);
    assert!(!log.is_empty());
}

#[test]
fn multiple_actions_stored_in_order() {
    let mut log = SessionLog::new(0);
    let actions = [GameAction::Wait, GameAction::Pickup, GameAction::Confirm];
    for (turn, action) in actions.iter().enumerate() {
        log.actions.push(ActionRecord {
            turn: turn as u64,
            action: action.clone(),
            source: ActionSource::Keyboard,
        });
    }
    assert_eq!(log.len(), 3);
    assert!(matches!(log.actions[0].action, GameAction::Wait));
    assert!(matches!(log.actions[1].action, GameAction::Pickup));
    assert!(matches!(log.actions[2].action, GameAction::Confirm));
}

#[test]
fn action_record_stores_turn_and_source() {
    let record = ActionRecord {
        turn: 99,
        action: GameAction::Open,
        source: ActionSource::Script,
    };
    assert_eq!(record.turn, 99);
    assert!(matches!(record.source, ActionSource::Script));
}

// ---------------------------------------------------------------------------
// Serialization round-trip (bytes)
// ---------------------------------------------------------------------------

#[test]
fn bytes_roundtrip_empty_log() {
    let original = SessionLog::new(77);
    let bytes = original.to_bytes().expect("serialize");
    let restored = SessionLog::from_bytes(&bytes).expect("deserialize");
    assert_eq!(restored.world_seed, 77);
    assert!(restored.is_empty());
}

#[test]
fn bytes_roundtrip_with_actions() {
    let mut original = SessionLog::new(123);
    original.actions.push(ActionRecord {
        turn: 5,
        action: GameAction::NavigateDown,
        source: ActionSource::Keyboard,
    });
    original.actions.push(ActionRecord {
        turn: 10,
        action: GameAction::Confirm,
        source: ActionSource::Keyboard,
    });

    let bytes = original.to_bytes().expect("serialize");
    let restored = SessionLog::from_bytes(&bytes).expect("deserialize");

    assert_eq!(restored.world_seed, 123);
    assert_eq!(restored.len(), 2);
    assert_eq!(restored.actions[0].turn, 5);
    assert_eq!(restored.actions[1].turn, 10);
    assert!(matches!(restored.actions[1].action, GameAction::Confirm));
}

#[test]
fn corrupt_bytes_returns_error() {
    let result = SessionLog::from_bytes(&[0xde, 0xad, 0xbe, 0xef]);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// File I/O round-trip
// ---------------------------------------------------------------------------

#[test]
fn file_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("replay.bin");

    let mut original = SessionLog::new(999);
    original.actions.push(ActionRecord {
        turn: 1,
        action: GameAction::Wait,
        source: ActionSource::Keyboard,
    });

    original.save_to_file(&path).expect("save");
    let restored = SessionLog::load_from_file(&path).expect("load");

    assert_eq!(restored.world_seed, 999);
    assert_eq!(restored.len(), 1);
    assert!(matches!(restored.actions[0].action, GameAction::Wait));
}

#[test]
fn load_from_missing_file_returns_error() {
    let result = SessionLog::load_from_file(std::path::Path::new("/nonexistent/replay.bin"));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Compressed round-trip
// ---------------------------------------------------------------------------

#[test]
fn compressed_roundtrip_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("replay.bin.zst");

    let original = SessionLog::new(55);
    original.save_compressed(&path).expect("save compressed");
    let restored = SessionLog::load_compressed(&path).expect("load compressed");

    assert_eq!(restored.world_seed, 55);
    assert!(restored.is_empty());
}

#[test]
fn compressed_roundtrip_with_actions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("replay.bin.zst");

    let mut original = SessionLog::new(7);
    for turn in 0..50 {
        original.actions.push(ActionRecord {
            turn,
            action: GameAction::Wait,
            source: ActionSource::Keyboard,
        });
    }

    original.save_compressed(&path).expect("save compressed");
    let restored = SessionLog::load_compressed(&path).expect("load compressed");

    assert_eq!(restored.world_seed, 7);
    assert_eq!(restored.len(), 50);
}

#[test]
fn compressed_file_is_smaller_than_uncompressed_for_large_log() {
    let dir = tempfile::tempdir().expect("tempdir");
    let raw_path = dir.path().join("replay.bin");
    let zst_path = dir.path().join("replay.bin.zst");

    let mut log = SessionLog::new(0);
    for turn in 0..1000 {
        log.actions.push(ActionRecord {
            turn,
            action: GameAction::Wait,
            source: ActionSource::Keyboard,
        });
    }

    log.save_to_file(&raw_path).expect("save raw");
    log.save_compressed(&zst_path).expect("save compressed");

    let raw_size = std::fs::metadata(&raw_path).unwrap().len();
    let zst_size = std::fs::metadata(&zst_path).unwrap().len();
    assert!(zst_size < raw_size, "compressed ({zst_size}B) should be smaller than raw ({raw_size}B)");
}

// ---------------------------------------------------------------------------
// State hashes (no devtools feature — hash list stays empty)
// ---------------------------------------------------------------------------

#[test]
fn state_hashes_empty_by_default() {
    let log = SessionLog::new(0);
    assert!(log.state_hashes.is_empty());
}

#[test]
fn state_hashes_survive_roundtrip() {
    let mut log = SessionLog::new(0);
    log.state_hashes.push((1, 0xdeadbeef));
    let bytes = log.to_bytes().expect("serialize");
    let restored = SessionLog::from_bytes(&bytes).expect("deserialize");
    assert_eq!(restored.state_hashes.len(), 1);
    assert_eq!(restored.state_hashes[0], (1, 0xdeadbeef));
}
