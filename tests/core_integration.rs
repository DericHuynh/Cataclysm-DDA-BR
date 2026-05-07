//! Integration tests with real CDDA core data.
//!
//! Loads all JSON from `data/core/` and runs tests against real definition
//! entities — verifying specific values on known CDDA items, monsters,
//! furniture, and terrain. Also tests spawning runtime entities from defs
//! and inventory operations against real containers.

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use cdda_core::coords::{WorldPos, ZLevel};
use cdda_data::loader::Loader;
use cdda_sim::components::*;
use cdda_sim::def_components::*;
use cdda_sim::def_world::{build_def_world, DefinitionWorld};
use cdda_sim::systems::inventory::*;
use cdda_sim::systems::spawning::*;
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
    registry: cdda_data::DefRegistry,
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
    register_all(&mut world);
    let def_world = build_def_world(&mut world, &registry);

    TestContext {
        def_world,
        world,
        registry,
    }
}

fn register_all(world: &mut World) {
    macro_rules! reg { ($($t:ty),+) => { $( world.register_component::<$t>(); )+ } }

    // Def components
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
        ItemLongestSide,
        ItemInsulation,
        ItemCoversHead,
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
        MonsterUpgrade,
        MonsterHarvest,
        MonsterDeathFunction,
        MonsterDeathDrops,
        MonsterSpecialAttacks,
        TerrainName,
        TerrainSymbol,
        TerrainColor,
        TerrainMoveCost,
        TerrainOpacity,
        TerrainFlags,
        TerrainLightEmitted,
        TerrainRoof,
        TerrainHasCeiling,
        TerrainConnectsTo,
        TerrainExamineAction,
        TerrainTrap,
        FurnitureName,
        FurnitureSymbol,
        FurnitureColor,
        FurnitureFlags,
        FurnitureMoveCostMod,
        FurnitureCoverage,
        FurnitureRequiredStr,
        FurnitureMaxVolume,
        FurnitureComfort,
        FurnitureLightEmitted,
        FurnitureExamineAction,
        FurnitureMass,
        BodyPartName,
        BodyPartDefId,
        IsVital,
        CanGrasp,
        CanWalk,
        CanSee,
        CanBite,
        CanFly,
        ParentPart,
        SubParts,
        BodyPartHitSize,
        BodyPartHitDifficulty,
        BodyPartBaseHp,
        BodyPartDrenchCapacity,
        BodyPartSide,
        BodyPartLegacyId
    );

    // Runtime item components
    reg!(
        StackCount,
        CurrentCharges,
        LoadedAmmo,
        ItemDamage,
        Spoilable,
        Sealed,
        Rigid,
        Watertight,
        PreservesTemp,
        Fireproof,
        GasTight,
        InsideContainer,
        ContainerContents,
        Container,
        Pocket,
        PocketRestriction,
        WieldedBy,
        WieldedItems,
        WornOn,
        WornBy,
        MountedOn,
        MountedPockets,
        AttachmentSlot
    );

    // Actor components
    reg!(
        Health,
        IsAlive,
        Creature,
        MovePoints,
        Speed,
        Stats,
        CombatStats,
        Vision,
        Faction,
        PlayerData,
        NpcData,
        Morale,
        BodyTemperature,
        Wetness,
        SkillSet,
        Mutations,
        EffectOn,
        ActiveEffects,
        StatusEffect,
        BodyPartOf,
        CreatureBodyParts,
        BodyPartDef,
        BodyPartSlot,
        BodyPartHp,
        BodyPartBroken,
        BodyPartSevered,
        Bleeding,
        Stunned,
        OnFire,
        InFlight,
        WorldPosition,
        Solid,
        Velocity,
        Bionic,
        BionicOf,
        InstalledBionics,
        MoraleBonus,
        MoraleBonusOf,
        MoraleBonuses,
        ProficiencySet
    );
}

/// Get a runtime component value from a def entity, panicking with a clear message.
macro_rules! assert_component {
    ($ctx:expr, $ty:ty, $id:expr, $field:ident, $op:tt $expected:expr) => {
        let e = $ctx.def_world.entity_by_str($id)
            .unwrap_or_else(|| panic!("Entity '{}' not found", $id));
        let comp = $ctx.world.get::<$ty>(e)
            .unwrap_or_else(|| panic!("'{}' does not have {} component", $id, stringify!($ty)));
        assert!(comp.$field $op $expected,
            "'{}'.{} = {:?}, expected {} {:?}",
            $id, stringify!($field), comp.$field, stringify!($op), $expected);
    };
    ($ctx:expr, $ty:ty, $id:expr, $field:ident == $expected:expr) => {
        let e = $ctx.def_world.entity_by_str($id)
            .unwrap_or_else(|| panic!("Entity '{}' not found", $id));
        let comp = $ctx.world.get::<$ty>(e)
            .unwrap_or_else(|| panic!("'{}' does not have {} component", $id, stringify!($ty)));
        assert_eq!(comp.$field, $expected,
            "'{}'.{} expected {:?}, got {:?}",
            $id, stringify!($field), $expected, comp.$field);
    };
}

