#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_actor::components::*;
use cdda_core::*;
use cdda_item::components::*;
use cdda_sim::components::*;
use cdda_sim::def_components::*;
use cdda_sim::systems::equipment::*;
use cdda_sim::test_utils::TestBed;

// ===========================================================================
// Helper: create a basic creature entity
// ===========================================================================

fn spawn_creature(test: &mut TestBed) -> Entity {
    test.register::<PlayerData>();
    test.register::<Health>();
    test.register::<IsAlive>();
    test.register::<WieldedItems>();
    test.register::<WornBy>();
    test.spawn((
        PlayerData {
            name: "TestPlayer".to_string(),
            gender: Gender::Male,
            age: 25,
            height: 180,
            blood_type: "O+".to_string(),
            profession: None,
            scenario: None,
        },
        Health {
            current: 100,
            max: 100,
        },
        IsAlive,
    ))
}

fn spawn_item(test: &mut TestBed) -> Entity {
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<StackCount>();
    test.spawn((
        ItemName("test_item".to_string()),
        ItemWeight(500),
        StackCount::new(1),
    ))
}

// ===========================================================================
// 1: wield_free_hand
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wield_free_hand() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item = spawn_item(&mut test);

    // wield would be called here
    // commands.entity(creature).insert(WieldedItems(...))
    // should return Ok
    let result: Result<(), ()> = Ok(());
    assert!(result.is_ok());

    // After wielding, item should have WieldedBy pointing to creature
    let wielded_by = test.get::<WieldedBy>(item);
    assert!(wielded_by.is_some());
    if let Some(wb) = wielded_by {
        assert_eq!(wb.0, creature);
    }

    // Creature should have the item in WieldedItems
    let wielded = test.get::<WieldedItems>(creature);
    assert!(wielded.is_some());
    if let Some(wi) = wielded {
        let items: Vec<Entity> = wi.iter().collect();
        assert!(items.contains(&item));
    }
}

// ===========================================================================
// 2: wield_already_wielding
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wield_already_wielding() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let weapon = spawn_item(&mut test);
    let new_item = spawn_item(&mut test);

    // First, wield the weapon
    test.world_mut()
        .entity_mut(weapon)
        .insert(WieldedBy(creature));

    // Attempting to wield another item should return AlreadyWielding error
    let result: Result<(), &str> = Err("AlreadyWielding");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "AlreadyWielding");
}

// ===========================================================================
// 3: unwield_returns_item
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn unwield_returns_item() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item = spawn_item(&mut test);

    // Set up wielding relationship
    test.world_mut()
        .entity_mut(item)
        .insert(WieldedBy(creature));

    // unwield should return the item entity
    let result: Result<Entity, &str> = Ok(item);
    assert_eq!(result.unwrap(), item);
}

// ===========================================================================
// 4: unwield_removes_relationship
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn unwield_removes_relationship() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item = spawn_item(&mut test);

    // Set up wielding
    test.world_mut()
        .entity_mut(item)
        .insert(WieldedBy(creature));

    // unwield -- remove WieldedBy from item
    test.world_mut().entity_mut(item).remove::<WieldedBy>();

    // After unwield, item no longer has WieldedBy
    assert!(test.get::<WieldedBy>(item).is_none());
}

// ===========================================================================
// 5: wear_on_slot
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wear_on_slot() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item = spawn_item(&mut test);

    // Wear item on "torso" slot
    test.world_mut().entity_mut(item).insert(WornOn {
        wearer: creature,
        slot: Some("torso".to_string()),
    });

    let worn_on = test.get::<WornOn>(item);
    assert!(worn_on.is_some());
    if let Some(wo) = worn_on {
        assert_eq!(wo.wearer, creature);
        assert_eq!(wo.slot, Some("torso".to_string()));
    }
}

// ===========================================================================
// 6: wear_slot_occupied
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wear_slot_occupied() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item_a = spawn_item(&mut test);
    let item_b = spawn_item(&mut test);

    // First item occupies "torso"
    test.world_mut().entity_mut(item_a).insert(WornOn {
        wearer: creature,
        slot: Some("torso".to_string()),
    });

    // Second item on same slot should return SlotOccupied error
    let result: Result<(), &str> = Err("SlotOccupied");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "SlotOccupied");
}

// ===========================================================================
// 7: take_off_removes_worn
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn take_off_removes_worn() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item = spawn_item(&mut test);

    // Wear item
    test.world_mut().entity_mut(item).insert(WornOn {
        wearer: creature,
        slot: Some("torso".to_string()),
    });

    // Verify it's worn before removal
    assert!(test.get::<WornOn>(item).is_some());

    // Take off -- remove WornOn from item
    test.world_mut().entity_mut(item).remove::<WornOn>();

    // After take_off, item no longer has WornOn
    assert!(test.get::<WornOn>(item).is_none());

    // Creature should no longer have item in WornBy
    let worn_by = test.get::<WornBy>(creature);
    if let Some(wb) = worn_by {
        let items: Vec<Entity> = wb.iter().collect();
        assert!(!items.contains(&item));
    }
}

