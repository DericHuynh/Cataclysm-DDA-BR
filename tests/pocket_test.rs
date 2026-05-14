//! Pocket, container, and item-stacking tests.
//!
//! Exercises the pocket system components, stack-count invariants,
//! and inventory relationships.

use bevy_ecs::entity::Entity;
use cdda_components::item::IsPocket;
use cdda_components::item::PocketRestriction;
use cdda_components::item::PocketType;
use cdda_components::item::*;
use cdda_sim::test_utils::TestBed;
use cdda_core_types::core::units::{Length, Volume, Weight};

// ===========================================================================
// Helpers
// ===========================================================================

fn small_pocket() -> Pocket {
    Pocket {
        max_volume: Volume::from_milliliters(1000),
        max_weight: Weight::from_grams(2000),
        max_item_length: Length::from_millimeters(300),
        min_item_volume: Volume::from_milliliters(50),
        pocket_type: PocketType::Container,
    }
}

fn make_container(test: &mut TestBed) -> Entity {
    test.spawn((Container {
        capacity: Volume::from_milliliters(5000),
    },))
}

// ===========================================================================
// 1–3: Volume fitting
// ===========================================================================

#[test]
fn small_item_fits_in_pocket() {
    let pocket = small_pocket();
    let item_vol = Volume::from_milliliters(250);
    assert!(item_vol <= pocket.max_volume);
}

#[test]
fn large_item_does_not_fit_in_pocket() {
    let pocket = small_pocket();
    let item_vol = Volume::from_milliliters(5000);
    assert!(item_vol > pocket.max_volume);
}

#[test]
fn pocket_volume_exact_fit() {
    let pocket = small_pocket();
    let item_vol = Volume::from_milliliters(1000);
    assert!(item_vol <= pocket.max_volume);
    assert_eq!(item_vol, pocket.max_volume);
}

// ===========================================================================
// 4: Weight capacity
// ===========================================================================

#[test]
fn pocket_weight_capacity() {
    let pocket = small_pocket();
    let light = Weight::from_grams(500);
    let heavy = Weight::from_grams(5000);
    assert!(light <= pocket.max_weight);
    assert!(heavy > pocket.max_weight);
}

// ===========================================================================
// 5: Item length constraint
// ===========================================================================

#[test]
fn pocket_item_length() {
    let pocket = small_pocket();
    let short = Length::from_millimeters(150);
    let long = Length::from_millimeters(600);
    assert!(short <= pocket.max_item_length);
    assert!(long > pocket.max_item_length);
}

// ===========================================================================
// 6: Minimum volume
// ===========================================================================

#[test]
fn pocket_min_volume() {
    let pocket = small_pocket();
    let tiny = Volume::from_milliliters(10);
    let normal = Volume::from_milliliters(100);
    assert!(tiny < pocket.min_item_volume);
    assert!(normal >= pocket.min_item_volume);
}

// ===========================================================================
// 7: Pocket type — magazine restriction
// ===========================================================================

#[test]
fn pocket_type_magazine_restriction() {
    let mag_pocket = Pocket {
        pocket_type: PocketType::Magazine,
        ..small_pocket()
    };
    let container_pocket = Pocket {
        pocket_type: PocketType::Container,
        ..small_pocket()
    };
    assert_eq!(mag_pocket.pocket_type, PocketType::Magazine);
    assert_eq!(container_pocket.pocket_type, PocketType::Container);
    assert_ne!(mag_pocket.pocket_type, container_pocket.pocket_type);
}

// ===========================================================================
// 8: Ammo type filter
// ===========================================================================

#[test]
fn pocket_ammo_type_filter() {
    let restriction = PocketRestriction {
        allowed_flags: Vec::new(),
        allowed_items: vec![],
        ammo_type: Some("223".to_string()),
        item_category: None,
        max_item_volume: Volume::from_milliliters(500),
    };
    assert_eq!(restriction.ammo_type.as_deref(), Some("223"));
}

// ===========================================================================
// 9: Container capacity
// ===========================================================================

#[test]
fn container_capacity() {
    let mut test = TestBed::new();
    test.register::<Container>();

    let e = test.spawn((Container {
        capacity: Volume::from_milliliters(5000),
    },));
    let container = test.get::<Container>(e).unwrap();
    assert_eq!(container.capacity.as_milliliters(), 5000);
}

// ===========================================================================
// 10–12: StackCount
// ===========================================================================

#[test]
fn stack_count_basics() {
    let mut test = TestBed::new();
    test.register::<StackCount>();

    let e = test.spawn((StackCount::new(1).unwrap(),));
    assert_eq!(test.get::<StackCount>(e).unwrap().get(), 1);
}

#[test]
fn stack_count_multiple() {
    let mut test = TestBed::new();
    test.register::<StackCount>();

    let e = test.spawn((StackCount::new(10).unwrap(),));
    assert_eq!(test.get::<StackCount>(e).unwrap().get(), 10);
}

#[test]
fn stack_count_zero_returns_err() {
    assert!(StackCount::new(0).is_err());
}

