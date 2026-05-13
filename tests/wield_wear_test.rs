//! Wielding and wearing relationship tests.
//!
//! Exercises the `WieldedBy`/`WieldedItems` and `WornOn`/`WornBy`
//! relationship pairs that connect items to creatures.
//!
//! Covers auto-population of the relationship target, multiple items,
//! reassignment between creatures, removal, slot metadata, and the
//! default empty state.

use bevy_ecs::entity::Entity;
use cdda_components::item::*;
use cdda_core::sim::test_utils::TestBed;

fn empty_entity(test: &mut TestBed) -> Entity {
    test.world_mut().spawn_empty().id()
}

// ===========================================================================
// 1: WieldedBy relationship
// ===========================================================================

#[test]
fn wielded_by_relationship() {
    let mut test = TestBed::new();
    test.register::<WieldedBy>();

    let creature = empty_entity(&mut test);
    let item = test.spawn((WieldedBy(creature),));
    let wielded = test.get::<WieldedBy>(item).unwrap();
    assert_eq!(wielded.0, creature);
}

// ===========================================================================
// 2: WieldedItems auto-populated
// ===========================================================================

#[test]
fn wielded_items_auto_populated() {
    let mut test = TestBed::new();
    test.register::<WieldedBy>();
    test.register::<WieldedItems>();

    let creature = empty_entity(&mut test);
    let item = test.spawn((WieldedBy(creature),));
    let items = test.get::<WieldedItems>(creature);
    assert!(items.is_some(), "WieldedItems should be auto-populated");
    let items = items.unwrap();
    let collected: Vec<Entity> = items.iter().collect();
    assert!(collected.contains(&item), "Item should be in WieldedItems");
}

// ===========================================================================
// 3: Multiple wielded items
// ===========================================================================

#[test]
fn wielded_items_multiple() {
    let mut test = TestBed::new();
    test.register::<WieldedBy>();
    test.register::<WieldedItems>();

    let creature = empty_entity(&mut test);
    let item_a = test.spawn((WieldedBy(creature),));
    let item_b = test.spawn((WieldedBy(creature),));
    let items = test.get::<WieldedItems>(creature).unwrap();
    let collected: Vec<Entity> = items.iter().collect();
    assert!(collected.contains(&item_a), "Item A should be in WieldedItems");
    assert!(collected.contains(&item_b), "Item B should be in WieldedItems");
    assert_eq!(collected.len(), 2, "Both items should be present");
}

// ===========================================================================
// 4: Unwield removes from WieldedItems
// ===========================================================================

#[test]
fn unwield_removes() {
    let mut test = TestBed::new();
    test.register::<WieldedBy>();
    test.register::<WieldedItems>();

    let creature = empty_entity(&mut test);
    let item = test.spawn((WieldedBy(creature),));

    // Verify it's there before removal
    {
        let items = test.get::<WieldedItems>(creature).unwrap();
        let collected: Vec<Entity> = items.iter().collect();
        assert!(collected.contains(&item));
    }

    // Remove WieldedBy from the item
    test.world_mut().entity_mut(item).remove::<WieldedBy>();

    // Check it's gone from WieldedItems
    let items = test.get::<WieldedItems>(creature);
    match items {
        Some(items) => {
            let collected: Vec<Entity> = items.iter().collect();
            assert!(!collected.contains(&item), "Item should be removed from WieldedItems");
        }
        None => {
            // If WieldedItems was removed entirely (empty relationship target),
            // that's also valid — no items means no WieldedItems component
        }
    }
}

// ===========================================================================
// 5: Wield reassignment — change which creature wields an item
// ===========================================================================

#[test]
fn wield_reassign() {
    let mut test = TestBed::new();
    test.register::<WieldedBy>();
    test.register::<WieldedItems>();

    let old_creature = empty_entity(&mut test);
    let new_creature = empty_entity(&mut test);
    let item = test.spawn((WieldedBy(old_creature),));

    // Reassign by reinserting WieldedBy with a new entity
    test.world_mut()
        .entity_mut(item)
        .insert(WieldedBy(new_creature));

    // Old creature should no longer have the item
    let old_items = test.get::<WieldedItems>(old_creature);
    match old_items {
        Some(items) => {
            let collected: Vec<Entity> = items.iter().collect();
            assert!(!collected.contains(&item), "Old creature should not have the item");
        }
        None => { /* valid — empty collection may be removed */ }
    }

    // New creature should have the item
    let new_items = test.get::<WieldedItems>(new_creature).unwrap();
    let collected: Vec<Entity> = new_items.iter().collect();
    assert!(collected.contains(&item), "New creature should have the item");
}

