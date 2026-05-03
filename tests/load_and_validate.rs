//! Comprehensive integration test: validate schema AND load full data set.
//!
//! This is the single authoritative integration test for the data pipeline:
//!   1. Schema validation — every JSON definition in `data/core/` is checked
//!      against its type's typed schema. Zero tolerance for mismatches.
//!   2. Full registry load — the loaded registry must be complete, with
//!      non-zero counts for every known definition category.

use cdda_data::for_each_raw_def_kind;
use cdda_data::schema::validate_all;
use cdda_data::Loader;
use std::path::PathBuf;

/// Resolve the path to `data/core/` relative to the workspace root.
fn data_core_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = <workspace>/tests/
    // data/core    = <workspace>/data/core
    manifest_dir.parent().unwrap().join("data/core")
}

/// Resolve the path to Magiclysm mod data.
fn magiclysm_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = <workspace>/tests/
    // data/mods    = <workspace>/data/mods
    manifest_dir.parent().unwrap().join("data/mods/Magiclysm")
}

#[test]
fn load_and_validate() {
    // -----------------------------------------------------------------------
    // Setup
    // -----------------------------------------------------------------------
    let core_path = data_core_path();
    let magic_path = magiclysm_path();
    assert!(
        core_path.exists(),
        "data/core directory not found at {:?}. Are you running from the workspace root?",
        core_path
    );
    assert!(
        magic_path.exists(),
        "Magiclysm directory not found at {:?}",
        magic_path
    );

    // =======================================================================
    // PHASE 1: Full Load
    // Run the complete two-pass pipeline (ingest + resolve).
    // After this, both the raw defs AND resolved registry are available.
    // =======================================================================
    eprintln!("Loading core data + Magiclysm mod...");
    let mut loader = Loader::new(vec![core_path, magic_path]);
    let registry = loader.load().expect("Full pipeline load should succeed");
    eprintln!(
        "Registry loaded: {} total definitions",
        registry.total_count()
    );

    // =======================================================================
    // PHASE 2: Schema Validation
    // Validate every raw definition against its type's schema.
    // Uses raw_by_type() from the loader (populated during Pass 1).
    // Zero tolerance: every def must match its schema exactly.
    // =======================================================================
    eprintln!("\n=== Schema Validation ===");

    let mut all_errors: Vec<(String, Vec<String>)> = Vec::new();

    // Use the centralized macro to validate each known type from raw (pre-resolution) data.
    // This replaces 27 individual validate_type! calls.
    macro_rules! validate_one {
        ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {{
            let raw_by_type = loader.raw_by_type();
            let errs = validate_all::<$def_ty>($json, raw_by_type);
            let count = errs.len();
            if !errs.is_empty() {
                eprintln!("  ❌ {} ({} errors)", $json, count);
                for (id, msgs) in &errs {
                    for msg in msgs {
                        eprintln!("     {}: {}", id, msg);
                    }
                }
            } else {
                eprintln!("  ✅ {}", $json);
            }
            all_errors.extend(
                errs.into_iter()
                    .map(|(k, v)| (format!("{}:{}", $json, k), v)),
            );
        }};
    }

    for_each_raw_def_kind!(call validate_one);

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

    // ---- New type assertions ----
    assert!(
        registry.json_flags.len() > 500,
        "Json flags: {}",
        registry.json_flags.len()
    );
    assert!(
        registry.ascii_art.len() > 300,
        "ASCII art: {}",
        registry.ascii_art.len()
    );
    assert!(
        registry.construction_groups.len() > 300,
        "Construction groups: {}",
        registry.construction_groups.len()
    );
    assert!(
        registry.item_actions.len() > 100,
        "Item actions: {}",
        registry.item_actions.len()
    );
    assert!(
        registry.techniques.len() > 150,
        "Techniques: {}",
        registry.techniques.len()
    );
    assert!(
        registry.ammunition_types.len() > 100,
        "Ammunition types: {}",
        registry.ammunition_types.len()
    );
    assert!(
        registry.morale_types.len() > 50,
        "Morale types: {}",
        registry.morale_types.len()
    );
    assert!(
        registry.scent_types.len() > 0,
        "Scent types: {}",
        registry.scent_types.len()
    );
    assert!(
        registry.movement_modes.len() > 0,
        "Movement modes: {}",
        registry.movement_modes.len()
    );
    assert!(
        registry.mood_faces.len() > 0,
        "Mood faces: {}",
        registry.mood_faces.len()
    );

    // ---- Batch B type assertions ----
    assert!(
        registry.achievements.len() > 100,
        "Achievements: {}",
        registry.achievements.len()
    );
    assert!(
        registry.body_parts.len() > 80,
        "Body parts: {}",
        registry.body_parts.len()
    );
    // Dreams have no id field, so they're not resolved by the standard pipeline.
    // TODO: Handle dreams separately when the dream system is implemented.
    // assert!(registry.dreams.len() > 50, "Dreams: {}", registry.dreams.len());
    assert!(registry.emits.len() > 30, "Emits: {}", registry.emits.len());
    assert!(
        registry.event_statistics.len() > 100,
        "Event statistics: {}",
        registry.event_statistics.len()
    );
    assert!(
        registry.harvests.len() > 200,
        "Harvests: {}",
        registry.harvests.len()
    );
    assert!(
        registry.item_migrations.len() > 500,
        "Item migrations: {}",
        registry.item_migrations.len()
    );
    assert!(
        registry.monster_groups.len() > 200,
        "Monster groups: {}",
        registry.monster_groups.len()
    );
    assert!(
        registry.mutation_types.len() > 20,
        "Mutation types: {}",
        registry.mutation_types.len()
    );
    assert!(
        registry.nested_categories.len() > 300,
        "Nested categories: {}",
        registry.nested_categories.len()
    );
    assert!(
        registry.practices.len() > 50,
        "Practices: {}",
        registry.practices.len()
    );
    assert!(
        registry.professions.len() > 100,
        "Professions: {}",
        registry.professions.len()
    );
    assert!(
        registry.proficiencies.len() > 100,
        "Proficiencies: {}",
        registry.proficiencies.len()
    );
    assert!(
        registry.scores.len() > 20,
        "Scores: {}",
        registry.scores.len()
    );
    assert!(
        registry.species.len() > 20,
        "Species: {}",
        registry.species.len()
    );
    assert!(
        registry.sub_body_parts.len() > 200,
        "Sub body parts: {}",
        registry.sub_body_parts.len()
    );
    assert!(
        registry.uncrafts.len() > 1000,
        "Uncrafts: {}",
        registry.uncrafts.len()
    );
    assert!(
        registry.vitamins.len() > 30,
        "Vitamins: {}",
        registry.vitamins.len()
    );

    // ---- Batch C type assertions ----
    assert!(
        registry.talk_topics.len() > 2000,
        "Talk topics: {}",
        registry.talk_topics.len()
    );
    assert!(
        registry.widgets.len() > 800,
        "Widgets: {}",
        registry.widgets.len()
    );
    assert!(
        registry.effects_on_condition.len() > 500,
        "EOCs: {}",
        registry.effects_on_condition.len()
    );
    assert!(
        registry.constructions.len() > 500,
        "Constructions: {}",
        registry.constructions.len()
    );
    // Snippets use `category` as key (not `id`), so they're not resolved by the standard pipeline.
    // TODO: Handle snippets separately.
    // assert!(registry.snippets.len() > 400, "Snippets: {}", registry.snippets.len());
    assert!(registry.npcs.len() > 150, "NPCs: {}", registry.npcs.len());
    assert!(
        registry.npc_classes.len() > 100,
        "NPC classes: {}",
        registry.npc_classes.len()
    );
    assert!(
        registry.requirements.len() > 300,
        "Requirements: {}",
        registry.requirements.len()
    );
    assert!(
        registry.spells.len() > 200,
        "Spells: {}",
        registry.spells.len()
    );
    assert!(
        registry.vehicles.len() > 200,
        "Vehicles: {}",
        registry.vehicles.len()
    );
    assert!(
        registry.city_buildings.len() > 300,
        "City buildings: {}",
        registry.city_buildings.len()
    );
    assert!(
        registry.mission_definitions.len() > 200,
        "Mission definitions: {}",
        registry.mission_definitions.len()
    );
    assert!(
        registry.event_transformations.len() > 100,
        "Event transformations: {}",
        registry.event_transformations.len()
    );
    assert!(
        registry.martial_arts.len() > 20,
        "Martial arts: {}",
        registry.martial_arts.len()
    );
    assert!(
        registry.monster_attacks.len() > 60,
        "Monster attacks: {}",
        registry.monster_attacks.len()
    );
    assert!(
        registry.weakpoint_sets.len() > 30,
        "Weakpoint sets: {}",
        registry.weakpoint_sets.len()
    );
    assert!(
        registry.recipe_groups.len() > 40,
        "Recipe groups: {}",
        registry.recipe_groups.len()
    );
    assert!(
        registry.monster_flags.len() > 100,
        "Monster flags: {}",
        registry.monster_flags.len()
    );
    assert!(
        registry.activity_types.len() > 100,
        "Activity types: {}",
        registry.activity_types.len()
    );
    assert!(
        registry.ammo_effects.len() > 60,
        "Ammo effects: {}",
        registry.ammo_effects.len()
    );
    assert!(
        registry.tool_qualities.len() > 60,
        "Tool qualities: {}",
        registry.tool_qualities.len()
    );
    assert!(
        registry.faults.len() > 50,
        "Faults: {}",
        registry.faults.len()
    );
    assert!(
        registry.map_extras.len() > 50,
        "Map extras: {}",
        registry.map_extras.len()
    );
    assert!(
        registry.fault_fixes.len() > 50,
        "Fault fixes: {}",
        registry.fault_fixes.len()
    );
    assert!(
        registry.ter_furn_transforms.len() > 40,
        "Ter/furn transforms: {}",
        registry.ter_furn_transforms.len()
    );
    assert!(
        registry.connect_groups.len() > 20,
        "Connect groups: {}",
        registry.connect_groups.len()
    );
    assert!(
        registry.attack_vectors.len() > 20,
        "Attack vectors: {}",
        registry.attack_vectors.len()
    );
    assert!(
        registry.item_categories.len() > 20,
        "Item categories: {}",
        registry.item_categories.len()
    );
    assert!(
        registry.oter_visions.len() > 20,
        "Oter visions: {}",
        registry.oter_visions.len()
    );
    assert!(
        registry.character_mods.len() > 15,
        "Character mods: {}",
        registry.character_mods.len()
    );
    assert!(
        registry.weapon_categories.len() > 15,
        "Weapon categories: {}",
        registry.weapon_categories.len()
    );
    // Rotatable symbols use `tuple` as key (not `id`), not resolved by standard pipeline.
    // assert!(registry.rotatable_symbols.len() > 15, "Rotatable symbols: {}", registry.rotatable_symbols.len());
    assert!(
        registry.weather_types.len() > 10,
        "Weather types: {}",
        registry.weather_types.len()
    );
    assert!(
        registry.body_graphs.len() > 10,
        "Body graphs: {}",
        registry.body_graphs.len()
    );
    assert!(
        registry.limb_scores.len() > 10,
        "Limb scores: {}",
        registry.limb_scores.len()
    );
    assert!(
        registry.construction_categories.len() > 10,
        "Construction categories: {}",
        registry.construction_categories.len()
    );
    assert!(
        registry.addiction_types.len() > 10,
        "Addiction types: {}",
        registry.addiction_types.len()
    );
    assert!(registry.gates.len() > 5, "Gates: {}", registry.gates.len());
    assert!(
        registry.damage_types.len() > 5,
        "Damage types: {}",
        registry.damage_types.len()
    );
    assert!(
        registry.anatomies.len() > 0,
        "Anatomies: {}",
        registry.anatomies.len()
    );
    assert!(
        registry.end_screens.len() > 5,
        "End screens: {}",
        registry.end_screens.len()
    );
    assert!(
        registry.conducts.len() > 10,
        "Conducts: {}",
        registry.conducts.len()
    );
    assert!(
        registry.proficiency_categories.len() > 10,
        "Proficiency categories: {}",
        registry.proficiency_categories.len()
    );
    assert!(
        registry.faction_missions.len() > 10,
        "Faction missions: {}",
        registry.faction_missions.len()
    );
    assert!(
        registry.fault_groups.len() > 10,
        "Fault groups: {}",
        registry.fault_groups.len()
    );
    assert!(
        registry.jmath_functions.len() > 10,
        "Jmath functions: {}",
        registry.jmath_functions.len()
    );
    assert!(
        registry.recipe_categories.len() > 10,
        "Recipe categories: {}",
        registry.recipe_categories.len()
    );
    assert!(
        registry.region_terrain_furnitures.len() > 20,
        "Region terrain furnitures: {}",
        registry.region_terrain_furnitures.len()
    );
    // Uses `item` as key (not `id`), not resolved by standard pipeline.
    // assert!(registry.profession_item_substitutions.len() > 15, ...);
    assert!(
        registry.climbing_aids.len() > 15,
        "Climbing aids: {}",
        registry.climbing_aids.len()
    );
    // Uses non-standard id field, not resolved by standard pipeline.
    // assert!(registry.oter_id_migrations.len() > 15, ...);

    eprintln!(
        "\n  ✅ All categories loaded successfully. {} total definitions.",
        total
    );
    eprintln!("\n=== ALL INTEGRATION CHECKS PASSED ===");
}

/// Verify that we can generate valid JSON Schema files for all 27 definition
/// types and that the produced files parse back as valid JSON.
#[test]
fn generate_and_validate_schemas() {
    let tmp = std::env::temp_dir().join("cdda_schemas_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    cdda_data::schema::write_all_schemas(&tmp).unwrap();

    // Verify each generated file is valid JSON and has expected structure.
    // The list of schema filenames is auto-generated from the centralized macro.
    macro_rules! schema_file {
        ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {
            format!("{}.schema.json", $json)
        };
    }
    let schema_names: Vec<String> = for_each_raw_def_kind!(list schema_file);

    for name in &schema_names {
        let path = tmp.join(name);
        assert!(path.exists(), "Schema file not found: {:?}", path);

        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", name, e));
        let schema: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {} as JSON: {}", name, e));

        // Verify it has the expected JSON Schema structure:
        // a valid JSON Schema must have either $schema or a type/title
        let has_title = schema.get("title").is_some();
        let has_type = schema.get("type").is_some();
        assert!(
            has_title || has_type,
            "Schema {} has no title or type field",
            name
        );
    }

    eprintln!(
        "✅ Generated and validated {} JSON Schema files",
        schema_names.len()
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
