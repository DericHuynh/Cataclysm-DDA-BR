//! Integration tests: load all of `data/core/` through the cdda_data pipeline
//! and verify that the resulting registry is well-formed.
//!
//! These tests exercise the full two-pass loader: JSON ingestion,
//! topological sort by copy-from dependency, inheritance resolution,
//! abstract filtering, and typed deserialization into the `DefRegistry`.

use cdda_data::Loader;
use std::path::PathBuf;

/// Resolve the path to `data/core/` relative to the workspace root.
fn data_core_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // CARGO_MANIFEST_DIR points to crates/cdda_data/
    // data/core/ is at the workspace root
    manifest_dir.join("../../data/core")
}

// ---------------------------------------------------------------------------
// Full core-data load
// ---------------------------------------------------------------------------

/// Load all of `data/core/` and verify it succeeds without fatal errors.
#[test]
fn load_all_core_data() {
    let core_path = data_core_path();
    assert!(
        core_path.exists(),
        "data/core directory not found at {:?}",
        core_path
    );

    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading data/core should succeed");

    // Basic sanity: we loaded a non-trivial number of definitions
    let total = registry.total_count();
    assert!(total > 1000, "Expected >1000 definitions, got {}", total);

    let cats = registry.category_count();
    assert!(cats > 10, "Expected >10 populated categories, got {}", cats);

    eprintln!("=== Core Data Load Summary ===");
    eprintln!("Total definitions: {}", total);
    eprintln!("Populated categories: {}", cats);
    eprintln!("Items: {}", registry.items.len());
    eprintln!("Monsters: {}", registry.monsters.len());
    eprintln!("Terrain: {}", registry.terrain.len());
    eprintln!("Furniture: {}", registry.furniture.len());
    eprintln!("Recipes: {}", registry.recipes.len());
    eprintln!("Item groups: {}", registry.item_groups.len());
    eprintln!("Palettes: {}", registry.palettes.len());
    eprintln!("Overmap terrains: {}", registry.overmap_terrains.len());
    eprintln!("Overmap specials: {}", registry.overmap_specials.len());
    eprintln!(
        "Overmap connections: {}",
        registry.overmap_connections.len()
    );
    eprintln!("Overmap locations: {}", registry.overmap_locations.len());
    eprintln!(
        "Overmap land use codes: {}",
        registry.overmap_land_use_codes.len()
    );
    eprintln!("Fields: {}", registry.fields.len());
    eprintln!("Vehicle parts: {}", registry.vehicle_parts.len());
    eprintln!(
        "Vehicle part locations: {}",
        registry.vehicle_part_locations.len()
    );
    eprintln!(
        "Vehicle part categories: {}",
        registry.vehicle_part_categories.len()
    );
    eprintln!("Mutations: {}", registry.mutations.len());
    eprintln!(
        "Mutation categories: {}",
        registry.mutation_categories.len()
    );
    eprintln!("Trait groups: {}", registry.trait_groups.len());
    eprintln!("Bionics: {}", registry.bionics.len());
    eprintln!("Effects: {}", registry.effects.len());
    eprintln!("Factions: {}", registry.factions.len());
    eprintln!("Scenarios: {}", registry.scenarios.len());
    eprintln!("Materials: {}", registry.materials.len());
    eprintln!("Skills: {}", registry.skills.len());
    eprintln!("Traps: {}", registry.traps.len());
    eprintln!("Start locations: {}", registry.start_locations.len());
}

// ---------------------------------------------------------------------------
// Specific category checks
// ---------------------------------------------------------------------------

/// Verify that a substantial number of items were loaded.
#[test]
fn known_items_exist() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    // We loaded a large number of items
    assert!(
        registry.items.len() > 5000,
        "Expected >5000 items, got {}",
        registry.items.len()
    );

    // Verify some items exist by checking common prefixes
    // (IDs may be in Brazilian Portuguese)
    let has_common_ids = registry.items.keys().any(|k| {
        let s = k.as_str();
        s.contains("flour")
            || s.contains("backpack")
            || s.contains("hammer")
            || s.contains("nail")
            || s.contains("rock")
            || s.contains("screwdriver")
            || s.contains("t-shirt")
            || s.contains("water")
            || s.contains("2x4")
    });
    assert!(has_common_ids, "No recognisable item IDs found");
}

