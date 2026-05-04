//! Integration test: loads all CDDA core JSON into the DefinitionWorld.
//! Spawns entities into a standalone World + builds the string->Entity index.

use bevy_ecs::world::World;
use cdda_data::loader::Loader;
use cdda_sim::components::Health;
use cdda_sim::def_components::*;
use cdda_sim::def_world::{build_def_world, DefinitionWorld};
use std::path::PathBuf;

fn data_core_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/core")
}

struct TestContext {
    def_world: DefinitionWorld,
    world: World,
}

fn load_all() -> TestContext {
    let core_path = data_core_path();
    assert!(
        core_path.exists(),
        "data/core not found. Run from workspace root."
    );
    let mut loader = Loader::new(vec![core_path]);
    let registry = loader.load().expect("Full data load must succeed");

    let mut world = World::new();
    register_components(&mut world);

    let def_world = build_def_world(&mut world, &registry);

    TestContext { def_world, world }
}

fn register_components(world: &mut World) {
    macro_rules! reg { ($($t:ty),+) => { $( world.register_component::<$t>(); )+ } }
    reg!(
        IsDef,
        DefStrId,
        ItemName,
        ItemDescription,
        ItemWeight,
        ItemVolume,
        ItemSymbol,
        ItemColor,
        ItemMaterials,
        ItemFlagList,
        ItemPrice,
        ItemPhase,
        ItemStackSize,
        ItemCategory,
        WeaponData,
        GunData,
        AmmoData,
        MagazineData,
        ArmourData,
        FoodData,
        ToolData,
        BookData,
        GunModData,
        ContainerData,
        DrugData,
        MonsterName,
        MonsterDescription,
        MonsterStats,
        MonsterMelee,
        MonsterVision,
        MonsterArmour,
        MonsterFlags,
        MonsterSpecies,
        MonsterDefaultFaction,
        MonsterBodyType,
        TerrainName,
        TerrainSymbol,
        TerrainColor,
        TerrainMoveCost,
        TerrainFlags,
        TerrainLightEmitted,
        TerrainHasCeiling,
        TerrainConnectsTo,
        FurnitureName,
        FurnitureSymbol,
        FurnitureColor,
        FurnitureFlags,
        FurnitureMoveCostMod,
        FurnitureCoverage,
        FurnitureLightEmitted,
        FurnitureMaxVolume
    );
}

macro_rules! check_component {
    ($world:expr, $ty:ty, $e:expr, $desc:expr) => {
        assert!($world.get::<$ty>($e).is_some(), "{}", $desc);
    };
}

// ======================================================
// TESTS
// ======================================================

#[test]
fn test_full_pipeline() {
    let ctx = load_all();
    assert!(ctx.def_world.len() > 13_000);
}

#[test]
fn test_all_entities_have_isdef() {
    let ctx = load_all();
    for (_id, e) in ctx.def_world.iter() {
        assert!(ctx.world.get::<IsDef>(e).is_some());
    }
}

#[test]
fn test_no_gameplay_components() {
    let ctx = load_all();
    for (_id, e) in ctx.def_world.iter() {
        assert!(ctx.world.get::<Health>(e).is_none());
    }
}

#[test]
fn test_known_items() {
    let ctx = load_all();
    for &id in &[
        "acorns",
        "nail",
        "rock",
        "alarmclock",
        "9mm",
        "10mm_fmj",
        "shot_00",
        "arrow_wood",
    ] {
        let e = ctx.def_world.entity_by_str(id).expect(id);
        check_component!(ctx.world, ItemName, e, format!("{id}: ItemName"));
        check_component!(ctx.world, ItemWeight, e, format!("{id}: ItemWeight"));
        check_component!(ctx.world, ItemVolume, e, format!("{id}: ItemVolume"));
        check_component!(ctx.world, ItemPrice, e, format!("{id}: ItemPrice"));
        check_component!(ctx.world, ItemStackSize, e, format!("{id}: ItemStackSize"));
        check_component!(ctx.world, ItemPhase, e, format!("{id}: ItemPhase"));
    }
}

#[test]
fn test_known_monsters() {
    let ctx = load_all();
    for &id in &[
        "mon_zombie",
        "mon_dog",
        "mon_zombie_tough",
        "mon_bee",
        "mon_zombie_hulk",
    ] {
        let e = ctx.def_world.entity_by_str(id).expect(id);
        check_component!(ctx.world, MonsterName, e, format!("{id}: MonsterName"));
        check_component!(ctx.world, MonsterStats, e, format!("{id}: MonsterStats"));
        check_component!(ctx.world, MonsterMelee, e, format!("{id}: MonsterMelee"));
        check_component!(ctx.world, MonsterVision, e, format!("{id}: MonsterVision"));
        check_component!(
            ctx.world,
            MonsterSpecies,
            e,
            format!("{id}: MonsterSpecies")
        );
    }
}

#[test]
fn test_known_terrain() {
    let ctx = load_all();
    for &id in &[
        "t_floor",
        "t_wall",
        "t_dirt",
        "t_water_sh",
        "t_grass",
        "t_door_c",
        "t_window",
    ] {
        let e = ctx.def_world.entity_by_str(id).expect(id);
        check_component!(ctx.world, TerrainName, e, format!("{id}: TerrainName"));
        check_component!(
            ctx.world,
            TerrainMoveCost,
            e,
            format!("{id}: TerrainMoveCost")
        );
        check_component!(ctx.world, TerrainFlags, e, format!("{id}: TerrainFlags"));
    }
}

