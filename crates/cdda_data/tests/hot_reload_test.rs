//! Hot-reload tests — verify that IDs and state survive reload cycles.
//!
//! Tests cover:
//! 1. StringInterner append-only invariant (in-memory)
//! 2. FlagMap index stability (in-memory)
//! 3. CDDA patch idempotency (in-memory)
//! 4. Filesystem-based reload: write JSON → load → add file → reload → verify

use cdda_data::interner::StringInterner;
use cdda_data::loader::Loader;
use cdda_data::patch::apply_cdda_patch;
use cdda_data::flags::{ItemFlagRegistry, MonsterFlagRegistry};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Write a JSON string to a file inside a temp directory.
fn write_json(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let path = dir.path().join(filename);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

/// Produce a minimal CDDA items JSON array string.
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
                r#"  {{
    "type": "ITEM",
    "id": "{id}",
    "name": "{id}",
    "volume": "250 ml",
    "weight": "100 g",
    "flags": [{flags_str}]
  }}"#
            )
        })
        .collect();
    format!("[\n{}\n]\n", entries.join(",\n"))
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. StringInterner — append-only invariant
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn interner_preserves_ids_across_rebuilds() {
    let mut interner = StringInterner::default();
    let sword_id = interner.intern("machete");
    let rock_id = interner.intern("rock");
    assert_eq!(sword_id, 0);
    assert_eq!(rock_id, 1);

    let mut re_interner = StringInterner::default();
    for id in &["machete", "rock", "new_item"] {
        re_interner.intern(id);
    }
    assert_eq!(re_interner.intern("machete"), 0, "machete ID changed");
    assert_eq!(re_interner.intern("rock"), 1, "rock ID changed");
    assert_eq!(re_interner.intern("new_item"), 2, "new item got wrong ID");
    assert_eq!(re_interner.resolve(0), Some("machete"));
    assert_eq!(re_interner.resolve(1), Some("rock"));
}

#[test]
fn interner_same_string_returns_same_id() {
    let mut interner = StringInterner::default();
    let a = interner.intern("FLAMING");
    let b = interner.intern("FLAMING");
    let c = interner.intern("FLAMING");
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(interner.resolve(a), Some("FLAMING"));
}

#[test]
fn interner_handles_empty_and_special_strings() {
    let mut interner = StringInterner::default();
    let empty = interner.intern("");
    let underscore = interner.intern("_");
    let spaces = interner.intern("has spaces");
    assert_eq!(interner.resolve(empty), Some(""));
    assert_eq!(interner.resolve(underscore), Some("_"));
    assert_eq!(interner.resolve(spaces), Some("has spaces"));
}

#[test]
fn interner_get_does_not_allocate() {
    let mut interner = StringInterner::default();
    assert_eq!(interner.get("nonexistent"), None);
    interner.intern("exists");
    assert_eq!(interner.get("exists"), Some(0));
    assert_eq!(interner.get("nonexistent"), None);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. FlagMap — append-only flag indices
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn flag_map_preserves_indices_across_reload() {
    let mut registry = ItemFlagRegistry::default();
    let fire = registry.0.register("FIRE");
    let wet = registry.0.register("WET");
    assert_eq!(fire, 0);
    assert_eq!(wet, 1);

    let mut re_registry = ItemFlagRegistry::default();
    let fire2 = re_registry.0.register("FIRE");
    let wet2 = re_registry.0.register("WET");
    assert_eq!(fire2, 0, "FIRE index changed");
    assert_eq!(wet2, 1, "WET index changed");
    assert_eq!(re_registry.0.register("HOT"), 2);
}

#[test]
fn flag_map_register_is_idempotent() {
    let mut registry = MonsterFlagRegistry::default();
    let a = registry.0.register("FLIES");
    let b = registry.0.register("FLIES");
    let c = registry.0.register("FLIES");
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, 0);
}

#[test]
fn separate_categories_have_independent_indices() {
    let mut item_flags = ItemFlagRegistry::default();
    let mut monster_flags = MonsterFlagRegistry::default();
    assert_eq!(item_flags.0.register("FIRE"), 0);
    assert_eq!(monster_flags.0.register("FIRE"), 0);
}