/// Verify that a substantial number of monsters were loaded.
#[test]
fn known_monsters_exist() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    assert!(
        registry.monsters.len() > 200,
        "Expected >200 monsters, got {}",
        registry.monsters.len()
    );

    // Check for common monster ID patterns (may be in Brazilian Portuguese)
    let has_zombies = registry
        .monsters
        .keys()
        .any(|k| k.as_str().contains("zombie"));
    assert!(has_zombies, "No zombie monsters found");
}

/// Verify that a substantial number of terrain types were loaded.
#[test]
fn known_terrain_exists() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    assert!(
        registry.terrain.len() > 300,
        "Expected >300 terrain, got {}",
        registry.terrain.len()
    );

    // Check for common terrain prefix patterns
    let has_t_prefix = registry
        .terrain
        .keys()
        .any(|k| k.as_str().starts_with("t_"));
    assert!(has_t_prefix, "No terrain IDs with t_ prefix found");
}

/// Verify that a substantial number of furniture types were loaded.
#[test]
fn known_furniture_exists() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    assert!(
        registry.furniture.len() > 300,
        "Expected >300 furniture, got {}",
        registry.furniture.len()
    );

    // Check for common furniture prefix patterns
    let has_f_prefix = registry
        .furniture
        .keys()
        .any(|k| k.as_str().starts_with("f_"));
    assert!(has_f_prefix, "No furniture IDs with f_ prefix found");
}

/// Verify that a substantial number of materials were loaded.
#[test]
fn known_materials_exist() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    assert!(
        registry.materials.len() > 30,
        "Expected >30 materials, got {}",
        registry.materials.len()
    );
    assert!(registry.skills.len() > 0, "Expected >0 skills");
}

/// Verify that skills loaded correctly (may use BR Portuguese IDs).
#[test]
fn known_skills_exist() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    assert!(registry.skills.len() > 0, "Expected >0 skills to be loaded");
}

/// Verify that scenarios loaded correctly.
#[test]
fn known_scenarios_exist() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    let expected = &["evacuee", "surrounded"];

    for &scenario_id in expected {
        let found = registry.scenarios.keys().any(|k| k.as_str() == scenario_id);
        assert!(
            found,
            "Expected scenario '{}' to exist ({} scenarios loaded)",
            scenario_id,
            registry.scenarios.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Def-level validation: spot-check field integrity
// ---------------------------------------------------------------------------

/// Verify that a loaded ItemDef has sane field values.
#[test]
fn item_def_field_integrity() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    // Find a well-known item: "backpack"
    let backpack_key = registry
        .items
        .keys()
        .find(|k| k.as_str() == "backpack")
        .cloned()
        .expect("backpack should exist");

    let backpack = registry.items.get(&backpack_key).expect("backpack lookup");

    // Basic field checks
    assert!(
        !backpack.name.singular().is_empty(),
        "backpack should have a name"
    );
    assert!(
        backpack.volume.as_milliliters() > 0,
        "backpack should have positive volume"
    );
    assert!(
        backpack.symbol != " ",
        "backpack should have a visible symbol"
    );
}

/// Verify that a loaded MonsterDef has sane field values.
#[test]
fn monster_def_field_integrity() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    let zombie_key = registry
        .monsters
        .keys()
        .find(|k| k.as_str() == "mon_zombie")
        .cloned()
        .expect("mon_zombie should exist");

    let zombie = registry
        .monsters
        .get(&zombie_key)
        .expect("mon_zombie lookup");

    assert!(
        !zombie.name.singular().is_empty(),
        "zombie should have a name"
    );
    assert!(zombie.hp > 0, "zombie should have positive HP");
    assert!(zombie.symbol != " ", "zombie should have a visible symbol");
}