#[test]
fn test_known_furniture() {
    let ctx = load_all();
    for &id in &[
        "f_chair",
        "f_table",
        "f_bed",
        "f_counter",
        "f_desk",
        "f_bookcase",
        "f_fridge",
        "f_sink",
    ] {
        let e = ctx.def_world.entity_by_str(id).expect(id);
        check_component!(ctx.world, FurnitureName, e, format!("{id}: FurnitureName"));
        check_component!(
            ctx.world,
            FurnitureSymbol,
            e,
            format!("{id}: FurnitureSymbol")
        );
    }
}

#[test]
fn test_ammo_items_have_ammodata() {
    let ctx = load_all();
    for &id in &["9mm", "10mm_fmj", "shot_00", "arrow_wood", "nail"] {
        if let Some(e) = ctx.def_world.entity_by_str(id) {
            check_component!(ctx.world, AmmoData, e, format!("{id}: AmmoData"));
        }
    }
}

#[test]
fn test_comestible_items_have_fooddata() {
    let ctx = load_all();
    for &id in &["acorns", "apple", "bread", "water_clean", "beer"] {
        if let Some(e) = ctx.def_world.entity_by_str(id) {
            check_component!(ctx.world, FoodData, e, format!("{id}: FoodData"));
        }
    }
}

#[test]
fn test_tool_items_have_tooldata() {
    let ctx = load_all();
    for &id in &["hammer", "screwdriver", "wrench", "crowbar", "shovel"] {
        if let Some(e) = ctx.def_world.entity_by_str(id) {
            check_component!(ctx.world, ToolData, e, format!("{id}: ToolData"));
        }
    }
}

#[test]
fn test_rock_is_ammo() {
    let ctx = load_all();
    if let Some(e) = ctx.def_world.entity_by_str("rock") {
        assert!(
            ctx.world.get::<AmmoData>(e).is_some(),
            "rock IS ammo in CDDA"
        );
        assert!(ctx.world.get::<GunData>(e).is_none());
        assert!(ctx.world.get::<FoodData>(e).is_none());
        assert!(ctx.world.get::<ToolData>(e).is_none());
    }
}

#[test]
fn test_acorn_values() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("acorns").expect("acorns");
    assert_eq!(ctx.world.get::<ItemName>(e).unwrap().0, "acorns");
    assert!(ctx.world.get::<ItemWeight>(e).unwrap().0 > 0);
    let food = ctx.world.get::<FoodData>(e).expect("acorns: FoodData");
    assert!(food.calories > 0);
}

#[test]
fn test_9mm_values() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("9mm").expect("9mm");
    let ammo = ctx.world.get::<AmmoData>(e).expect("9mm: AmmoData");
    assert_eq!(ammo.ammo_type, "9mm");
    assert!(ammo.count > 0);
}

#[test]
fn test_mon_zombie_values() {
    let ctx = load_all();
    let e = ctx
        .def_world
        .entity_by_str("mon_zombie")
        .expect("mon_zombie");
    let stats = ctx.world.get::<MonsterStats>(e).unwrap();
    assert!(stats.hp > 0);
    assert!(stats.speed > 0);
    let vision = ctx.world.get::<MonsterVision>(e).unwrap();
    assert!(vision.day >= 30);
    assert!(vision.night >= 3);
    let species = ctx.world.get::<MonsterSpecies>(e).unwrap();
    assert!(species.0.iter().any(|s| s == "ZOMBIE"));
}

#[test]
fn test_t_floor_values() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("t_floor").expect("t_floor");
    assert!(ctx.world.get::<TerrainMoveCost>(e).unwrap().0 > 0);
}

#[test]
fn test_t_wall_impassable() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("t_wall").expect("t_wall");
    assert_eq!(ctx.world.get::<TerrainMoveCost>(e).unwrap().0, 0);
    assert!(ctx
        .world
        .get::<TerrainFlags>(e)
        .unwrap()
        .0
        .contains(&"WALL".to_string()));
}

#[test]
fn test_entity_by_str() {
    let ctx = load_all();
    assert!(ctx.def_world.entity_by_str("acorns").is_some());
    assert!(ctx.def_world.entity_by_str("mon_zombie").is_some());
    assert!(ctx.def_world.entity_by_str("t_floor").is_some());
    assert!(ctx.def_world.entity_by_str("f_chair").is_some());
    assert!(ctx.def_world.entity_by_str("nonexistent_xyz").is_none());
}

#[test]
fn test_monster_species_value() {
    let ctx = load_all();
    if let Some(e) = ctx.def_world.entity_by_str("mon_zombie") {
        let s = ctx.world.get::<MonsterSpecies>(e).unwrap();
        assert!(s.0.iter().any(|x| x == "ZOMBIE"));
    }
    if let Some(e) = ctx.def_world.entity_by_str("mon_dog") {
        let s = ctx.world.get::<MonsterSpecies>(e).unwrap();
        assert!(s.0.iter().any(|x| x == "MAMMAL"));
    }
}