// ===========================================================================
// 6: WornOn relationship with slot
// ===========================================================================

#[test]
fn worn_on_relationship() {
    let mut test = TestBed::new();
    test.register::<WornOn>();

    let creature = empty_entity(&mut test);
    let item = test.spawn((WornOn {
        wearer: creature,
        slot: Some("torso".into()),
    },));
    let worn = test.get::<WornOn>(item).unwrap();
    assert_eq!(worn.wearer, creature);
    assert_eq!(worn.slot.as_deref(), Some("torso"));
}

// ===========================================================================
// 7: WornBy auto-populated
// ===========================================================================

#[test]
fn worn_by_auto_populated() {
    let mut test = TestBed::new();
    test.register::<WornOn>();
    test.register::<WornBy>();

    let creature = empty_entity(&mut test);
    let item = test.spawn((WornOn {
        wearer: creature,
        slot: None,
    },));
    let worn_items = test.get::<WornBy>(creature);
    assert!(worn_items.is_some(), "WornBy should be auto-populated");
    let collected: Vec<Entity> = worn_items.unwrap().iter().collect();
    assert!(collected.contains(&item), "Item should be in WornBy");
}

// ===========================================================================
// 8: Worn slot values — various slot types stored and read back
// ===========================================================================

#[test]
fn worn_slot_stored_and_read() {
    let mut test = TestBed::new();
    test.register::<WornOn>();

    let creature = empty_entity(&mut test);

    // No slot
    let item_none = test.spawn((WornOn {
        wearer: creature,
        slot: None,
    },));
    assert_eq!(
        test.get::<WornOn>(item_none).unwrap().slot,
        None,
        "None slot should be None"
    );

    // Torso slot
    let item_torso = test.spawn((WornOn {
        wearer: creature,
        slot: Some("torso".into()),
    },));
    assert_eq!(
        test.get::<WornOn>(item_torso).unwrap().slot.as_deref(),
        Some("torso"),
        "torso slot should be stored and read back"
    );

    // Arm slot
    let item_arm = test.spawn((WornOn {
        wearer: creature,
        slot: Some("arm_l".into()),
    },));
    assert_eq!(
        test.get::<WornOn>(item_arm).unwrap().slot.as_deref(),
        Some("arm_l"),
        "arm_l slot should be stored and read back"
    );
}

// ===========================================================================
// 9: Worn reassignment — move worn item between creatures
// ===========================================================================

#[test]
fn worn_reassign() {
    let mut test = TestBed::new();
    test.register::<WornOn>();
    test.register::<WornBy>();

    let old_creature = empty_entity(&mut test);
    let new_creature = empty_entity(&mut test);

    let item = test.spawn((WornOn {
        wearer: old_creature,
        slot: Some("torso".into()),
    },));

    // Reassign to new creature by reinserting WornOn
    test.world_mut()
        .entity_mut(item)
        .insert(WornOn {
            wearer: new_creature,
            slot: Some("torso".into()),
        });

    // Old creature should no longer have the item
    let old_worn = test.get::<WornBy>(old_creature);
    match old_worn {
        Some(worn) => {
            let collected: Vec<Entity> = worn.iter().collect();
            assert!(!collected.contains(&item), "Old creature should not have the item");
        }
        None => { /* valid — empty collection removed */ }
    }

    // New creature should have the item
    let new_worn = test.get::<WornBy>(new_creature).unwrap();
    let collected: Vec<Entity> = new_worn.iter().collect();
    assert!(collected.contains(&item), "New creature should have the item");
}

// ===========================================================================
// 10: No equipment by default
// ===========================================================================

#[test]
fn no_equipment_by_default() {
    let mut test = TestBed::new();
    test.register::<WieldedBy>();
    test.register::<WieldedItems>();
    test.register::<WornOn>();
    test.register::<WornBy>();

    let creature = empty_entity(&mut test);

    // A freshly spawned creature has no wielded or worn items
    assert!(
        test.get::<WieldedItems>(creature).is_none(),
        "No WieldedItems on fresh creature"
    );
    assert!(
        test.get::<WornBy>(creature).is_none(),
        "No WornBy on fresh creature"
    );
}
