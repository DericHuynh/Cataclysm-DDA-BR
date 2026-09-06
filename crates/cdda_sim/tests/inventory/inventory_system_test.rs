//! Integration tests for the inventory system functions.
//!
//! Tests call the actual implementations in `cdda_core::sim::systems::inventory`
//! and verify correct behaviour against CDDA's item-pocket rules.

use cdda_components::def::{DefStrId, ItemLongestSide, ItemVolume, ItemWeight};
use cdda_components::item::{
    Container, ContainerContents, CurrentCharges, DefOrigin, InsideContainer, ItemDamage, Pocket,
    StackCount,
};
use cdda_core_types::core::units::*;
use cdda_sim::inventory::systems::*;
use cdda_sim::runtime::test_utils::TestBed;

// ---- helpers ---------------------------------------------------------------

/// Register all components needed by the inventory system functions.
fn register_inventory_components(test: &mut TestBed) {
    // Def components
    test.register::<DefStrId>();
    test.register::<ItemVolume>();
    test.register::<ItemWeight>();
    test.register::<ItemLongestSide>();
    // Runtime item components
    test.register::<DefOrigin>();
    test.register::<StackCount>();
    test.register::<CurrentCharges>();
    test.register::<ItemDamage>();
    // Container / pocket components
    test.register::<Container>();
    test.register::<Pocket>();
    // Relationship components (hooks need registration)
    test.register::<InsideContainer>();
    test.register::<ContainerContents>();
}

// ============================================================================
// can_fit_in_container
// ============================================================================

#[test]
fn small_item_fits_pocket() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("backpack".into()),
        Pocket {
            max_volume: Volume::from_milliliters(2000),
            max_weight: Weight::from_grams(5000),
            max_item_length: Length::from_millimeters(500),
            min_item_volume: Volume::ZERO,
            pocket_type: cdda_components::item::PocketType::Container,
        },
    ));
    let item = test.spawn((DefStrId("rock".into()), ItemVolume(250), ItemWeight(100)));

    assert!(can_fit_in_container(test.world(), container, item));
}

#[test]
fn large_item_exceeds_pocket_volume() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("small_pouch".into()),
        Pocket {
            max_volume: Volume::from_milliliters(500),
            max_weight: Weight::from_grams(5000),
            max_item_length: Length::from_millimeters(500),
            min_item_volume: Volume::ZERO,
            pocket_type: cdda_components::item::PocketType::Container,
        },
    ));
    let item = test.spawn((
        DefStrId("big_rock".into()),
        ItemVolume(1000),
        ItemWeight(100),
    ));

    assert!(!can_fit_in_container(test.world(), container, item));
}

#[test]
fn heavy_item_exceeds_pocket_weight() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("pouch".into()),
        Pocket {
            max_volume: Volume::from_milliliters(5000),
            max_weight: Weight::from_grams(1000),
            max_item_length: Length::from_millimeters(500),
            min_item_volume: Volume::ZERO,
            pocket_type: cdda_components::item::PocketType::Container,
        },
    ));
    let item = test.spawn((DefStrId("anvil".into()), ItemVolume(100), ItemWeight(5000)));

    assert!(!can_fit_in_container(test.world(), container, item));
}

#[test]
fn short_item_fits_length_constraint() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("holster".into()),
        Pocket {
            max_volume: Volume::from_milliliters(1000),
            max_weight: Weight::from_grams(5000),
            max_item_length: Length::from_millimeters(300),
            min_item_volume: Volume::ZERO,
            pocket_type: cdda_components::item::PocketType::Container,
        },
    ));
    let item = test.spawn((
        DefStrId("short_stick".into()),
        ItemVolume(200),
        ItemWeight(100),
        ItemLongestSide(150),
    ));

    assert!(can_fit_in_container(test.world(), container, item));
}

#[test]
fn long_item_exceeds_length() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("small_holster".into()),
        Pocket {
            max_volume: Volume::from_milliliters(1000),
            max_weight: Weight::from_grams(5000),
            max_item_length: Length::from_millimeters(200),
            min_item_volume: Volume::ZERO,
            pocket_type: cdda_components::item::PocketType::Container,
        },
    ));
    let item = test.spawn((
        DefStrId("long_stick".into()),
        ItemVolume(400),
        ItemWeight(100),
        ItemLongestSide(500),
    ));

    assert!(!can_fit_in_container(test.world(), container, item));
}

#[test]
fn item_below_min_volume_rejected() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("pouch".into()),
        Pocket {
            max_volume: Volume::from_milliliters(1000),
            max_weight: Weight::from_grams(5000),
            max_item_length: Length::from_millimeters(500),
            min_item_volume: Volume::from_milliliters(100),
            pocket_type: cdda_components::item::PocketType::Container,
        },
    ));
    let item = test.spawn((
        DefStrId("tiny_pebble".into()),
        ItemVolume(50),
        ItemWeight(10),
    ));

    assert!(!can_fit_in_container(test.world(), container, item));
}

