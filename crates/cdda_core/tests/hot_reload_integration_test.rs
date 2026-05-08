//! End-to-end hot-reload integration tests.
//!
//! Tests the full pipeline from JSON files on disk → Loader → DefRegistry,
//! then file modification → re-load → state verification.
//!
//! Covers all CDDA mod layering operations: extend, delete, override.

use cdda_core::data::interner::StringInterner;
use cdda_core::data::loader::Loader;
use cdda_core::data::patch::apply_cdda_patch;
use cdda_core::sim::flags::ItemFlagRegistry;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────

fn write_json(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let path = dir.path().join(filename);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn items_json(items: &[(&str, &[&str])]) -> String {
    let entries: Vec<String> = items
        .iter()
        .map(|(id, flags)| {
            let flags_str = flags
                .iter()
                .map(|f| format!("\"{f}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                r#"  {{ "id": "{id}", "type": "ITEM", "name": "{id}", "volume": "250 ml", "weight": "100 g", "flags": [{flags_str}] }}"#
            )
        })
        .collect();
    format!("[\n{}\n]\n", entries.join(",\n"))
}

fn raw_json(items: &[&str]) -> String {
    let entries: Vec<String> = items.iter().map(|s| format!("  {s}")).collect();
    format!("[\n{}\n]\n", entries.join(",\n"))
}

// ═══════════════════════════════════════════════════════════════════════════
// End-to-end: write → load → verify → modify → reload → verify
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_full_lifecycle_with_all_layering_ops() {
    let dir = TempDir::new().unwrap();

    // ── Step 1: Write base data (core) ─────────────────────────────────
    write_json(
        &dir,
        "core/items.json",
        &items_json(&[
            ("sword", &["DURABLE_MELEE"]),
            ("shield", &["BLOCK_WHILE_WORN"]),
            ("stew", &["EATEN_HOT"]),
        ]),
    );

    // ── Step 2: Write mod A — extend flags ─────────────────────────────
    write_json(
        &dir,
        "mod_a/items.json",
        &raw_json(&[
            r#"{ "id": "sword", "type": "ITEM", "name": "sword", "volume": "250 ml", "weight": "100 g", "extend": {"flags": ["FLAMING"]} }"#,
            r#"{ "id": "stew", "type": "ITEM", "name": "stew", "volume": "250 ml", "weight": "100 g", "extend": {"flags": ["NUTRIENT_OVERRIDE"]} }"#,
        ]),
    );

    // ── Step 3: Write mod B — override + delete ────────────────────────
    write_json(
        &dir,
        "mod_b/items.json",
        &raw_json(&[
            r#"{ "id": "sword", "type": "ITEM", "name": "sword", "volume": "250 ml", "weight": "100 g", "delete": {"flags": ["DURABLE_MELEE"]} }"#,
            r#"{ "id": "shield", "type": "ITEM", "name": "shield", "volume": "500 ml", "weight": "200 g" }"#,
            r#"{ "id": "stew", "type": "ITEM", "name": "stew", "volume": "250 ml", "weight": "100 g", "delete": {"flags": ["EATEN_HOT"]} }"#,
        ]),
    );

    // ── Step 4: First load (core + mod_a + mod_b) ──────────────────────
    let mut loader = Loader::new(vec![
        dir.path().join("core"),
        dir.path().join("mod_a"),
        dir.path().join("mod_b"),
    ]);
    let raw_map = loader.ingest_all();
    let mut interner = StringInterner::default();
    let mut flag_reg = ItemFlagRegistry::default();

    // Resolve per-ID: first occurrence is base, subsequent apply patches.
    let mut resolved: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();

    for (_, raw_defs) in raw_map.iter() {
        for raw in raw_defs {
            let Some(ref id) = raw.id else { continue };
            match resolved.get_mut(id) {
                Some(base) => apply_cdda_patch(base, &raw.value),
                None => {
                    resolved.insert(id.clone(), raw.value.clone());
                }
            }
        }
    }

    // Register flags from ALL raw data (before patching) so that even
    // flags deleted by patches are in the registry (idx won't panic).
    // Uses FlagMap::register_flags_from_json which handles top-level
    // "flags" as well as "extend.flags" / "delete.flags" in mod entries.
    for raw_defs in raw_map.values() {
        for raw in raw_defs {
            flag_reg.0.register_flags_from_json(&raw.value);
        }
    }

    // Intern everything from resolved values, sorted for deterministic IDs.
    let mut sorted_ids: Vec<_> = resolved.keys().cloned().collect();
    sorted_ids.sort();
    for id in &sorted_ids {
        interner.intern(id);
    }

    // ── Verify after first load ──────────────────────────────────────────
    assert_eq!(interner.resolve(0).unwrap(), "shield");
    assert_eq!(interner.resolve(1).unwrap(), "stew");
    assert_eq!(interner.resolve(2).unwrap(), "sword");

    // sword: DURABLE_MELEE deleted, FLAMING extended → only FLAMING
    let sword_flags: Vec<&str> = resolved["sword"]["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        !sword_flags.contains(&"DURABLE_MELEE"),
        "delete failed on sword"
    );
    assert!(sword_flags.contains(&"FLAMING"), "extend failed on sword");
    assert_eq!(sword_flags.len(), 1);

    // shield: override changed volume
    assert_eq!(resolved["shield"]["volume"], "500 ml");
    assert_eq!(resolved["shield"]["weight"], "200 g");

    // stew: EATEN_HOT deleted, NUTRIENT_OVERRIDE extended
    let stew_flags: Vec<&str> = resolved["stew"]["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(!stew_flags.contains(&"EATEN_HOT"), "delete failed on stew");
    assert!(
        stew_flags.contains(&"NUTRIENT_OVERRIDE"),
        "extend failed on stew"
    );
    assert_eq!(stew_flags.len(), 1);

    // All expected flags are registered (including those deleted by patches).
    assert!(
        flag_reg
            .0
            .try_idx("FLAMING")
            .expect("FLAMING should be registered")
            <= 4
    );
    assert!(
        flag_reg
            .0
            .try_idx("DURABLE_MELEE")
            .expect("DURABLE_MELEE should be registered")
            <= 4
    );
    assert!(
        flag_reg
            .0
            .try_idx("BLOCK_WHILE_WORN")
            .expect("BLOCK_WHILE_WORN should be registered")
            <= 4
    );
    assert!(
        flag_reg
            .0
            .try_idx("EATEN_HOT")
            .expect("EATEN_HOT should be registered")
            <= 4
    );
    assert!(
        flag_reg
            .0
            .try_idx("NUTRIENT_OVERRIDE")
            .expect("NUTRIENT_OVERRIDE should be registered")
            <= 4
    );

    // ── Step 5: Hot-reload — modify mod_b (change override) ─────────────
    write_json(
        &dir,
        "mod_b/items.json",
        &raw_json(&[
            r#"{ "id": "sword", "type": "ITEM", "name": "sword", "volume": "250 ml", "weight": "100 g", "delete": {"flags": ["DURABLE_MELEE"]} }"#,
            // shield volume changed again — now 1 L
            r#"{ "id": "shield", "type": "ITEM", "name": "shield", "volume": "1 L", "weight": "400 g" }"#,
            // stew: now extend instead of delete
            r#"{ "id": "stew", "type": "ITEM", "name": "stew", "volume": "250 ml", "weight": "100 g", "extend": {"flags": ["COLD"]} }"#,
        ]),
    );

    // Rebuild: fresh loader, same dirs.
    let mut re_loader = Loader::new(vec![
        dir.path().join("core"),
        dir.path().join("mod_a"),
        dir.path().join("mod_b"),
    ]);
    let raw_v2 = re_loader.ingest_all();
    let mut re_interner = StringInterner::default();
    let mut re_flags = ItemFlagRegistry::default();
    let mut resolved_v2: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();

    for (_, raw_defs) in raw_v2.iter() {
        for raw in raw_defs {
            let Some(ref id) = raw.id else { continue };
            match resolved_v2.get_mut(id) {
                Some(base) => apply_cdda_patch(base, &raw.value),
                None => {
                    resolved_v2.insert(id.clone(), raw.value.clone());
                }
            }
        }
    }

    // Register flags from ALL raw v2 data (before patching) so even
    // flags deleted by patches are in the registry.
    for raw_defs in raw_v2.values() {
        for raw in raw_defs {
            re_flags.0.register_flags_from_json(&raw.value);
        }
    }

    let mut sorted_v2: Vec<_> = resolved_v2.keys().cloned().collect();
    sorted_v2.sort();
    for id in &sorted_v2 {
        re_interner.intern(id);
    }

    // ── Verify after hot-reload: IDs stable, data reflects new mod_b ────
    assert_eq!(re_interner.resolve(0), Some("shield"), "ID 0 shifted");
    assert_eq!(re_interner.resolve(1), Some("stew"), "ID 1 shifted");
    assert_eq!(re_interner.resolve(2), Some("sword"), "ID 2 shifted");

    // sword unchanged (same mod_b content for sword)
    let sword2: Vec<&str> = resolved_v2["sword"]["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(sword2.contains(&"FLAMING"));
    assert!(!sword2.contains(&"DURABLE_MELEE"));
    assert_eq!(sword2.len(), 1);

    // shield volume changed in mod_b reload
    assert_eq!(resolved_v2["shield"]["volume"], "1 L");
    assert_eq!(resolved_v2["shield"]["weight"], "400 g");

    // stew: now has EATEN_HOT (base), NUTRIENT_OVERRIDE (mod_a extend),
    //      and COLD (mod_b extend, replacing the previous delete).
    let stew2: Vec<&str> = resolved_v2["stew"]["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(stew2.contains(&"EATEN_HOT"), "base flag should survive");
    assert!(
        stew2.contains(&"NUTRIENT_OVERRIDE"),
        "mod_a extend should survive"
    );
    assert!(stew2.contains(&"COLD"), "new mod_b extend should apply");
    assert_eq!(stew2.len(), 3);

    // Flag indices preserved across reload.
    assert!(
        re_flags
            .0
            .try_idx("FLAMING")
            .expect("FLAMING should be registered")
            <= 5
    );
    assert!(
        re_flags
            .0
            .try_idx("DURABLE_MELEE")
            .expect("DURABLE_MELEE should be registered")
            <= 5
    );
    assert!(
        re_flags
            .0
            .try_idx("COLD")
            .expect("COLD should be registered")
            <= 5
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Content actually changed — verifies diff detection
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn content_change_detection_across_reload() {
    let dir = TempDir::new().unwrap();

    // Version 1.
    write_json(
        &dir,
        "data/weapons.json",
        &items_json(&[("axe", &["DURABLE_MELEE"])]),
    );

    let loader = || Loader::new(vec![dir.path().join("data")]);
    let mut l = loader();
    let v1 = l.ingest_all();
    let v1_items: Vec<_> = v1.values().flatten().filter_map(|r| r.id.clone()).collect();
    assert_eq!(v1_items, vec!["axe"]);

    // Version 2: add a new item.
    write_json(
        &dir,
        "data/weapons.json",
        &items_json(&[("axe", &["DURABLE_MELEE"]), ("mace", &["NONCONDUCTIVE"])]),
    );

    let mut l = loader();
    let v2 = l.ingest_all();
    let v2_items: Vec<_> = v2.values().flatten().filter_map(|r| r.id.clone()).collect();
    assert_eq!(v2_items.len(), 2, "should detect added item");
    assert!(v2_items.contains(&"axe".to_string()));
    assert!(v2_items.contains(&"mace".to_string()));

    // Version 3: remove an item.
    write_json(
        &dir,
        "data/weapons.json",
        &items_json(&[("mace", &["NONCONDUCTIVE"])]),
    );

    let mut l = loader();
    let v3 = l.ingest_all();
    let v3_items: Vec<_> = v3.values().flatten().filter_map(|r| r.id.clone()).collect();
    assert_eq!(v3_items, vec!["mace"], "should detect removed item");

    // Version 4: change a flag (content change, not just item count).
    write_json(
        &dir,
        "data/weapons.json",
        &items_json(&[("mace", &["NONCONDUCTIVE", "FLAMING"])]),
    );

    let mut l = loader();
    let v4 = l.ingest_all();
    let mut interner = StringInterner::default();
    let mut flags = ItemFlagRegistry::default();
    for d in v4.values().flatten() {
        if let Some(id) = &d.id {
            interner.intern(id);
        }
        flags.0.register_flags_from_json(&d.value);
    }
    assert_eq!(
        flags
            .0
            .try_idx("FLAMING")
            .expect("FLAMING should be registered"),
        1
    );
    assert_eq!(interner.resolve(0), Some("mace"));
}