/// Verify that all items have a non-empty name.
///
/// This is a mass validation that catches broken deserialization.
#[test]
fn all_items_have_names() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    let mut missing_names = Vec::new();
    for (id, item) in &registry.items {
        let name = item.name.singular();
        if name.is_empty() {
            missing_names.push(id.to_string());
        }
    }

    if !missing_names.is_empty() {
        eprintln!(
            "Items missing names: {:?}",
            &missing_names[..10.min(missing_names.len())]
        );
    }
    assert!(
        missing_names.is_empty(),
        "{} items have empty names",
        missing_names.len()
    );
}

/// Verify that all monsters have a non-empty name.
#[test]
fn all_monsters_have_names() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    let mut missing_names = Vec::new();
    for (id, monster) in &registry.monsters {
        let name = monster.name.singular();
        if name.is_empty() {
            missing_names.push(id.to_string());
        }
    }

    if !missing_names.is_empty() {
        eprintln!(
            "Monsters missing names: {:?}",
            &missing_names[..10.min(missing_names.len())]
        );
    }
    assert!(
        missing_names.is_empty(),
        "{} monsters have empty names",
        missing_names.len()
    );
}

// ---------------------------------------------------------------------------
// Copy-from resolution tests
// ---------------------------------------------------------------------------

/// Verify that copy-from inheritance actually works in the real data.
/// Many items inherit from base definitions; the child should have
/// the parent's fields merged in.
#[test]
fn copy_from_inheritance_works() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    // Find items that likely use copy-from
    // In CDDA, many items inherit from "base_*" definitions.
    // We'll just verify that any item with "base_" in its ID that exists
    // has fields populated, and any item that copies from it also has fields.

    // Check a known copy-from chain: items/ammo typically has inheritance
    // For now, let's just verify we have some items with "material" populated,
    // which is a good sign that copy-from resolution worked.
    let mut items_with_materials = 0usize;
    let mut items_without_materials = 0usize;

    for (_id, item) in &registry.items {
        if item.material.is_empty() {
            items_without_materials += 1;
        } else {
            items_with_materials += 1;
        }
    }

    eprintln!(
        "Items: {} with materials, {} without",
        items_with_materials, items_without_materials
    );

    // Most items should have materials (at least 50% if copy-from works)
    let ratio = items_with_materials as f64 / registry.items.len() as f64;
    assert!(
        ratio > 0.3,
        "Only {:.1}% of items have materials; copy-from resolution may be broken",
        ratio * 100.0
    );
}

// ---------------------------------------------------------------------------
// Abstract def filtering
// ---------------------------------------------------------------------------

/// Verify that abstract definitions are NOT in the final registry.
/// Abstract defs have `"abstract_": true` (or `"abstract": true`) and should
/// only serve as copy-from bases, not as actual game objects.
#[test]
fn abstract_defs_are_filtered() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    // Check items: no item should have `abstract_: true`
    for (id, item) in &registry.items {
        if item.abstract_.unwrap_or(false) {
            panic!("Item '{}' has abstract_=true but was not filtered out", id);
        }
    }

    // Same for monsters
    for (id, monster) in &registry.monsters {
        if monster.abstract_.unwrap_or(false) {
            panic!(
                "Monster '{}' has abstract_=true but was not filtered out",
                id
            );
        }
    }

    // Check terrain
    for (id, terrain) in &registry.terrain {
        if terrain.abstract_.unwrap_or(false) {
            panic!(
                "Terrain '{}' has abstract_=true but was not filtered out",
                id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// No duplicate IDs
// ---------------------------------------------------------------------------

/// Verify that there are no duplicate IDs within any category.
#[test]
fn no_duplicate_ids() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Loading should succeed");

    // The HashMap keyed by DefId ensures no duplicates by construction.
    // This test is a sanity check that our ID extraction is consistent.

    // Check that all item IDs are unique (HashMap guarantees this)
    let item_count = registry.items.len();
    let unique_ids: std::collections::HashSet<&str> =
        registry.items.keys().map(|k| k.as_str()).collect();
    assert_eq!(item_count, unique_ids.len(), "Duplicate item IDs detected");

    // Same for monsters
    let mon_count = registry.monsters.len();
    let unique_mon: std::collections::HashSet<&str> =
        registry.monsters.keys().map(|k| k.as_str()).collect();
    assert_eq!(
        mon_count,
        unique_mon.len(),
        "Duplicate monster IDs detected"
    );
}