#[test]
fn fits_in_capacity_container() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("sack".into()),
        Container {
            capacity: Volume::from_milliliters(5000),
        },
    ));
    let item = test.spawn((DefStrId("rock".into()), ItemVolume(250), ItemWeight(100)));

    assert!(can_fit_in_container(test.world(), container, item));
}

#[test]
fn exceeds_container_capacity() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("small_sack".into()),
        Container {
            capacity: Volume::from_milliliters(500),
        },
    ));
    let item = test.spawn((
        DefStrId("big_rock".into()),
        ItemVolume(1000),
        ItemWeight(100),
    ));

    assert!(!can_fit_in_container(test.world(), container, item));
}

// ============================================================================
// total_container_volume
// ============================================================================

#[test]
fn empty_container_has_zero_volume() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("sack".into()),
        Container {
            capacity: Volume::from_milliliters(5000),
        },
    ));

    assert_eq!(
        total_container_volume(test.world(), container),
        Volume::ZERO
    );
}

#[test]
fn container_volume_sums_items() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("sack".into()),
        Container {
            capacity: Volume::from_milliliters(5000),
        },
    ));
    // Spawn items with InsideContainer — Bevy relationship hooks
    // auto-populate ContainerContents on the container entity.
    test.spawn((
        DefStrId("rock".into()),
        ItemVolume(250),
        ItemWeight(100),
        InsideContainer(container),
    ));
    test.spawn((
        DefStrId("stick".into()),
        ItemVolume(500),
        ItemWeight(50),
        InsideContainer(container),
    ));

    assert_eq!(
        total_container_volume(test.world(), container),
        Volume::from_milliliters(750)
    );
}

#[test]
fn container_volume_includes_stack_count() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("sack".into()),
        Container {
            capacity: Volume::from_milliliters(5000),
        },
    ));
    // A single item with StackCount(5) — volume should be 5 × 100 = 500
    test.spawn((
        DefStrId("rock".into()),
        ItemVolume(100),
        ItemWeight(50),
        StackCount::new(5).unwrap(),
        InsideContainer(container),
    ));

    assert_eq!(
        total_container_volume(test.world(), container),
        Volume::from_milliliters(500)
    );
}

// ============================================================================
// total_container_weight
// ============================================================================

#[test]
fn container_weight_sums_items() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let container = test.spawn((
        DefStrId("sack".into()),
        Container {
            capacity: Volume::from_milliliters(5000),
        },
    ));
    test.spawn((
        DefStrId("rock".into()),
        ItemVolume(100),
        ItemWeight(200),
        InsideContainer(container),
    ));
    test.spawn((
        DefStrId("stick".into()),
        ItemVolume(100),
        ItemWeight(300),
        InsideContainer(container),
    ));

    assert_eq!(
        total_container_weight(test.world(), container),
        Weight::from_grams(500)
    );
}

// ============================================================================
// merge_or_stack
// ============================================================================