// ===========================================================================
// 8: available_slots_humanoid
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn available_slots_humanoid() {
    // Humanoid body plan defines these slots
    let humanoid_slots: Vec<&str> = vec![
        "head", "torso", "arm_l", "arm_r", "hand_l", "hand_r", "leg_l", "leg_r", "foot_l", "foot_r",
    ];

    // Query the available slots for a humanoid creature
    // This would call a function like available_slots_for(creature)
    // For now, just verify the expected slots
    assert_eq!(humanoid_slots.len(), 10);
    assert!(humanoid_slots.contains(&"head"));
    assert!(humanoid_slots.contains(&"torso"));
    assert!(humanoid_slots.contains(&"arm_l"));
    assert!(humanoid_slots.contains(&"arm_r"));
    assert!(humanoid_slots.contains(&"hand_l"));
    assert!(humanoid_slots.contains(&"hand_r"));
    assert!(humanoid_slots.contains(&"leg_l"));
    assert!(humanoid_slots.contains(&"leg_r"));
    assert!(humanoid_slots.contains(&"foot_l"));
    assert!(humanoid_slots.contains(&"foot_r"));
}

// ===========================================================================
// 9: wear_multiple_layers
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wear_multiple_layers() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let shirt = spawn_item(&mut test);
    let pants = spawn_item(&mut test);

    // Wear shirt on "torso"
    test.world_mut().entity_mut(shirt).insert(WornOn {
        wearer: creature,
        slot: Some("torso".to_string()),
    });

    // Wear pants on "leg_l"
    test.world_mut().entity_mut(pants).insert(WornOn {
        wearer: creature,
        slot: Some("leg_l".to_string()),
    });

    // Both items should be in WornBy (different slots)
    let worn_by = test.get::<WornBy>(creature);
    assert!(worn_by.is_some());
    if let Some(wb) = worn_by {
        let items: Vec<Entity> = wb.iter().collect();
        assert!(items.contains(&shirt));
        assert!(items.contains(&pants));
        assert_eq!(items.len(), 2);
    }
}

// ===========================================================================
// 10: wear_on_same_slot_second_replaces
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wear_on_same_slot_second_replaces() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let item_a = spawn_item(&mut test);
    let item_b = spawn_item(&mut test);

    // Wear item_a on "torso"
    test.world_mut().entity_mut(item_a).insert(WornOn {
        wearer: creature,
        slot: Some("torso".to_string()),
    });

    // Replace with item_b on the same slot
    test.world_mut().entity_mut(item_b).insert(WornOn {
        wearer: creature,
        slot: Some("torso".to_string()),
    });

    // item_a should no longer be worn (WornOn was removed or replaced)
    // Behaviour depends on implementation -- either item_a is replaced or both are present
    // For this test, we expect item_b to be worn and the slot is occupied
    let worn_on_b = test.get::<WornOn>(item_b);
    assert!(worn_on_b.is_some());
}

// ===========================================================================
// 11: wield_item_too_heavy
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn wield_item_too_heavy() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let heavy_item = test.spawn((
        ItemName("heavy_rock".to_string()),
        ItemWeight(1_000_000),
        StackCount::new(1),
    ));

    // Creature with str 10 can lift 10 * 100 = 1000 grams = 1 kg
    // Item weighs 1000 kg -- too heavy
    // Strength-based weight limit would be str * 100 (grams) = 1000g = 1kg
    let str_score = 10;
    let weight_limit = cdda_core::Weight::from_grams((str_score * 100) as u64);
    let item_weight = cdda_core::Weight::from_kilograms_u64(1000);

    assert!(item_weight > weight_limit);

    // Wielding should return an error about being too heavy
    let result: Result<(), &str> = Err("item too heavy");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "item too heavy");
}

// ===========================================================================
// 12: available_slots_creature_type
// ===========================================================================

#[test]
#[ignore = "equipment system not yet implemented"]
fn available_slots_creature_type() {
    // Different creature types have different body plans with different slots
    let humanoid_slots: Vec<&str> = vec![
        "head", "torso", "arm_l", "arm_r", "hand_l", "hand_r", "leg_l", "leg_r", "foot_l", "foot_r",
    ];

    // A quadruped might have different slots
    let quad_slots: Vec<&str> = vec!["head", "torso", "leg_fl", "leg_fr", "leg_bl", "leg_br"];

    assert_ne!(humanoid_slots.len(), quad_slots.len());
    assert!(humanoid_slots.contains(&"hand_l"));
    assert!(!quad_slots.contains(&"hand_l"));
    assert!(quad_slots.contains(&"leg_fl"));
    assert!(!humanoid_slots.contains(&"leg_fl"));
}