// ===========================================================================
// 13–15: InsideContainer ↔ ContainerContents relationship
// ===========================================================================

#[test]
fn inside_container_relationship() {
    let mut test = TestBed::new();
    test.register::<InsideContainer>();
    test.register::<ContainerContents>();

    let container = make_container(&mut test);
    let item = test.spawn((StackCount::new(1).unwrap(), InsideContainer(container)));

    let on_item = test.get::<InsideContainer>(item).unwrap();
    assert_eq!(on_item.0, container);
}

#[test]
fn container_contents_queryable() {
    let mut test = TestBed::new();
    test.register::<InsideContainer>();
    test.register::<ContainerContents>();
    test.register::<StackCount>();

    let container = make_container(&mut test);
    let item = test.spawn((StackCount::new(1).unwrap(), InsideContainer(container)));

    // ContainerContents should have been populated by the relationship hook
    let contents = test.get::<ContainerContents>(container).unwrap();
    let entities: Vec<Entity> = contents.iter().collect();
    assert!(
        entities.contains(&item),
        "container should contain the item"
    );
}

#[test]
fn container_reinsertion_updates() {
    let mut test = TestBed::new();
    test.register::<InsideContainer>();
    test.register::<ContainerContents>();
    test.register::<StackCount>();

    let container_a = make_container(&mut test);
    let container_b = make_container(&mut test);
    let item = test.spawn((StackCount::new(1).unwrap(), InsideContainer(container_a)));

    // Verify it's in container_a
    let contents_a = test.get::<ContainerContents>(container_a).unwrap();
    assert!(contents_a.iter().any(|e| e == item));

    // Reinsert with new container — hooks should update both sides
    test.world_mut()
        .entity_mut(item)
        .insert(InsideContainer(container_b));

    // container_a should no longer contain the item;
    // ContainerContents may be removed entirely when empty
    let contents_a = test.get::<ContainerContents>(container_a);
    match contents_a {
        Some(c) => assert!(!c.iter().any(|e| e == item)),
        None => { /* removed because empty — also valid */ }
    }

    // container_b should contain the item
    let contents_b = test.get::<ContainerContents>(container_b).unwrap();
    assert!(contents_b.iter().any(|e| e == item));
}

// ===========================================================================
// 16–17: Charges and ammo
// ===========================================================================

#[test]
fn current_charges_default() {
    let mut test = TestBed::new();
    test.register::<CurrentCharges>();

    let e = test.spawn((CurrentCharges::default(),));
    assert_eq!(test.get::<CurrentCharges>(e).unwrap().0, 0);
}

#[test]
fn loaded_ammo_store_and_read() {
    let mut test = TestBed::new();
    test.register::<LoadedAmmo>();

    let e = test.spawn((LoadedAmmo(30),));
    assert_eq!(test.get::<LoadedAmmo>(e).unwrap().0, 30);
}

// ===========================================================================
// 18: MountedOn ↔ MountedPockets relationship
// ===========================================================================

#[test]
fn mounted_pockets_relationship() {
    let mut test = TestBed::new();
    test.register::<MountedOn>();
    test.register::<MountedPockets>();
    test.register::<Pocket>();

    let garment = test.spawn((Pocket {
        max_volume: Volume::from_milliliters(500),
        max_weight: Weight::from_grams(1000),
        max_item_length: Length::from_millimeters(200),
        min_item_volume: Volume::from_milliliters(10),
        pocket_type: PocketType::Container,
    },));
    let pocket_entity = test.spawn((MountedOn(garment),));

    let mounted = test.get::<MountedOn>(pocket_entity).unwrap();
    assert_eq!(mounted.0, garment);

    let pockets = test.get::<MountedPockets>(garment).unwrap();
    assert!(
        pockets.iter().any(|e| e == pocket_entity),
        "garment should list the pocket entity"
    );
}

// ===========================================================================
// 19: Container tags
// ===========================================================================

#[test]
fn container_tags() {
    let mut test = TestBed::new();
    test.register::<Sealed>();
    test.register::<Rigid>();
    test.register::<Watertight>();
    test.register::<PreservesTemp>();

    let e = test.spawn((Sealed, Rigid, Watertight, PreservesTemp));
    let world = test.world();
    assert!(world.entity(e).contains::<Sealed>());
    assert!(world.entity(e).contains::<Rigid>());
    assert!(world.entity(e).contains::<Watertight>());
    assert!(world.entity(e).contains::<PreservesTemp>());
}

// ===========================================================================
// 20: PocketRestriction flag filter
// ===========================================================================

#[test]
fn pocket_restriction_flag_filter() {
    let restriction = PocketRestriction {
        allowed_flags: Vec::new(),
        allowed_items: vec![],
        ammo_type: None,
        item_category: None,
        max_item_volume: Volume::from_milliliters(1000),
    };
    // allowed_flags is now Vec<u16> (flag indices), checked via registry lookups
    assert!(restriction.allowed_flags.is_empty());
}