#[test]
fn same_items_merge() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let target = test.spawn((
        DefStrId("rock".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));
    let incoming = test.spawn((
        DefStrId("rock".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));

    // Phase 1: read-only checks (all immutable borrows)
    let t_id = test.get::<DefStrId>(target).unwrap();
    let i_id = test.get::<DefStrId>(incoming).unwrap();
    assert_eq!(t_id.0, i_id.0);

    // Phase 2: mutation
    for e in [target, incoming] {
        test.world_mut()
            .entity_mut(e)
            .insert(cdda_components::sim::WorldPosition::new(
                cdda_core_types::core::coords::WorldPos::new(
                    0,
                    0,
                    cdda_core_types::core::coords::ZLevel::new(0),
                ),
            ));
    }
    assert!(merge_or_stack(test.world_mut(), target, incoming));

    // Phase 3: verify target has StackCount(2), incoming is despawned
    let merged = test.get::<StackCount>(target).unwrap().get();
    assert_eq!(merged, 2);
    assert!(test.get::<DefStrId>(incoming).is_none());
}

#[test]
fn different_items_dont_merge() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let target = test.spawn((
        DefStrId("rock".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));
    let incoming = test.spawn((
        DefStrId("stick".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));

    // Phase 1: verify they have different IDs
    let t_id = test.get::<DefStrId>(target).unwrap();
    let i_id = test.get::<DefStrId>(incoming).unwrap();
    assert_ne!(t_id.0, i_id.0);

    // Phase 2: mutation — should return false
    assert!(!merge_or_stack(test.world_mut(), target, incoming));

    // Phase 3: both entities still alive and unchanged
    assert_eq!(test.get::<StackCount>(target).unwrap().get(), 1);
    assert!(test.get::<DefStrId>(incoming).is_some());
}

#[test]
fn different_damage_prevents_merge() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let target = test.spawn((
        DefStrId("rock".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));
    let incoming = test.spawn((
        DefStrId("rock".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(2),
    ));

    // Phase 1: same DefStrId, different damage
    let t_id = test.get::<DefStrId>(target).unwrap();
    let i_id = test.get::<DefStrId>(incoming).unwrap();
    assert_eq!(t_id.0, i_id.0);
    let t_dmg = test.get::<ItemDamage>(target).unwrap();
    let i_dmg = test.get::<ItemDamage>(incoming).unwrap();
    assert_ne!(t_dmg.0, i_dmg.0);

    // Phase 2: mutation — should return false (CDDA rule: only same-condition stacks merge)
    assert!(!merge_or_stack(test.world_mut(), target, incoming));

    // Phase 3: both still alive, target count unchanged
    assert_eq!(test.get::<StackCount>(target).unwrap().get(), 1);
    assert!(test.get::<StackCount>(incoming).is_some());
}

#[test]
fn merge_accumulates_charges() {
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let target = test.spawn((
        DefStrId("lighter".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(5),
        ItemDamage(0),
    ));
    let incoming = test.spawn((
        DefStrId("lighter".into()),
        StackCount::new(1).unwrap(),
        CurrentCharges(3),
        ItemDamage(0),
    ));

    // Phase 1: verify same type, same damage
    assert_eq!(
        test.get::<DefStrId>(target).unwrap().0,
        test.get::<DefStrId>(incoming).unwrap().0,
    );
    assert_eq!(
        test.get::<ItemDamage>(target).unwrap().0,
        test.get::<ItemDamage>(incoming).unwrap().0,
    );

    // Phase 2: merge
    for e in [target, incoming] {
        test.world_mut()
            .entity_mut(e)
            .insert(cdda_components::sim::WorldPosition::new(
                cdda_core_types::core::coords::WorldPos::new(
                    0,
                    0,
                    cdda_core_types::core::coords::ZLevel::new(0),
                ),
            ));
    }
    assert!(merge_or_stack(test.world_mut(), target, incoming));

    // Phase 3: StackCount = 2, CurrentCharges = 8
    assert_eq!(test.get::<StackCount>(target).unwrap().get(), 2);
    assert_eq!(test.get::<CurrentCharges>(target).unwrap().0, 8);
    assert!(test.get::<DefStrId>(incoming).is_none());
}

#[test]
fn deforigin_merge_same_origin() {
    // Items with the same DefOrigin should merge even without DefStrId.
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let target = test.spawn((
        DefOrigin(42),
        StackCount::new(3).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));
    let incoming = test.spawn((
        DefOrigin(42),
        StackCount::new(2).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));

    // Phase 1: verify same DefOrigin
    assert_eq!(
        test.get::<DefOrigin>(target).unwrap().0,
        test.get::<DefOrigin>(incoming).unwrap().0,
    );

    // Phase 2: merge
    for e in [target, incoming] {
        test.world_mut()
            .entity_mut(e)
            .insert(cdda_components::sim::WorldPosition::new(
                cdda_core_types::core::coords::WorldPos::new(
                    0,
                    0,
                    cdda_core_types::core::coords::ZLevel::new(0),
                ),
            ));
    }
    assert!(merge_or_stack(test.world_mut(), target, incoming));

    // Phase 3: StackCount = 5, incoming despawned
    assert_eq!(test.get::<StackCount>(target).unwrap().get(), 5);
    assert!(test.get::<DefOrigin>(incoming).is_none());
}

#[test]
fn deforigin_different_origin_no_merge() {
    // Items with different DefOrigin should not merge.
    let mut test = TestBed::new();
    register_inventory_components(&mut test);

    let target = test.spawn((
        DefOrigin(42),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));
    let incoming = test.spawn((
        DefOrigin(99),
        StackCount::new(1).unwrap(),
        CurrentCharges(0),
        ItemDamage(0),
    ));

    // Phase 1: verify different DefOrigin
    assert_ne!(
        test.get::<DefOrigin>(target).unwrap().0,
        test.get::<DefOrigin>(incoming).unwrap().0,
    );

    // Phase 2: should not merge
    assert!(!merge_or_stack(test.world_mut(), target, incoming));

    // Phase 3: both entities still alive and unchanged
    assert_eq!(test.get::<StackCount>(target).unwrap().get(), 1);
    assert!(test.get::<DefOrigin>(incoming).is_some());
}
