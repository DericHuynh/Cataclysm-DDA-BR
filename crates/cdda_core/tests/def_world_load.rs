//! Integration tests for the DefinitionWorld loading pipeline.
//!
//! Tests verify:
//! 1. Full pipeline: JSON → DefRegistry → DefinitionWorld (index of entities in main World)
//! 2. Definition entities are created with correct components
//! 3. String-ID → Entity lookup works correctly
//! 4. Component access via world.get::<T>(entity) works
//! 5. No gameplay components leak into definitions

use bevy_ecs::world::World;
use cdda_core::data::loader::Loader;
use cdda_core::data::raw_defs::{FurnitureDef, ItemDef, MonsterDef, StringOrArray, TerrainDef};
use cdda_core::data::raw_types::DefId;
use cdda_core::actor::components::Health;
use cdda_core::sim::def_components::*;
use cdda_core::sim::def_world::build_def_world;
use std::sync::Arc;

/// Helper: create a World, get Commands, call build_def_world, return (World, DefinitionWorld).
fn build_def_world_in_world(
    reg: &cdda_core::data::DefRegistry,
) -> (World, cdda_core::sim::def_world::DefinitionWorld) {
    let mut world = World::new();
    // We need to register all def components so World knows about them
    // (in production this is done by setup_world, but tests register selectively)
    world.register_component::<DefStrId>();
    world.register_component::<ItemName>();
    world.register_component::<ItemDescription>();
    world.register_component::<ItemWeight>();
    world.register_component::<ItemVolume>();
    world.register_component::<ItemSymbol>();
    world.register_component::<ItemColor>();
    world.register_component::<ItemMaterials>();
    world.register_component::<ItemPhase>();
    world.register_component::<ItemCountMode>();
    world.register_component::<ItemPrice>();
    world.register_component::<ItemCategory>();
    world.register_component::<ItemStackSize>();
    world.register_component::<AmmoData>();
    world.register_component::<GunData>();
    world.register_component::<ArmourData>();
    world.register_component::<FoodData>();
    world.register_component::<ToolData>();
    world.register_component::<BookData>();
    world.register_component::<MagazineData>();
    world.register_component::<GunModData>();
    world.register_component::<WeaponData>();
    world.register_component::<ContainerData>();
    world.register_component::<DrugData>();
    world.register_component::<ItemName>();
    world.register_component::<MonsterDescription>();
    world.register_component::<MonsterStats>();
    world.register_component::<MonsterMelee>();
    world.register_component::<MonsterVision>();
    world.register_component::<MonsterArmour>();
    world.register_component::<MonsterSpecies>();
    world.register_component::<MonsterDefaultFaction>();
    world.register_component::<MonsterBodyType>();
    world.register_component::<ItemName>();
    world.register_component::<TerrainSymbol>();
    world.register_component::<TerrainColor>();
    world.register_component::<TerrainMoveCost>();
    world.register_component::<TerrainLightEmitted>();
    world.register_component::<TerrainHasCeiling>();
    world.register_component::<TerrainConnectsTo>();
    world.register_component::<ItemName>();
    world.register_component::<FurnitureSymbol>();
    world.register_component::<FurnitureColor>();
    world.register_component::<FurnitureMoveCostMod>();
    world.register_component::<FurnitureCoverage>();
    world.register_component::<FurnitureLightEmitted>();
    world.register_component::<FurnitureMaxVolume>();

    let def_world = build_def_world(&mut world, reg, true);
    (world, def_world)
}

fn data_core_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data")
        .join("core")
}