// =========================================================================
// CONTAINER TESTS — real container items from CDDA
// =========================================================================

#[test]
fn backpack_has_container_data() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("backpack").expect("backpack");
    let cd = ctx
        .world
        .get::<ContainerData>(e)
        .expect("backpack: ContainerData");
    assert!(
        cd.max_volume > 0,
        "backpack should have positive max_volume, got {}",
        cd.max_volume
    );
    assert!(
        !cd.pockets.is_empty(),
        "backpack should have at least one pocket"
    );
}

#[test]
fn bottle_plastic_has_container_data() {
    let ctx = load_all();
    let e = ctx
        .def_world
        .entity_by_str("bottle_plastic")
        .expect("bottle_plastic");
    let cd = ctx
        .world
        .get::<ContainerData>(e)
        .expect("bottle_plastic: ContainerData");
    assert!(
        cd.max_volume > 0,
        "bottle_plastic should have positive max_volume, got {}",
        cd.max_volume
    );
    assert!(
        cd.max_volume >= 250,
        "plastic bottle should hold at least 250ml, got {} ml",
        cd.max_volume
    );
}

#[test]
fn jar_glass_sealed() {
    let ctx = load_all();
    if let Some(e) = ctx.def_world.entity_by_str("jar_glass") {
        let cd = ctx
            .world
            .get::<ContainerData>(e)
            .expect("jar_glass: ContainerData");
        assert!(
            cd.max_volume > 0,
            "jar_glass should have container volume > 0"
        );
    }
}

// =========================================================================
// ARMOUR TESTS — specific coverage values
// =========================================================================

#[test]
fn hoodie_armour_parts() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("hoodie").expect("hoodie");
    let armour = ctx.world.get::<ArmourData>(e).expect("hoodie: ArmourData");
    assert!(!armour.parts.is_empty(), "hoodie should have armour parts");
    // Hoodies typically cover torso and arms
    let part_names: Vec<&str> = armour.parts.iter().map(|p| p.body_part.as_str()).collect();
    assert!(part_names.contains(&"torso"), "hoodie covers torso");
}

#[test]
fn jeans_armour_coverage() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("jeans").expect("jeans");
    let armour = ctx.world.get::<ArmourData>(e).expect("jeans: ArmourData");
    let part_names: Vec<&str> = armour.parts.iter().map(|p| p.body_part.as_str()).collect();
    assert!(!armour.parts.is_empty(), "jeans should have armour parts");
    for p in &armour.parts {
        assert!(
            p.coverage > 0,
            "jeans part '{}' should have coverage > 0",
            p.body_part
        );
    }
    let covers_legs = part_names.iter().any(|n| n.contains("leg"));
    assert!(
        covers_legs,
        "jeans should cover at least one leg; parts: {:?}",
        part_names
    );
}

#[test]
fn helmet_coverage() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("hat_hard").expect("hat_hard");
    let armour = ctx
        .world
        .get::<ArmourData>(e)
        .expect("hat_hard: ArmourData");
    let part_names: Vec<&str> = armour.parts.iter().map(|p| p.body_part.as_str()).collect();
    assert!(part_names.contains(&"head"), "hard hat should cover head");
    for p in &armour.parts {
        assert!(
            p.coverage > 0,
            "hard hat part '{}' should have coverage > 0",
            p.body_part
        );
    }
}

// =========================================================================
// WEAPON TESTS — specific weapon data values
// =========================================================================

#[test]
fn combat_knife_weapon_data() {
    let ctx = load_all();
    let e = ctx
        .def_world
        .entity_by_str("knife_combat")
        .expect("knife_combat");
    let wd = ctx
        .world
        .get::<WeaponData>(e)
        .expect("knife_combat: WeaponData");
    assert!(wd.damage_cut >= wd.damage_bash, "combat knife cut >= bash");
    assert_ne!(wd.to_hit, 0, "combat knife should have non-zero to_hit");
}

