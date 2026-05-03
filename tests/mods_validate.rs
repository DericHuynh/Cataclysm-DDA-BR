//! Integration test: load all mods from `data/mods/` and validate their
//! definitions against every known schema, exactly as `load_and_validate`
//! does for `data/core/`.
//!
//! This ensures:
//! 1. Every mod can be discovered by `ModManager::scan_mods`.
//! 2. Every mod's JSON data passes schema validation.
//! 3. The combined core + mod data loads without errors.

use cdda_data::for_each_raw_def_kind;
use cdda_data::mod_layer::ModManager;
use cdda_data::schema::validate_all;
use cdda_data::Loader;
use std::path::PathBuf;

fn data_core_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = <workspace>/tests/
    // data/core    = <workspace>/data/core
    manifest_dir.parent().unwrap().join("data/core")
}

fn data_mods_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = <workspace>/tests/
    // data/mods    = <workspace>/data/mods
    manifest_dir.parent().unwrap().join("data/mods")
}

#[test]
fn mods_load_and_validate() {
    let core_path = data_core_path();
    let mods_path = data_mods_path();
    assert!(core_path.exists(), "data/core not found");
    assert!(mods_path.exists(), "data/mods not found");

    // ---- Phase 1: Load core registry ----
    eprintln!("Loading core data...");
    let mut core_loader = Loader::new(vec![core_path]);
    let core_registry = core_loader.load().expect("Core data must load");
    eprintln!(
        "Core loaded: {} total definitions",
        core_registry.total_count()
    );

    // ---- Phase 2: Discover mods ----
    let mut mgr = ModManager::new(core_registry, core_loader);
    mgr.scan_mods(&mods_path).expect("Scan mods must succeed");
    eprintln!("Discovered {} mods:", mgr.available.len());
    for m in &mgr.available {
        eprintln!("  {} ({})", m.id, m.name);
    }
    assert!(!mgr.available.is_empty(), "Expected at least one mod");

    // ---- Phase 3: Validate each mod's JSON schemas ----
    let mut all_errors: Vec<(String, Vec<String>)> = Vec::new();
    let mut total_defs = 0u32;

    for mod_info in &mgr.available {
        eprintln!("\n--- Mod: {} ---", mod_info.id);

        // Load raw definitions for this mod
        let mut mod_loader = Loader::new(vec![mod_info.path.clone()]);
        mod_loader.ingest_all();

        let count: usize = mod_loader.raw_by_type().values().map(|v| v.len()).sum();
        total_defs += count as u32;
        eprintln!("  Raw defs: {}", count);

        // Validate each known type using the centralized macro.
        // This replaces 27 individual validate_type! calls.
        macro_rules! validate_one {
            ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {{
                let raw_by_type = mod_loader.raw_by_type();
                let errs = validate_all::<$def_ty>($json, raw_by_type);
                let count = errs.len();
                if !errs.is_empty() {
                    eprintln!("    {} ({} errors)", $json, count);
                    for (id, msgs) in &errs {
                        for msg in msgs {
                            eprintln!("      {}: {}", id, msg);
                        }
                    }
                } else {
                    eprintln!("    {} [ok]", $json);
                }
                all_errors.extend(
                    errs.into_iter()
                        .map(|(k, v)| (format!("{}:{}", $json, k), v)),
                );
            }};
        }
        for_each_raw_def_kind!(call validate_one);
    }

    // ---- Report ----
    let total_errors: usize = all_errors.iter().map(|(_, v)| v.len()).sum();
    eprintln!("\n=== Results ===");
    if total_errors == 0 {
        eprintln!(
            "All {} mods passed. {} raw definitions across {} types.",
            mgr.available.len(),
            total_defs,
            27,
        );
    } else {
        eprintln!(
            "FAILED: {} mod errors across {} mods. {} total defs.",
            total_errors,
            mgr.available.len(),
            total_defs,
        );
        for (id, msgs) in &all_errors {
            for msg in msgs {
                eprintln!("  {}: {}", id, msg);
            }
        }
    }

    assert_eq!(
        total_errors, 0,
        "{} schema errors in mod data",
        total_errors
    );
}
