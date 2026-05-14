use cdda_data::mod_info::{check_conflicts, ModError, ModInfo};

// ---------------------------------------------------------------------------
// No conflicts
// ---------------------------------------------------------------------------

#[test]
fn no_conflicts_when_empty() {
    assert!(check_conflicts(&[]).is_ok());
}

#[test]
fn no_conflicts_with_compatible_mods() {
    let mods = [
        ModInfo::new("dda", "DDA"),
        ModInfo::new("guns", "Guns Everywhere"),
    ];
    assert!(check_conflicts(&mods).is_ok());
}

#[test]
fn declaring_conflict_with_absent_mod_is_ok() {
    // "a" conflicts with "ghost" but "ghost" is not active
    let mods = [ModInfo::new("a", "A").with_conflict("ghost")];
    assert!(check_conflicts(&mods).is_ok());
}

// ---------------------------------------------------------------------------
// Conflict detected
// ---------------------------------------------------------------------------

#[test]
fn mutual_conflict_detected() {
    let mods = [
        ModInfo::new("a", "A").with_conflict("b"),
        ModInfo::new("b", "B"),
    ];
    let err = check_conflicts(&mods).unwrap_err();
    assert!(matches!(err, ModError::Conflict { .. }));
}

#[test]
fn conflict_error_names_both_mods() {
    let mods = [
        ModInfo::new("medieval", "Medieval World").with_conflict("modern"),
        ModInfo::new("modern", "Modern Setting"),
    ];
    let err = check_conflicts(&mods).unwrap_err();
    if let ModError::Conflict { a, b } = err {
        assert_eq!(a, "medieval");
        assert_eq!(b, "modern");
    } else {
        panic!("expected Conflict variant");
    }
}

#[test]
fn single_mod_conflicts_with_itself_detected() {
    let mods = [ModInfo::new("a", "A").with_conflict("a")];
    let err = check_conflicts(&mods).unwrap_err();
    assert!(matches!(err, ModError::Conflict { .. }));
}

#[test]
fn three_mods_one_pair_conflicts() {
    let mods = [
        ModInfo::new("a", "A"),
        ModInfo::new("b", "B").with_conflict("c"),
        ModInfo::new("c", "C"),
    ];
    assert!(check_conflicts(&mods).is_err());
}

#[test]
fn three_mods_no_conflicts() {
    let mods = [
        ModInfo::new("a", "A").with_conflict("x"),
        ModInfo::new("b", "B").with_conflict("y"),
        ModInfo::new("c", "C").with_conflict("z"),
    ];
    assert!(check_conflicts(&mods).is_ok());
}