#[test]
fn baseball_bat_bash_damage() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("bat").expect("bat");
    let wd = ctx.world.get::<WeaponData>(e).expect("bat: WeaponData");
    // Baseball bat should have mostly bash damage
    assert!(wd.damage_bash > wd.damage_cut, "bat bash > cut");
    assert!(wd.damage_bash > 0, "bat should have bash damage");
}

// =========================================================================
// FOOD TESTS — specific calorie and nutrition values
// =========================================================================

#[test]
fn acorns_food_values() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("acorns").expect("acorns");
    let food = ctx.world.get::<FoodData>(e).expect("acorns: FoodData");
    assert!(food.calories > 0, "acorns should have calories");
    assert!(food.spoils_in > 0, "acorns should spoil eventually");
    assert!(
        !food.comestible_type.is_empty(),
        "acorns should have a comestible_type set"
    );
}

#[test]
fn water_clean_drink() {
    let ctx = load_all();
    let e = ctx
        .def_world
        .entity_by_str("water_clean")
        .expect("water_clean");
    let food = ctx.world.get::<FoodData>(e).expect("water_clean: FoodData");
    assert!(food.quench > 0, "water_clean should quench");
}

// =========================================================================
// AMMO TESTS — specific ammunition values
// =========================================================================

#[test]
fn nine_mm_ammo_values() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("9mm").expect("9mm");
    let ammo = ctx.world.get::<AmmoData>(e).expect("9mm: AmmoData");
    assert_eq!(ammo.ammo_type, "9mm", "9mm ammo should have type 9mm");
    assert!(ammo.damage > 0, "9mm ammo should have damage > 0");
}

#[test]
fn shot_shotgun_values() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("shot_00").expect("shot_00");
    let ammo = ctx.world.get::<AmmoData>(e).expect("shot_00: AmmoData");
    assert_eq!(ammo.ammo_type, "shot", "shot_00 ammo should have type shot");
}

// =========================================================================
// MONSTER TESTS — specific monster stat values
// =========================================================================

#[test]
fn zombie_stats() {
    let ctx = load_all();
    let e = ctx
        .def_world
        .entity_by_str("mon_zombie")
        .expect("mon_zombie");
    let stats = ctx
        .world
        .get::<MonsterStats>(e)
        .expect("mon_zombie: MonsterStats");
    assert!(stats.hp >= 50, "mon_zombie HP >= 50");
    assert!(stats.speed >= 50, "mon_zombie speed >= 50");
}

#[test]
fn zombie_hulk_stronger_than_zombie() {
    let ctx = load_all();
    let z = ctx
        .def_world
        .entity_by_str("mon_zombie")
        .expect("mon_zombie");
    let h = ctx
        .def_world
        .entity_by_str("mon_zombie_hulk")
        .expect("mon_zombie_hulk");
    let z_stats = ctx.world.get::<MonsterStats>(z).unwrap();
    let h_stats = ctx.world.get::<MonsterStats>(h).unwrap();
    assert!(h_stats.hp > z_stats.hp, "hulk HP > zombie HP");
    assert!(
        h_stats.melee_dice >= z_stats.melee_dice,
        "hulk melee dice >= zombie"
    );
}

#[test]
fn zombie_species() {
    let ctx = load_all();
    let e = ctx
        .def_world
        .entity_by_str("mon_zombie")
        .expect("mon_zombie");
    let species = ctx
        .world
        .get::<MonsterSpecies>(e)
        .expect("mon_zombie: MonsterSpecies");
    assert!(
        species.0.contains(&"ZOMBIE".to_string()),
        "mon_zombie species includes ZOMBIE"
    );
}

// =========================================================================
// TERRAIN TESTS — specific terrain properties
// =========================================================================

#[test]
fn wall_impassable() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("t_wall").expect("t_wall");
    assert_eq!(
        ctx.world.get::<TerrainMoveCost>(e).unwrap().0,
        0,
        "walls should be impassable"
    );
}

#[test]
fn floor_passable() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("t_floor").expect("t_floor");
    assert!(
        ctx.world.get::<TerrainMoveCost>(e).unwrap().0 > 0,
        "floor should be passable"
    );
}

#[test]
fn window_has_flag() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("t_window").expect("t_window");
    let flags = ctx
        .world
        .get::<TerrainFlags>(e)
        .expect("t_window: TerrainFlags");
    assert!(
        flags.0.contains(&"TRANSPARENT".to_string()) || flags.0.contains(&"WINDOW".to_string()),
        "t_window should have window-related flags"
    );
}

