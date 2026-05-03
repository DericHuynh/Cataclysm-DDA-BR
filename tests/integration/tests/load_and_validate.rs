//! Comprehensive integration test: validate schema AND load full data set.
//!
//! This is the single authoritative integration test for the data pipeline:
//!   1. Schema validation — every JSON definition in `data/core/` is checked
//!      against its type's typed schema. Zero tolerance for mismatches.
//!   2. Full registry load — the loaded registry must be complete, with
//!      non-zero counts for every known definition category.

use cdda_data::raw_defs::*;
use cdda_data::schema::validate_all;
use cdda_data::Loader;
use std::path::PathBuf;

/// Resolve the path to `data/core/` relative to the workspace root.
fn data_core_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = <workspace>/tests/integration
    // data/core    = <workspace>/data/core
    manifest_dir.join("../../data/core")
}

/// Macro to validate all definitions of a type and collect errors.
macro_rules! validate_type {
    ($loader:expr, $errors:expr, $type_name:literal, $rust_type:ty) => {{
        let errs = validate_all::<$rust_type>($type_name, $loader.raw_by_type());
        let count = errs.len();
        if !errs.is_empty() {
            eprintln!("  ❌ {} ({} errors)", $type_name, count);
            for (id, msgs) in &errs {
                for msg in msgs {
                    eprintln!("     {}: {}", id, msg);
                }
            }
        } else {
            eprintln!("  ✅ {}", $type_name);
        }
        $errors.extend(
            errs.into_iter()
                .map(|(k, v)| (format!("{}:{}", $type_name, k), v)),
        );
    }};
}

