//! Cargo entry point for the actor suites retained during crate consolidation.

#[path = "actor/actor_test.rs"]
mod actor_test;
#[path = "actor/ap_system_test.rs"]
mod ap_system_test;
#[path = "actor/bionics_system_test.rs"]
mod bionics_system_test;
#[path = "actor/bionics_test.rs"]
mod bionics_test;
#[path = "actor/effects_system_test.rs"]
mod effects_system_test;
#[path = "actor/healing_system_test.rs"]
mod healing_system_test;
#[path = "actor/morale_system_test.rs"]
mod morale_system_test;
#[path = "actor/morale_test.rs"]
mod morale_test;
#[path = "actor/movement_system_test.rs"]
mod movement_system_test;
#[path = "actor/movement_test.rs"]
mod movement_test;
#[path = "actor/status_effect_test.rs"]
mod status_effect_test;
#[path = "actor/temperature_system_test.rs"]
mod temperature_system_test;
#[path = "actor/temperature_test.rs"]
mod temperature_test;
#[path = "actor/turn_system_test.rs"]
mod turn_system_test;
#[path = "actor/vision_system_test.rs"]
mod vision_system_test;
#[path = "actor/vision_test.rs"]
mod vision_test;

/// A nested suite must have an explicit module in its Cargo integration target.
/// Check all three migration directories so adding a dormant file fails CI.
#[test]
fn migrated_suite_modules_remain_discoverable() {
    use std::collections::BTreeSet;
    use std::path::Path;

    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for (directory, source) in [
        ("actor", include_str!("migrated_actor.rs")),
        ("combat", include_str!("migrated_combat.rs")),
        ("inventory", include_str!("migrated_inventory.rs")),
    ] {
        let declared: BTreeSet<_> = source
            .lines()
            .filter_map(|line| line.strip_prefix("#[path = \""))
            .map(|path| path.strip_suffix("\"]").expect("valid path attribute"))
            .map(str::to_owned)
            .collect();
        let on_disk: BTreeSet<_> = std::fs::read_dir(tests.join(directory))
            .expect("migrated test directory exists")
            .map(|entry| entry.expect("read migrated test entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .map(|path| {
                format!(
                    "{directory}/{}",
                    path.file_name().unwrap().to_str().unwrap()
                )
            })
            .collect();
        assert_eq!(
            declared, on_disk,
            "unwired or stale {directory} test module"
        );
        for path in declared {
            let module = Path::new(&path).file_stem().unwrap().to_str().unwrap();
            assert!(
                source.contains(&format!("#[path = \"{path}\"]\nmod {module};")),
                "{path} must be an unconditional module, not just a path attribute"
            );
        }
    }
}