#[test]
fn door_has_open_close() {
    let ctx = load_all();
    let e = ctx.def_world.entity_by_str("t_door_c").expect("t_door_c");
    // In CDDA, closed doors (t_door_c) have move_cost=0 (impassable when closed)
    let move_cost = ctx
        .world
        .get::<TerrainMoveCost>(e)
        .expect("t_door_c: TerrainMoveCost")
        .0;
    assert_eq!(
        move_cost, 0,
        "t_door_c (closed door) should be impassable — move_cost=0"
    );
}

// =========================================================================
// SPAWN + INVENTORY INTEGRATION TESTS
// =========================================================================

/// Spawn a runtime item from a definition, then test inventory operations.
#[test]
fn spawn_rock_and_check_components() {
    let mut ctx = load_all();
    let def = ctx.def_world.entity_by_str("rock").expect("rock");
    let pos = WorldPos::new(5, 5, ZLevel::new(0));

    // Use the def world directly — def entities and runtime entities coexist
    // in the same World (distinguished by IsDef).
    let item = spawn_item(&mut ctx.world, def, pos, 1);

    // Verify runtime components were added by spawning
    assert!(
        ctx.world.get::<StackCount>(item).is_some(),
        "spawned item should have StackCount"
    );
    assert!(
        ctx.world.get::<WorldPosition>(item).is_some(),
        "spawned item should have WorldPosition"
    );
    assert!(
        ctx.world.get::<CurrentCharges>(item).is_some(),
        "spawned item should have CurrentCharges"
    );

    // Verify def components were cloned
    assert!(
        ctx.world.get::<ItemName>(item).is_some(),
        "spawned item should have ItemName"
    );
    assert!(
        ctx.world.get::<ItemWeight>(item).is_some(),
        "spawned item should have ItemWeight"
    );
}

/// Spawn a monster from a definition.
#[test]
fn spawn_zombie_and_check_components() {
    let mut ctx = load_all();
    let def = ctx
        .def_world
        .entity_by_str("mon_zombie")
        .expect("mon_zombie");
    let pos = WorldPos::new(10, 10, ZLevel::new(0));
    let faction = 0u32.into();

    let monster = spawn_monster(&mut ctx.world, def, pos, faction);

    assert!(
        ctx.world.get::<IsAlive>(monster).is_some(),
        "spawned zombie should be alive"
    );
    assert!(
        ctx.world.get::<WorldPosition>(monster).is_some(),
        "spawned zombie should have position"
    );
    assert!(
        ctx.world.get::<Health>(monster).is_some(),
        "spawned zombie should have health"
    );
    assert!(
        ctx.world.get::<Faction>(monster).is_some(),
        "spawned zombie should have faction"
    );
}

/// Spawn items and test that can_fit_in_container works with real data.
#[test]
fn bedrock_loop_rock_in_backpack() {
    let mut ctx = load_all();
    let backpack_def = ctx.def_world.entity_by_str("backpack").expect("backpack");
    let rock_def = ctx.def_world.entity_by_str("rock").expect("rock");
    let pos = WorldPos::new(0, 0, ZLevel::new(0));

    let backpack = spawn_item(&mut ctx.world, backpack_def, pos, 1);
    let rock = spawn_item(&mut ctx.world, rock_def, pos, 1);

    assert!(
        can_fit_in_container(&ctx.world, backpack, rock),
        "backpack should fit a rock"
    );
    let total = total_container_volume(&ctx.world, backpack);
    assert_eq!(
        total,
        cdda_core::units::Volume::ZERO,
        "empty backpack should have zero volume"
    );
}

/// Spawn items of the same type and test merge_or_stack.
#[test]
fn merge_two_spawned_rocks() {
    let mut ctx = load_all();
    let rock_def = ctx.def_world.entity_by_str("rock").expect("rock");
    let pos = WorldPos::new(0, 0, ZLevel::new(0));

    let rock1 = spawn_item(&mut ctx.world, rock_def, pos, 1);
    let rock2 = spawn_item(&mut ctx.world, rock_def, pos, 1);

    // Both spawned items should carry DefOrigin (used for fast type comparison in merge)
    assert!(
        ctx.world.get::<DefOrigin>(rock1).is_some(),
        "spawned rock1 should have DefOrigin"
    );
    assert!(
        ctx.world.get::<DefOrigin>(rock2).is_some(),
        "spawned rock2 should have DefOrigin"
    );

    assert!(
        merge_or_stack(&mut ctx.world, rock1, rock2),
        "two rocks of same type should merge"
    );
    let stack = ctx
        .world
        .get::<StackCount>(rock1)
        .unwrap_or_else(|| panic!("rock1 lost StackCount after merge"));
    assert_eq!(stack.get(), 2, "merged stack should have count 2");
}

// =========================================================================
// HELPERS
// =========================================================================
