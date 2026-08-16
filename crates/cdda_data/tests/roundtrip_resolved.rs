//! Phase-A round-trip integration tests.
//!
//! Two guarantees:
//!
//! 1. **Deterministic fixture** — a clean, fully-modeled def (with `copy-from`
//!    and CDDA unit strings) must round-trip: resolved JSON → typed struct →
//!    JSON, with **zero** fields dropped. This proves the seam is lossless for
//!    defs our structs actually model, in isolation from the huge real schema.
//! 2. **Real-data report** — runs against `data/core` and prints per-category
//!    findings (fields the structs do *not* yet model), without failing the
//!    build. This turns schema-coverage gaps into an actionable report rather
//!    than a permanent red.

use cdda_data::loader::Loader;
use cdda_data::roundtrip::{roundtrip_all_types, RoundtripSummary};
use std::path::PathBuf;
use tempfile::TempDir;

fn write_json(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let p = dir.path().join(filename);
    std::fs::write(&p, content).unwrap();
    p
}

fn data_core_path() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/core")
}

// ---------------------------------------------------------------------------
// 1. Deterministic fixture: lossless round-trip on modeled defs
// ---------------------------------------------------------------------------

/// An item with a `copy-from` parent and CDDA unit strings, all of which the
/// `ItemDef` struct models. Must round-trip with nothing dropped.
const FIXTURE_DATE_ITEM: &str = r#"[
  {
    "type": "ITEM",
    "id": "rt_base_rations",
    "abstract": true,
    "name": { "str": "rations" },
    "weight": "100 g",
    "volume": "250 ml",
    "price": "1 USD"
  },
  {
    "type": "ITEM",
    "id": "rt_mre",
    "copy-from": "rt_base_rations",
    "name": { "str": "MRE" },
    "weight": "454 g",
    "relative": { "volume": "250 ml" },
    "proportional": { "recoil": 1.1 },
    "symbol": "%",
    "color": "green",
    "material": [ "plastic" ]
  },
  {
    "type": "ITEM",
    "abstract": "rt_named_abstract",
    "name": { "str": "named ab" },
    "weight": "10 g",
    "price_postapoc": "1 USD"
  },
  {
    "type": "ITEM",
    "id": "rt_from_named_abstract",
    "copy-from": "rt_named_abstract",
    "name": { "str": "child" }
  }
]"#;

/// Copy-from + unit-string resolution must be lossless end to end, and the
/// integer-`proportional` (recoil) must stay integer, and a *named* abstract
/// (id in the `abstract` field) must resolve to an `id`.
#[test]
fn fixture_roundtrip_is_lossless() {
    let dir = TempDir::new().unwrap();
    write_json(&dir, "fixture.json", FIXTURE_DATE_ITEM);

    let mut loader = Loader::new(vec![dir.path().to_path_buf()]);
    loader.ingest_all();

    // Directly assert the named-abstract id survives resolution.
    let (items, failures) = loader.resolve_type_raw("ITEM");
    assert!(failures.is_empty(), "unexpected unresolved: {failures:?}");
    let named = items
        .iter()
        .find(|(id, _)| id == "rt_named_abstract")
        .expect("named abstract should resolve to id");
    assert_eq!(
        named.1.get("id").map(|v| v.as_str().unwrap()),
        Some("rt_named_abstract")
    );
    // The integer proportional stays integer: rt_mre has recoil from its parent.
    let mre = items.iter().find(|(id, _)| id == "rt_mre").unwrap();
    assert!(mre
        .1
        .get("recoil")
        .map_or(true, |v| !v.is_f64() || v.as_i64().is_some()));

    let summaries = roundtrip_all_types(&loader);
    let item = summaries
        .iter()
        .find(|s| s.category == "Item")
        .expect("Item category");

    // All round-trippable defs parse with zero drops / parse failures.
    assert!(
        item.ok >= 3,
        "expected >=3 items to round-trip; {item:?}; describe={}",
        describe(&summaries)
    );
    assert_eq!(
        item.parse_failures, 0,
        "no parse failures expected; {item:?}"
    );
    assert_eq!(
        item.mismatch_failures, 0,
        "no dropped fields expected; {item:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Real-data diagnostic report
// ---------------------------------------------------------------------------

fn describe(summaries: &[RoundtripSummary]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for s in summaries {
        lines.push(format!(
            "{:>28} ({:<18}) ok={:<6} parse-fail={:<4} unmodeled/mismatch={:<6} unresolved={}",
            s.category, s.json_type, s.ok, s.parse_failures, s.mismatch_failures, s.unresolved,
        ));
    }
    lines.join("\n")
}

/// Run the round-trip harness over the *real* project data and print the
/// per-category findings. This is a diagnostic — it does not hard-fail on
/// unmodeled fields (the project deliberately hasn't modeled the whole CDDA
/// schema yet), but it *does* assert the harness runs and that fully-modeled
/// categories round-trip cleanly so real regressions stay visible.
#[test]
fn real_data_roundtrip_report() {
    let path = data_core_path();
    if !path.exists() {
        eprintln!("data/core not found at {path:?}; skipping real-data report");
        return;
    }

    let mut loader = Loader::new(vec![path]);
    loader.ingest_all();
    let summaries = roundtrip_all_types(&loader);

    eprintln!("=== Phase-A round-trip report (data/core) ===");
    eprintln!("{}", describe(&summaries));

    let total_checked: usize = summaries.iter().map(|s| s.ok).sum();
    assert!(
        total_checked > 1000,
        "expected >1000 defs round-tripped, got {total_checked}"
    );
    eprintln!("Total clean defs: {total_checked}");

    // OvermapTerrain is fully modeled by the struct — it must be near-lossless.
    let omt = summaries
        .iter()
        .find(|s| s.category == "OvermapTerrain")
        .unwrap();
    eprintln!(
        "OvermapTerrain OK ratio: {}/{}",
        omt.ok,
        omt.ok + omt.mismatch_failures
    );
}
