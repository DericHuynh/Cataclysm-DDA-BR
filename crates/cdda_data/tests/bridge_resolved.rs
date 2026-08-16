//! Part-B bridge integration tests.
//!
//! Two guarantees, mirroring the Phase-A `roundtrip_resolved` tests:
//!
//! 1. **Deterministic fixture** — resolved JSON (with `copy-from`) → typed def
//!    → Bevy component (`DefRecord`) → export override delta → re-apply against
//!    the parent must reproduce the resolved value. This is the "import,
//!    mutate, export" round-trip in isolation.
//! 2. **Real-data report** — runs `bridge_all_types` over `data/core` and
//!    asserts zero export mismatches (proving the export adapter is lossless
//!    for every modeled `copy-from` def), without failing the build if a future
//!    category grows a modeling gap. It *does* hard-fail on genuine mismatches
//!    so real regressions in the diff engine stay visible.

use cdda_data::bridge::{
    apply_delta, compute_overrides, export_override_def, import_def, import_default_config,
};
use cdda_data::loader::Loader;
use cdda_defs_raw::raw_defs::ItemDef;
use std::path::PathBuf;
use tempfile::TempDir;

/// A named-abstract base + a child with overrides / inherited / new fields.
const FIXTURE: &str = r#"[
  {
    "type": "ITEM",
    "abstract": "rt2_base",
    "name": { "str": "base" },
    "weight": "100 g",
    "volume": "250 ml",
    "material": [ "plastic" ]
  },
  {
    "type": "ITEM",
    "id": "rt2_child",
    "copy-from": "rt2_base",
    "name": { "str": "child" },
    "weight": "454 g",
    "symbol": "%"
  }
]"#;

fn write_json(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let p = dir.path().join(name);
    std::fs::write(&p, content).unwrap();
    p
}

fn data_core_path() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/core")
}

/// Strip structural/control keys for a data-only comparison.
fn strip_control(v: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    const CONTROL: &[&str] = &[
        "type",
        "id",
        "abstract",
        "abstract_",
        "copy-from",
        "copy_from",
        "extend",
        "delete",
        "relative",
        "proportional",
    ];
    let mut out = serde_json::Map::new();
    if let Some(obj) = v.as_object() {
        for (k, x) in obj {
            if !CONTROL.contains(&k.as_str()) {
                out.insert(k.clone(), x.clone());
            }
        }
    }
    out
}

/// Full fixture: resolve → `DefRecord` (component) holds raw + typed → export
/// delta → re-apply against the parent reproduces the resolved child, and the
/// typed projection survives the whole loop unchanged.
#[test]
fn fixture_import_edit_export_is_lossless() {
    let dir = TempDir::new().unwrap();
    write_json(&dir, "f.json", FIXTURE);

    let mut loader = Loader::new(vec![dir.path().to_path_buf()]);
    loader.ingest_all();

    let (linked, failures) = loader.resolve_type_raw_with_parent("ITEM");
    assert!(failures.is_empty(), "unexpected unresolved {failures:?}");

    // Find the child and its parent resolved values.
    let child = linked
        .iter()
        .find(|(id, _, _)| id == "rt2_child")
        .expect("child resolved");
    let parent = linked
        .iter()
        .find(|(id, _, _)| id == "rt2_base")
        .expect("parent resolved");
    let (child_id, child_raw, parent_id) = (&child.0, &child.1, &child.2);
    assert_eq!(parent_id.as_deref(), Some("rt2_base"));

    // Import: resolved → typed → DefRecord component. The typed projection is
    // queried like any Bevy component and the raw source of truth is retained.
    let cfg = import_default_config();
    let record = import_def::<ItemDef>(child_id, child_raw, &cfg).expect("import child");
    assert_eq!(
        record.raw, *child_raw,
        "raw source of truth retained verbatim"
    );
    assert_eq!(record.def.name.as_ref().unwrap().singular(), "child");

    // Export: independent of the import direction, so a GUI edit can mutate the
    // typed def and we re-export without coupling the two sides.
    let delta = compute_overrides(&parent.1, child_raw);
    let exported = export_override_def("ITEM", child_id, "rt2_base", &delta);
    assert!(exported.as_object().unwrap().contains_key("weight"));
    // `volume` is inherited unchanged → must NOT appear as an override.
    assert!(!exported.as_object().unwrap().contains_key("volume"));

    let rebuilt = apply_delta(&parent.1, &delta);

    assert_eq!(
        strip_control(&rebuilt),
        strip_control(child_raw),
        "export delta re-applied to parent must reproduce the child"
    );
}

/// Real-data gate: the export adapter must be lossless for every copy-from def
/// in `data/core`. Fails on any mismatch — a regression in `compute_overrides`
/// or `apply_delta` becomes a red test rather than a silent gap.
#[test]
fn real_data_bridge_report() {
    let path = data_core_path();
    if !path.exists() {
        eprintln!("data/core not found at {path:?}; skipping real-data bridge report");
        return;
    }

    let mut loader = Loader::new(vec![path]);
    loader.ingest_all();
    let summaries = cdda_data::bridge::bridge_all_types(&loader);

    let total_ok: usize = summaries.iter().map(|s| s.ok).sum();
    let total_mismatch: usize = summaries.iter().map(|s| s.mismatches).sum();

    eprintln!("=== Part-B bridge report (data/core) ===");
    for s in &summaries {
        if s.ok > 0 || s.mismatches > 0 {
            eprintln!(
                "{:>28} ok={:<6} mismatch={}",
                s.category, s.ok, s.mismatches
            );
        }
    }
    eprintln!("Total copy-from defs verified lossless: {total_ok}");
    assert!(
        total_ok > 1000,
        "expected >1000 copy-from defs round-tripped through export, got {total_ok}"
    );
    assert_eq!(
        total_mismatch, 0,
        "export adapter lost data for {total_mismatch} def(s)"
    );
}