#[test]
fn flag_map_to_bitset_round_trip() {
    let mut registry = ItemFlagRegistry::default();
    let flags = vec!["FIRE".to_string(), "WET".to_string(), "HOT".to_string()];
    let bs = registry.0.to_bitset(&flags);
    assert!(bs.contains(0));
    assert!(bs.contains(1));
    assert!(bs.contains(2));
    assert!(!bs.contains(3));
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. CDDA patch logic — idempotent across reloads
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn patch_extend_is_idempotent() {
    let mut base = json!({"id": "sword", "flags": ["FIRE", "WET"]});
    let mod_a = json!({"extend": {"flags": ["HOT"]}});
    apply_cdda_patch(&mut base, &mod_a);
    let first = base.clone();

    let mut base2 = json!({"id": "sword", "flags": ["FIRE", "WET"]});
    apply_cdda_patch(&mut base2, &mod_a);
    assert_eq!(first, base2, "patch not idempotent");

    let flags: Vec<&str> = base2["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(flags.contains(&"FIRE") && flags.contains(&"HOT"));
    assert_eq!(flags.len(), 3, "extend should not duplicate");
}

#[test]
fn patch_delete_is_idempotent() {
    let mut base = json!({"id": "sword", "flags": ["FIRE", "WET", "HOT"]});
    let mod_a = json!({"delete": {"flags": ["WET"]}});
    apply_cdda_patch(&mut base, &mod_a);
    let first = base.clone();
    let mut base2 = json!({"id": "sword", "flags": ["FIRE", "WET", "HOT"]});
    apply_cdda_patch(&mut base2, &mod_a);
    assert_eq!(first, base2);
}

#[test]
fn patch_multiple_mods_in_order() {
    let mut base = json!({"id": "soup", "flags": ["EATEN_HOT"]});
    apply_cdda_patch(
        &mut base,
        &json!({"extend": {"flags": ["NUTRIENT_OVERRIDE"]}, "volume": "500 ml"}),
    );
    apply_cdda_patch(
        &mut base,
        &json!({"delete": {"flags": ["EATEN_HOT"]}, "weight": 300}),
    );
    let flags: Vec<&str> = base["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(!flags.contains(&"EATEN_HOT"));
    assert!(flags.contains(&"NUTRIENT_OVERRIDE"));
    assert_eq!(base["volume"], "500 ml");
    assert_eq!(base["weight"], 300);
}

#[test]
fn patch_nested_objects_recurse() {
    let mut base = json!({"armor": {"coverage": 50, "encumbrance": 5}});
    apply_cdda_patch(&mut base, &json!({"armor": {"coverage": 75}}));
    assert_eq!(base["armor"]["coverage"], 75);
    assert_eq!(base["armor"]["encumbrance"], 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Filesystem hot-reload — write → load → modify → reload → verify
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn filesystem_hot_reload_preserves_state() {
    // ── Create temp dir with initial data ────────────────────────────────
    let dir = TempDir::new().unwrap();
    write_json(
        &dir,
        "data/core/items.json",
        &items_json(&[
            ("rock", &[]),
            ("stick", &["FLAMMABLE"]),
            ("torch", &["FLAMMABLE", "FIRE"]),
        ]),
    );

    // ── First load ───────────────────────────────────────────────────────
    let mut loader = Loader::new(vec![dir.path().join("data/core")]);
    let mut interner = StringInterner::default();
    let mut flag_registry = ItemFlagRegistry::default();

    // Ingest and intern everything from the raw defs.
    let raw_map = loader.ingest_all();
    let mut item_count = 0usize;
    let mut ids: Vec<u32> = vec![];

    for raw_defs in raw_map.values() {
        for raw in raw_defs {
            if let Some(ref id_str) = raw.id {
                let id = interner.intern(id_str);
                ids.push(id);
                // Intern flags
                if let Some(flags) = raw.value.get("flags").and_then(|v| v.as_array()) {
                    for flag in flags {
                        if let Some(s) = flag.as_str() {
                            flag_registry.0.register(s);
                        }
                    }
                }
                item_count += 1;
            }
        }
    }

    assert_eq!(item_count, 3, "should find 3 items on first load");
    assert_eq!(ids.len(), 3);
    assert_eq!(ids, vec![0, 1, 2]);
    assert_eq!(
        flag_registry
            .0
            .try_idx("FLAMMABLE")
            .expect("FLAMMABLE should be registered"),
        0
    );
    assert_eq!(
        flag_registry
            .0
            .try_idx("FIRE")
            .expect("FIRE should be registered"),
        1
    );

    // ── Hot-reload: add a new file without removing the old one ──────────
    // The loader instance is fresh, simulating a full re-build triggered
    // by Bevy's asset watcher.
    let mut re_loader = Loader::new(vec![dir.path().join("data/core")]);

    // Simulate a mod being enabled — write a new file.
    write_json(
        &dir,
        "data/mod_flare/items.json",
        &items_json(&[("flare", &["FIRE", "HOT"])]),
    );

    re_loader = re_loader.with_dir(dir.path().join("data/mod_flare"));
    let raw_map_v2 = re_loader.ingest_all();
    let mut re_interner = StringInterner::default();
    let mut re_flags = ItemFlagRegistry::default();
    let mut ids_v2: Vec<u32> = vec![];
    let mut names_v2: Vec<String> = vec![];

    for raw_defs in raw_map_v2.values() {
        for raw in raw_defs {
            if let Some(ref id_str) = raw.id {
                let id = re_interner.intern(id_str);
                ids_v2.push(id);
                names_v2.push(id_str.clone());
                if let Some(flags) = raw.value.get("flags").and_then(|v| v.as_array()) {
                    for flag in flags {
                        if let Some(s) = flag.as_str() {
                            re_flags.0.register(s);
                        }
                    }
                }
            }
        }
    }

    // ── Verify IDs are stable ────────────────────────────────────────────
    // Rock, stick, torch should still be in order, flare appended.
    assert_eq!(ids_v2.len(), 4, "should find 3 original + 1 new item");
    for (i, name) in ["rock", "stick", "torch", "flare"].iter().enumerate() {
        assert_eq!(
            re_interner.resolve(i as u32).unwrap(),
            *name,
            "ID {} should resolve to '{}'",
            i,
            name
        );
    }

    // Flag indices preserved; new flag gets sequential index.
    assert_eq!(
        re_flags
            .0
            .try_idx("FLAMMABLE")
            .expect("FLAMMABLE should be registered"),
        0,
        "FLAMMABLE index shifted"
    );
    assert_eq!(
        re_flags
            .0
            .try_idx("FIRE")
            .expect("FIRE should be registered"),
        1,
        "FIRE index shifted"
    );
    assert_eq!(
        re_flags.0.try_idx("HOT").expect("HOT should be registered"),
        2,
        "HOT should be next available"
    );
}

#[test]
fn filesystem_reload_item_added_then_patched() {
    let dir = TempDir::new().unwrap();

    // Base: one item.
    write_json(
        &dir,
        "data/core/gear.json",
        &items_json(&[("helmet", &["HARD"])]),
    );

    let mut loader = Loader::new(vec![dir.path().join("data/core")]);
    let raw_map = loader.ingest_all();
    let mut interner = StringInterner::default();
    let mut flags = ItemFlagRegistry::default();

    for raw_defs in raw_map.values() {
        for raw in raw_defs {
            if let Some(id) = &raw.id {
                interner.intern(id);
                if let Some(arr) = raw.value.get("flags").and_then(|v| v.as_array()) {
                    for f in arr {
                        if let Some(s) = f.as_str() {
                            flags.0.register(s);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(interner.resolve(0), Some("helmet"));
    assert_eq!(
        flags.0.try_idx("HARD").expect("HARD should be registered"),
        0
    );

    // Hot-reload: a mod overrides the helmet with extend.
    let mod_json = r#"[{
    "type": "ITEM",
    "id": "helmet",
    "name": "helmet",
    "volume": "500 ml",
    "weight": "200 g",
    "extend": {"flags": ["PADDED"]}
  }]"#;
    write_json(&dir, "data/mod_comfort/gear.json", mod_json);

    let mut re_loader = Loader::new(vec![
        dir.path().join("data/core"),
        dir.path().join("data/mod_comfort"),
    ]);
    let raw_v2 = re_loader.ingest_all();
    let mut re_interner = StringInterner::default();
    let mut re_flags = ItemFlagRegistry::default();

    let mut patched_val = None;
    for raw_defs in raw_v2.values() {
        for raw in raw_defs {
            if raw.id.as_deref() == Some("helmet") {
                // Resolve manually: first def is core, second (mod) should patch it.
                if patched_val.is_none() {
                    patched_val = Some(raw.value.clone());
                } else if let Some(ref mut base) = patched_val {
                    apply_cdda_patch(base, &raw.value);
                }
            }
        }
    }

    let val = patched_val.unwrap();
    re_interner.intern(val["id"].as_str().unwrap());
    let flag_arr = val["flags"].as_array().unwrap();
    for f in flag_arr {
        re_flags.0.register(f.as_str().unwrap());
    }

    assert_eq!(re_interner.resolve(0), Some("helmet"));
    assert_eq!(
        re_flags
            .0
            .try_idx("HARD")
            .expect("HARD should be registered"),
        0,
        "HARD index shifted"
    );
    assert_eq!(
        re_flags
            .0
            .try_idx("PADDED")
            .expect("PADDED should be registered"),
        1,
        "PADDED should be next"
    );

    // Verify the patched result has both flags.
    let flag_strings: Vec<&str> = flag_arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(flag_strings.contains(&"HARD"));
    assert!(flag_strings.contains(&"PADDED"));
    assert_eq!(flag_strings.len(), 2);
}

#[test]
fn filesystem_reload_item_removed_by_mod() {
    let dir = TempDir::new().unwrap();

    write_json(
        &dir,
        "data/core/food.json",
        &items_json(&[("stew", &["EATEN_HOT", "NUTRIENT_OVERRIDE"])]),
    );

    // Mod deletes NUTRIENT_OVERRIDE from stew.
    let mod_json = r#"[{
    "type": "ITEM",
    "id": "stew",
    "name": "stew",
    "volume": "250 ml",
    "weight": "100 g",
    "delete": {"flags": ["NUTRIENT_OVERRIDE"]}
  }]"#;
    write_json(&dir, "data/mod_diet/food.json", mod_json);

    let mut loader = Loader::new(vec![
        dir.path().join("data/core"),
        dir.path().join("data/mod_diet"),
    ]);
    let raw_map = loader.ingest_all();

    let mut interner = StringInterner::default();
    let mut flags = ItemFlagRegistry::default();
    let mut final_val = None;

    for raw_defs in raw_map.values() {
        for raw in raw_defs {
            if raw.id.as_deref() == Some("stew") {
                if final_val.is_none() {
                    final_val = Some(raw.value.clone());
                } else if let Some(ref mut base) = final_val {
                    apply_cdda_patch(base, &raw.value);
                }
            }
        }
    }

    let val = final_val.unwrap();
    interner.intern(val["id"].as_str().unwrap());
    for f in val["flags"].as_array().unwrap() {
        flags.0.register(f.as_str().unwrap());
    }

    assert_eq!(interner.resolve(0), Some("stew"));
    // NUTRIENT_OVERRIDE should have been deleted.
    let remaining: Vec<&str> = val["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(remaining.contains(&"EATEN_HOT"));
    assert!(!remaining.contains(&"NUTRIENT_OVERRIDE"));
    assert_eq!(remaining.len(), 1);
    // Only one flag was registered.
    assert_eq!(
        flags
            .0
            .try_idx("EATEN_HOT")
            .expect("EATEN_HOT should be registered"),
        0
    );
}
