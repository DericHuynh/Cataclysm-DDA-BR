use cdda_core::data::mod_info::ModInfo;

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

#[test]
fn new_mod_has_no_deps_or_conflicts() {
    let m = ModInfo::new("dda", "Dark Days Ahead");
    assert_eq!(m.id, "dda");
    assert_eq!(m.name, "Dark Days Ahead");
    assert!(m.dependencies.is_empty());
    assert!(m.conflicts.is_empty());
}

#[test]
fn builder_with_dependency() {
    let m = ModInfo::new("dda_guns", "DDA Guns").with_dependency("dda");
    assert_eq!(m.dependencies, vec!["dda"]);
}

#[test]
fn builder_with_conflict() {
    let m = ModInfo::new("mod_a", "Mod A").with_conflict("mod_b");
    assert_eq!(m.conflicts, vec!["mod_b"]);
}

#[test]
fn builder_chains_multiple_deps() {
    let m = ModInfo::new("big_mod", "Big Mod")
        .with_dependency("base")
        .with_dependency("extras");
    assert_eq!(m.dependencies.len(), 2);
    assert!(m.dependencies.contains(&"base".to_string()));
    assert!(m.dependencies.contains(&"extras".to_string()));
}

#[test]
fn builder_chains_multiple_conflicts() {
    let m = ModInfo::new("a", "A")
        .with_conflict("b")
        .with_conflict("c");
    assert_eq!(m.conflicts.len(), 2);
}

// ---------------------------------------------------------------------------
// Serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn json_roundtrip_basic() {
    let original = ModInfo::new("test_mod", "Test Mod")
        .with_dependency("base")
        .with_conflict("other");

    let json = serde_json::to_string(&original).expect("serialize");
    let restored: ModInfo = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(restored.id, "test_mod");
    assert_eq!(restored.name, "Test Mod");
    assert_eq!(restored.dependencies, vec!["base"]);
    assert_eq!(restored.conflicts, vec!["other"]);
}

#[test]
fn json_roundtrip_empty_deps_and_conflicts() {
    let original = ModInfo::new("bare", "Bare Mod");
    let json = serde_json::to_string(&original).unwrap();
    let restored: ModInfo = serde_json::from_str(&json).unwrap();
    assert!(restored.dependencies.is_empty());
    assert!(restored.conflicts.is_empty());
}

#[test]
fn equality_holds_after_clone() {
    let a = ModInfo::new("x", "X").with_dependency("y");
    let b = a.clone();
    assert_eq!(a, b);
}
