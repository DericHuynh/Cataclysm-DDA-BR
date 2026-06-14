use cdda_data::mod_info::{check_dependencies, resolve_load_order, ModError, ModInfo};

// ---------------------------------------------------------------------------
// check_dependencies
// ---------------------------------------------------------------------------

#[test]
fn no_deps_always_valid() {
    let mods = [ModInfo::new("dda", "DDA")];
    assert!(check_dependencies(&mods).is_ok());
}

#[test]
fn satisfied_dep_is_valid() {
    let mods = [
        ModInfo::new("base", "Base"),
        ModInfo::new("extra", "Extra").with_dependency("base"),
    ];
    assert!(check_dependencies(&mods).is_ok());
}

#[test]
fn missing_dep_returns_error() {
    let mods = [ModInfo::new("extra", "Extra").with_dependency("missing")];
    let err = check_dependencies(&mods).unwrap_err();
    assert!(matches!(err, ModError::UnknownDependency { .. }));
    if let ModError::UnknownDependency { dep, requirer } = err {
        assert_eq!(dep, "missing");
        assert_eq!(requirer, "extra");
    }
}

#[test]
fn multiple_mods_one_missing_dep_returns_error() {
    let mods = [
        ModInfo::new("a", "A"),
        ModInfo::new("b", "B").with_dependency("a"),
        ModInfo::new("c", "C").with_dependency("ghost"),
    ];
    assert!(check_dependencies(&mods).is_err());
}

#[test]
fn empty_mod_list_is_valid() {
    assert!(check_dependencies(&[]).is_ok());
}

// ---------------------------------------------------------------------------
// resolve_load_order — basic ordering
// ---------------------------------------------------------------------------

#[test]
fn single_mod_no_deps() {
    let mods = [ModInfo::new("dda", "DDA")];
    let order = resolve_load_order(&mods).expect("ok");
    assert_eq!(order, vec!["dda"]);
}

#[test]
fn dep_comes_before_dependent() {
    let mods = [
        ModInfo::new("extra", "Extra").with_dependency("base"),
        ModInfo::new("base", "Base"),
    ];
    let order = resolve_load_order(&mods).expect("ok");
    let base_pos = order.iter().position(|s| s == "base").unwrap();
    let extra_pos = order.iter().position(|s| s == "extra").unwrap();
    assert!(base_pos < extra_pos, "base must load before extra");
}

#[test]
fn chain_a_b_c_ordered() {
    let mods = [
        ModInfo::new("c", "C").with_dependency("b"),
        ModInfo::new("a", "A"),
        ModInfo::new("b", "B").with_dependency("a"),
    ];
    let order = resolve_load_order(&mods).expect("ok");
    let pos_a = order.iter().position(|s| s == "a").unwrap();
    let pos_b = order.iter().position(|s| s == "b").unwrap();
    let pos_c = order.iter().position(|s| s == "c").unwrap();
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
}

#[test]
fn diamond_dependency_resolves() {
    // A <- B, A <- C, B <- D, C <- D
    let mods = [
        ModInfo::new("a", "A"),
        ModInfo::new("b", "B").with_dependency("a"),
        ModInfo::new("c", "C").with_dependency("a"),
        ModInfo::new("d", "D").with_dependency("b").with_dependency("c"),
    ];
    let order = resolve_load_order(&mods).expect("ok");
    assert_eq!(order.len(), 4);
    let pos = |id: &str| order.iter().position(|s| s == id).unwrap();
    assert!(pos("a") < pos("b"));
    assert!(pos("a") < pos("c"));
    assert!(pos("b") < pos("d"));
    assert!(pos("c") < pos("d"));
}

#[test]
fn empty_list_returns_empty() {
    let order = resolve_load_order(&[]).expect("ok");
    assert!(order.is_empty());
}

#[test]
fn all_mods_included_in_result() {
    let mods = [
        ModInfo::new("x", "X"),
        ModInfo::new("y", "Y"),
        ModInfo::new("z", "Z"),
    ];
    let order = resolve_load_order(&mods).expect("ok");
    assert_eq!(order.len(), 3);
    for m in &mods {
        assert!(order.contains(&m.id), "{} missing from result", m.id);
    }
}

// ---------------------------------------------------------------------------
// resolve_load_order — error cases
// ---------------------------------------------------------------------------

#[test]
fn circular_dep_two_mods_returns_error() {
    let mods = [
        ModInfo::new("a", "A").with_dependency("b"),
        ModInfo::new("b", "B").with_dependency("a"),
    ];
    let err = resolve_load_order(&mods).unwrap_err();
    assert!(matches!(err, ModError::CircularDependency(_)));
}

#[test]
fn self_dep_returns_circular_error() {
    let mods = [ModInfo::new("a", "A").with_dependency("a")];
    let err = resolve_load_order(&mods).unwrap_err();
    assert!(matches!(err, ModError::CircularDependency(_)));
}

#[test]
fn missing_dep_propagates_from_order() {
    let mods = [ModInfo::new("a", "A").with_dependency("nonexistent")];
    let err = resolve_load_order(&mods).unwrap_err();
    assert!(matches!(err, ModError::UnknownDependency { .. }));
}