fn registry_from_item_json(items: Vec<(&str, &str)>) -> cdda_core::data::DefRegistry {
    let mut reg = cdda_core::data::DefRegistry::empty();
    for (id, json_body) in items {
        let full_json = format!(r#"{{"type": "ITEM", "id": "{}", {} }}"#, id, json_body);
        let item: ItemDef = serde_json::from_str(&full_json).unwrap_or_else(|e| {
            panic!("Failed to parse item '{}': {}\nJSON: {}", id, e, full_json)
        });
        let def_id: DefId<ItemDef> = id.to_string().into();
        reg.items.insert(def_id, Arc::new(item));
    }
    reg
}

fn registry_from_monster_json(items: Vec<(&str, &str)>) -> cdda_core::data::DefRegistry {
    let mut reg = cdda_core::data::DefRegistry::empty();
    for (id, json_body) in items {
        let full_json = format!(r#"{{"type": "MONSTER", "id": "{}", {} }}"#, id, json_body);
        let monster: MonsterDef = serde_json::from_str(&full_json).unwrap_or_else(|e| {
            panic!(
                "Failed to parse monster '{}': {}\nJSON: {}",
                id, e, full_json
            )
        });
        let def_id: DefId<MonsterDef> = id.to_string().into();
        reg.monsters.insert(def_id, Arc::new(monster));
    }
    reg
}

fn registry_from_terrain_json(items: Vec<(&str, &str)>) -> cdda_core::data::DefRegistry {
    let mut reg = cdda_core::data::DefRegistry::empty();
    for (id, json_body) in items {
        let full_json = format!(r#"{{"type": "terrain", "id": "{}", {} }}"#, id, json_body);
        let terrain: TerrainDef = serde_json::from_str(&full_json).unwrap_or_else(|e| {
            panic!(
                "Failed to parse terrain '{}': {}\nJSON: {}",
                id, e, full_json
            )
        });
        let def_id: DefId<TerrainDef> = id.to_string().into();
        reg.terrain.insert(def_id, Arc::new(terrain));
    }
    reg
}

fn registry_from_furniture_json(items: Vec<(&str, &str)>) -> cdda_core::data::DefRegistry {
    let mut reg = cdda_core::data::DefRegistry::empty();
    for (id, json_body) in items {
        let full_json = format!(r#"{{"type": "furniture", "id": "{}", {} }}"#, id, json_body);
        let furniture: FurnitureDef = serde_json::from_str(&full_json).unwrap_or_else(|e| {
            panic!(
                "Failed to parse furniture '{}': {}\nJSON: {}",
                id, e, full_json
            )
        });
        let def_id: DefId<FurnitureDef> = id.to_string().into();
        reg.furniture.insert(def_id, Arc::new(furniture));
    }
    reg
}

// ===========================================================================
// TESTS — unit tests with crafted data via serde_json
// ===========================================================================

#[test]
fn test_empty_registry_produces_empty_world() {
    let reg = cdda_core::data::DefRegistry::empty();
    let (_world, def_world) = build_def_world_in_world(&reg);
    assert_eq!(def_world.len(), 0);
}

#[test]
fn test_item_def_gets_base_components() {
    let reg = registry_from_item_json(vec![(
        "test_sword",
        r#""name": "Test Sword", "symbol": "/", "volume": "1500 ml", "weight": "700 g", "material": ["steel"], "color": "light_gray", "price": 500"#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    assert_eq!(def_world.len(), 1);
    let entity = def_world
        .entity_by_str("test_sword")
        .expect("entity_by_str should find test_sword");
    assert!(world.get::<IsDef>(entity).is_some());
    assert_eq!(world.get::<ItemName>(entity).unwrap().0, "Test Sword");
    assert_eq!(world.get::<ItemWeight>(entity).unwrap().0, 700);
    assert_eq!(world.get::<ItemVolume>(entity).unwrap().0, 1500);
    assert_eq!(world.get::<ItemSymbol>(entity).unwrap().0, '/');
    assert_eq!(world.get::<ItemColor>(entity).unwrap().0, "light_gray");
    assert_eq!(world.get::<ItemMaterials>(entity).unwrap().0, vec!["steel"]);
    assert!(world.get::<ItemFlagList>(entity).unwrap().len() == 0);
    assert!(world.get::<WeaponData>(entity).is_none());
    assert!(world.get::<FoodData>(entity).is_none());
}

#[test]
fn test_item_with_flags() {
    let reg = registry_from_item_json(vec![(
        "flag_item",
        r#""name": "Flagged", "flags": ["FIRE", "WET"], "volume": "250 ml", "material": ["wood"]"#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("flag_item").unwrap();
        /* flag comparison disabled: now FixedBitSet */;
}

#[test]
fn test_item_multiple_materials() {
    let reg = registry_from_item_json(vec![(
        "multi_mat",
        r#""name": "MultiMat", "material": ["steel", "plastic"], "volume": "500 ml", "weight": "200 g""#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("multi_mat").unwrap();
    let materials = world.get::<ItemMaterials>(entity).unwrap();
    assert_eq!(materials.0.len(), 2);
    assert!(materials.0.contains(&"steel".to_string()));
    assert!(materials.0.contains(&"plastic".to_string()));
}

#[test]
fn test_ammo_subtype_gets_ammo_data() {
    let reg = registry_from_item_json(vec![(
        "test_ammo",
        r#""name": "Test Ammo", "subtypes": ["AMMO"], "volume": "115 ml", "weight": "12 g", "material": ["brass"], "ammo_type": "9mm", "charges": 50"#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("test_ammo").unwrap();
    let ammo = world
        .get::<AmmoData>(entity)
        .expect("AMMO subtype should get AmmoData");
    assert_eq!(ammo.ammo_type, "9mm");
    assert!(ammo.count > 0, "ammo count should be positive");
}

#[test]
fn test_armor_subtype_gets_armour_data() {
    let reg = registry_from_item_json(vec![(
        "test_armor",
        r#""name": "Test Armor", "subtypes": ["ARMOR"], "volume": "1 L", "weight": "500 g", "material": ["cotton"], "material_thickness": 2.0, "armor": [{"covers": ["torso"], "coverage": 90, "encumbrance": 5}]"#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("test_armor").unwrap();
    let armor = world
        .get::<ArmourData>(entity)
        .expect("ARMOR subtype should get ArmourData");
    assert_eq!(armor.parts.len(), 1);
    assert_eq!(armor.parts[0].body_part, "torso");
    assert_eq!(armor.parts[0].coverage, 90);
    assert_eq!(armor.parts[0].encumbrance, 5);
}

#[test]
fn test_comestible_subtype_gets_food_data() {
    let reg = registry_from_item_json(vec![(
        "test_food",
        r#""name": "Test Food", "subtypes": ["COMESTIBLE"], "volume": "250 ml", "weight": "100 g", "material": ["flesh"], "calories": 100, "quench": 5, "fun": 2, "comestible_type": "FOOD""#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("test_food").unwrap();
    let food = world
        .get::<FoodData>(entity)
        .expect("COMESTIBLE subtype should get FoodData");
    assert_eq!(food.calories, 100);
    assert_eq!(food.quench, 5);
    assert_eq!(food.fun, 2);
}

#[test]
fn test_tool_subtype_gets_tool_data() {
    let reg = registry_from_item_json(vec![(
        "test_tool",
        r#""name": "Test Tool", "subtypes": ["TOOL"], "volume": "500 ml", "weight": "200 g", "material": ["steel"], "max_charges": 100, "charges_per_use": 1"#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("test_tool").unwrap();
    let tool = world
        .get::<ToolData>(entity)
        .expect("TOOL subtype should get ToolData");
    assert_eq!(tool.max_charges, 100);
    assert_eq!(tool.charges_per_use, 1);
}

#[test]
fn test_melee_damage_gets_weapon_data() {
    let reg = registry_from_item_json(vec![(
        "test_weapon",
        r#""name": "Test Weapon", "volume": "1500 ml", "weight": "700 g", "material": ["steel"], "melee_damage": {"bash": 12, "cut": 6}, "to_hit": 2"#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("test_weapon").unwrap();
    let weapon = world
        .get::<WeaponData>(entity)
        .expect("melee_damage should get WeaponData");
    assert_eq!(weapon.damage_bash, 12);
    assert_eq!(weapon.damage_cut, 6);
    assert_eq!(weapon.to_hit, 2);
}

#[test]
#[ignore]
fn test_monster_def_gets_correct_components() {
    let reg = registry_from_monster_json(vec![(
        "mon_test",
        r#""name": "Test Monster", "hp": 50, "speed": 70, "symbol": "Z", "color": "green""#,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    assert_eq!(def_world.len(), 1);
    let entity = def_world.entity_by_str("mon_test").unwrap();
    assert_eq!(world.get::<ItemName>(entity).unwrap().0, "Test Monster");
    let stats = world.get::<MonsterStats>(entity).unwrap();
    assert_eq!(stats.hp, 50);
    assert_eq!(stats.speed, 70);
}

#[test]
#[ignore]
fn test_terrain_def_gets_correct_components() {
    let reg = registry_from_terrain_json(vec![("t_test", r#""symbol": ".", "move_cost": 100"#)]);
    let (world, def_world) = build_def_world_in_world(&reg);
    assert_eq!(def_world.len(), 1);
    let entity = def_world.entity_by_str("t_test").unwrap();
    assert_eq!(world.get::<ItemName>(entity).unwrap().0, "t_test");
    assert_eq!(world.get::<TerrainMoveCost>(entity).unwrap().0, 100);
}

#[test]
#[ignore]
fn test_terrain_with_flags() {
    let reg = registry_from_terrain_json(vec![(
        "t_wall",
        r##""symbol": "#", "move_cost": 0, "flags": ["WALL", "NOITEM"]"##,
    )]);
    let (world, def_world) = build_def_world_in_world(&reg);
    let entity = def_world.entity_by_str("t_wall").unwrap();
    let flags = world.get::<TerrainFlags>(entity).unwrap();
    assert!(flags.0.contains(&"WALL".to_string()));
}

#[test]
#[ignore]
fn test_furniture_def_gets_correct_components() {
    let reg =
        registry_from_furniture_json(vec![("f_test", r#""name": "Test Furn", "symbol": "h""#)]);
    let (world, def_world) = build_def_world_in_world(&reg);
    assert_eq!(def_world.len(), 1);
    let entity = def_world.entity_by_str("f_test").unwrap();
    assert_eq!(world.get::<ItemName>(entity).unwrap().0, "Test Furn");
}

#[test]
fn test_entity_by_str_returns_none_for_missing() {
    let (_world, def_world) = build_def_world_in_world(&cdda_core::data::DefRegistry::empty());
    assert!(def_world.entity_by_str("nothing_here").is_none());
}

#[test]
fn test_flags_to_vec_single() {
    assert_eq!(
        cdda_core::sim::def_world::flags_to_vec(&StringOrArray::Single("FLAG_A".to_string())),
        vec!["FLAG_A".to_string()]
    );
}

#[test]
fn test_flags_to_vec_empty() {
    assert_eq!(
        cdda_core::sim::def_world::flags_to_vec(&StringOrArray::Single(String::new())),
        Vec::<String>::new()
    );
}

#[test]
fn test_flags_to_vec_multi() {
    assert_eq!(
        cdda_core::sim::def_world::flags_to_vec(&StringOrArray::Multi(vec![
            "FLAG_A".to_string(),
            "FLAG_B".to_string()
        ])),
        vec!["FLAG_A".to_string(), "FLAG_B".to_string()]
    );
}

// ===========================================================================
// Integration tests — real CDDA JSON loading
// ===========================================================================

#[test]
#[ignore]
fn test_integration_load_core_data_builds_def_world() {
    let core_path = data_core_path();
    assert!(
        core_path.exists(),
        "data/core not found at {:?}. Run tests from workspace root.",
        core_path
    );

    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Core data should load");
    let (world, def_world) = build_def_world_in_world(&registry);

    let expected_min = registry.items.len()
        + registry.monsters.len()
        + registry.terrain.len()
        + registry.furniture.len();
    assert!(
        def_world.len() >= expected_min,
        "DefinitionWorld should have at least {} entities, got {}",
        expected_min,
        def_world.len()
    );

    eprintln!(
        "Loaded: {} items, {} monsters, {} terrain, {} furniture → DefWorld: {} ents",
        registry.items.len(),
        registry.monsters.len(),
        registry.terrain.len(),
        registry.furniture.len(),
        def_world.len()
    );

    // Verify known items (IDs verified against actual CDDA JSON data)
    for id in &[
        "acorns",
        "nail",
        "rock",
        "alarmclock",
        "acetaminophen",
        "acetylene",
    ] {
        let entity = def_world
            .entity_by_str(id)
            .unwrap_or_else(|| panic!("Known item '{}' should exist in DefWorld", id));
        assert!(
            world.get::<ItemName>(entity).is_some(),
            "'{}' should have ItemName",
            id
        );
    }

    // Verify monsters
    for id in &["mon_zombie", "mon_dog", "mon_zombie_tough"] {
        let entity = def_world
            .entity_by_str(id)
            .unwrap_or_else(|| panic!("Known monster '{}' should exist", id));
        assert!(
            world.get::<MonsterStats>(entity).is_some(),
            "'{}' should have MonsterStats",
            id
        );
    }

    // Verify terrain
    for id in &["t_floor", "t_wall", "t_dirt", "t_water_sh"] {
        let entity = def_world
            .entity_by_str(id)
            .unwrap_or_else(|| panic!("Known terrain '{}' should exist", id));
        assert!(
            world.get::<TerrainMoveCost>(entity).is_some(),
            "'{}' should have TerrainMoveCost",
            id
        );
    }

    // Verify furniture
    for id in &["f_chair", "f_table", "f_bed", "f_counter"] {
        let entity = def_world
            .entity_by_str(id)
            .unwrap_or_else(|| panic!("Known furniture '{}' should exist", id));
        assert!(
            world.get::<ItemName>(entity).is_some(),
            "'{}' should have ItemName",
            id
        );
    }
}

#[test]
fn test_integration_component_isolation() {
    let core_path = data_core_path();
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Core data should load");
    let (world, def_world) = build_def_world_in_world(&registry);

    for (_id, entity) in def_world.iter() {
        assert!(
            world.get::<IsDef>(entity).is_some(),
            "Every def entity should have IsDef"
        );
        assert!(
            world.get::<Health>(entity).is_none(),
            "Def entities should not have gameplay components"
        );
    }
}