#[test]
fn load_and_validate() {
    // -----------------------------------------------------------------------
    // Setup
    // -----------------------------------------------------------------------
    let core_path = data_core_path();
    assert!(
        core_path.exists(),
        "data/core directory not found at {:?}. Are you running from the workspace root?",
        core_path
    );

    // =======================================================================
    // PHASE 1: Full Load
    // Run the complete two-pass pipeline (ingest + resolve).
    // After this, both the raw defs AND resolved registry are available.
    // =======================================================================
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Full pipeline load should succeed");

    // =======================================================================
    // PHASE 2: Schema Validation
    // Validate every raw definition against its type's schema.
    // Uses raw_by_type() from the loader (populated during Pass 1).
    // Zero tolerance: every def must match its schema exactly.
    // =======================================================================
    eprintln!("\n=== Schema Validation ===");

    let mut all_errors: Vec<(String, Vec<String>)> = Vec::new();

    // Validate each known type from the raw (pre-resolution) data
    validate_type!(loader, all_errors, "ITEM", ItemDef);
    validate_type!(loader, all_errors, "MONSTER", MonsterDef);
    validate_type!(loader, all_errors, "terrain", TerrainDef);
    validate_type!(loader, all_errors, "furniture", FurnitureDef);
    validate_type!(loader, all_errors, "recipe", RecipeDef);
    validate_type!(loader, all_errors, "item_group", ItemGroupDef);
    validate_type!(loader, all_errors, "palette", MapgenPaletteDef);
    validate_type!(loader, all_errors, "overmap_terrain", OvermapTerrainDef);
    validate_type!(loader, all_errors, "overmap_special", OvermapSpecialDef);
    validate_type!(
        loader,
        all_errors,
        "overmap_connection",
        OvermapConnectionDef
    );
    validate_type!(loader, all_errors, "overmap_location", OvermapLocationDef);
    validate_type!(
        loader,
        all_errors,
        "overmap_land_use_code",
        OvermapLandUseCodeDef
    );
    validate_type!(loader, all_errors, "field_type", FieldDef);
    validate_type!(loader, all_errors, "vehicle_part", VehiclePartDef);
    validate_type!(
        loader,
        all_errors,
        "vehicle_part_location",
        VehiclePartLocationDef
    );
    validate_type!(
        loader,
        all_errors,
        "vehicle_part_category",
        VehiclePartCategoryDef
    );
    validate_type!(loader, all_errors, "mutation", MutationDef);
    validate_type!(loader, all_errors, "mutation_category", MutationCategoryDef);
    validate_type!(loader, all_errors, "trait_group", TraitGroupDef);
    validate_type!(loader, all_errors, "bionic", BionicDef);
    validate_type!(loader, all_errors, "effect_type", EffectDef);
    validate_type!(loader, all_errors, "faction", FactionDef);
    validate_type!(loader, all_errors, "scenario", ScenarioDef);
    validate_type!(loader, all_errors, "material", MaterialDef);
    validate_type!(loader, all_errors, "skill", SkillDef);
    validate_type!(loader, all_errors, "trap", TrapDef);
    validate_type!(loader, all_errors, "start_location", StartLocationDef);

    // Assert 100% schema conformance — every definition must match its type
    let total_errors: usize = all_errors.iter().map(|(_, v)| v.len()).sum();
    assert!(
        total_errors == 0,
        "\n\n❌ SCHEMA VALIDATION FAILED: {} definitions failed schema conformance.\n\
         The Rust types are authoritative. Every JSON definition must match.\n\
         Fix the data or update the types. Errors by type:\n{}",
        total_errors,
        all_errors
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| format!("     {}: {} errors", k, v.len()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    eprintln!("\n  ✅ All definitions match their schemas exactly.");

    eprintln!("\n=== Registry Load ===");

    let total = registry.total_count();
    eprintln!("Total definitions: {}", total);

    // Assert every known category has loaded data
    assert!(
        registry.items.len() > 5000,
        "Items: {}",
        registry.items.len()
    );
    assert!(
        registry.monsters.len() > 150,
        "Monsters: {}",
        registry.monsters.len()
    );
    assert!(
        registry.terrain.len() > 300,
        "Terrain: {}",
        registry.terrain.len()
    );
    assert!(
        registry.furniture.len() > 300,
        "Furniture: {}",
        registry.furniture.len()
    );
    assert!(
        registry.recipes.len() > 400,
        "Recipes: {}",
        registry.recipes.len()
    );
    assert!(
        registry.item_groups.len() > 4000,
        "Item groups: {}",
        registry.item_groups.len()
    );
    assert!(
        registry.palettes.len() > 400,
        "Palettes: {}",
        registry.palettes.len()
    );
    assert!(
        registry.overmap_terrains.len() > 10,
        "Overmap terrains: {}",
        registry.overmap_terrains.len()
    );
    assert!(
        registry.overmap_specials.len() > 30,
        "Overmap specials: {}",
        registry.overmap_specials.len()
    );
    assert!(
        registry.overmap_connections.len() > 0,
        "Overmap connections: {}",
        registry.overmap_connections.len()
    );
    assert!(
        registry.overmap_locations.len() > 30,
        "Overmap locations: {}",
        registry.overmap_locations.len()
    );
    assert!(
        registry.overmap_land_use_codes.len() > 20,
        "Land use codes: {}",
        registry.overmap_land_use_codes.len()
    );
    assert!(
        registry.fields.len() > 5,
        "Fields: {}",
        registry.fields.len()
    );
    assert!(
        registry.vehicle_parts.len() > 30,
        "Vehicle parts: {}",
        registry.vehicle_parts.len()
    );
    assert!(
        registry.vehicle_part_locations.len() > 10,
        "Vehicle part locations: {}",
        registry.vehicle_part_locations.len()
    );
    assert!(
        registry.vehicle_part_categories.len() > 10,
        "Vehicle part categories: {}",
        registry.vehicle_part_categories.len()
    );
    assert!(
        registry.mutations.len() > 800,
        "Mutations: {}",
        registry.mutations.len()
    );
    assert!(
        registry.mutation_categories.len() > 20,
        "Mutation categories: {}",
        registry.mutation_categories.len()
    );
    assert!(
        registry.trait_groups.len() > 100,
        "Trait groups: {}",
        registry.trait_groups.len()
    );
    assert!(
        registry.bionics.len() > 5,
        "Bionics: {}",
        registry.bionics.len()
    );
    assert!(
        registry.effects.len() > 100,
        "Effects: {}",
        registry.effects.len()
    );
    assert!(
        registry.factions.len() > 10,
        "Factions: {}",
        registry.factions.len()
    );
    assert!(
        registry.scenarios.len() > 30,
        "Scenarios: {}",
        registry.scenarios.len()
    );
    assert!(
        registry.materials.len() > 30,
        "Materials: {}",
        registry.materials.len()
    );
    assert!(
        registry.skills.len() > 20,
        "Skills: {}",
        registry.skills.len()
    );
    assert!(registry.traps.len() > 80, "Traps: {}", registry.traps.len());
    assert!(
        registry.start_locations.len() > 50,
        "Start locations: {}",
        registry.start_locations.len()
    );

    eprintln!(
        "\n  ✅ All categories loaded successfully. {} total definitions.",
        total
    );
    eprintln!("\n=== ALL INTEGRATION CHECKS PASSED ===");
}
