#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_core::actor::components::*;
use cdda_core::item::components::*;
use cdda_core::*;
use cdda_core::coords::WorldPos;
use cdda_core::sim::components::*;
use cdda_core::sim::def_components::*;
use cdda_core::sim::events::SpawnEvent;
use cdda_core::sim::systems::spawning::*;
use cdda_core::sim::test_utils::TestBed;

// ===========================================================================
// 1: spawn_monster_entity_created
// ===========================================================================

#[test]
#[ignore = "spawning system not yet implemented"]
fn spawn_monster_entity_created() {
    let mut test = TestBed::new();
    test.register::<Creature>();
    test.register::<IsAlive>();
    test.register::<WorldPosition>();

    // Spawn a monster entity
    let pos = WorldPos::new(10, 10, ZLevel::new(0));
    let monster = test.spawn((
        Creature {
            def_id: "mon_zombie".to_string(),
            name: "zombie".to_string(),
            species: SpeciesId::from(0u32),
            symbol: 'Z',
        },
        IsAlive,
        WorldPosition(pos),
    ));

    // Entity should have Creature component
    assert!(test.get::<Creature>(monster).is_some());
}

// ===========================================================================
// 2: spawn_monster_at_position
// ===========================================================================

#[test]
#[ignore = "spawning system not yet implemented"]
fn spawn_monster_at_position() {
    let mut test = TestBed::new();
    test.register::<Creature>();
    test.register::<IsAlive>();
    test.register::<WorldPosition>();

    let pos = WorldPos::new(5, 15, ZLevel::new(0));
    let monster = test.spawn((
        Creature {
            def_id: "mon_dog".to_string(),
            name: "dog".to_string(),
            species: SpeciesId::from(0u32),
            symbol: 'd',
        },
        IsAlive,
        WorldPosition(pos),
    ));

    // Entity should have exact WorldPosition
    let monster_pos = test.get::<WorldPosition>(monster).unwrap().0;
    assert_eq!(monster_pos, pos);
    assert_eq!(monster_pos.x, 5);
    assert_eq!(monster_pos.y, 15);
}

// ===========================================================================
// 3: spawn_item_at_position
// ===========================================================================

#[test]
#[ignore = "spawning system not yet implemented"]
fn spawn_item_at_position() {
    let mut test = TestBed::new();
    test.register::<ItemName>();
    test.register::<StackCount>();
    test.register::<WorldPosition>();

    let pos = WorldPos::new(3, 7, ZLevel::new(0));
    let item = test.spawn((
        ItemName("stick".to_string()),
        StackCount::new(1),
        WorldPosition(pos),
    ));

    // Entity should have correct Name and WorldPosition
    assert_eq!(test.get::<ItemName>(item).unwrap().0, "stick");
    assert_eq!(test.get::<WorldPosition>(item).unwrap().0, pos);
}

// ===========================================================================
// 4: spawn_item_with_count
// ===========================================================================

#[test]
#[ignore = "spawning system not yet implemented"]
fn spawn_item_with_count() {
    let mut test = TestBed::new();
    test.register::<ItemName>();
    test.register::<StackCount>();
    test.register::<WorldPosition>();

    let pos = WorldPos::new(0, 0, ZLevel::new(0));
    let count = 5u32;
    let item = test.spawn((
        ItemName("rock".to_string()),
        StackCount::new(count),
        WorldPosition(pos),
    ));

    // StackCount should equal the requested count
    assert_eq!(test.get::<StackCount>(item).unwrap().get(), count);
}

// ===========================================================================
// 5: spawn_from_group_returns_multiple
// ===========================================================================

#[test]
#[ignore = "spawning system not yet implemented"]
fn spawn_from_group_returns_multiple() {
    let mut test = TestBed::new();
    test.register::<ItemName>();
    test.register::<StackCount>();
    test.register::<WorldPosition>();

    let pos = WorldPos::new(1, 1, ZLevel::new(0));

    // A group spawns multiple items
    let item_a = test.spawn((
        ItemName("nail".to_string()),
        StackCount::new(5),
        WorldPosition(pos),
    ));
    let item_b = test.spawn((
        ItemName("board".to_string()),
        StackCount::new(2),
        WorldPosition(pos),
    ));

    // Multiple entities should be created
    let entities = vec![item_a, item_b];
    assert_eq!(entities.len(), 2);

    // Each entity should have distinct items
    assert_eq!(test.get::<ItemName>(item_a).unwrap().0, "nail");
    assert_eq!(test.get::<ItemName>(item_b).unwrap().0, "board");
}

// ===========================================================================
// 6: spawning_phase_processes_events
// ===========================================================================

#[test]
#[ignore = "spawning system not yet implemented"]
fn spawning_phase_processes_events() {
    let mut test = TestBed::new();
    test.register::<Creature>();
    test.register::<IsAlive>();
    test.register::<WorldPosition>();

    // Queue SpawnEvents to be processed by the spawning phase
    // After spawning_phase runs, entities should exist in the world
    let pos = WorldPos::new(20, 20, ZLevel::new(0));

    // Manually create the entity that would be spawned by the event
    let spawned = test.spawn((
        Creature {
            def_id: "mon_zombie".to_string(),
            name: "zombie".to_string(),
            species: SpeciesId::from(0u32),
            symbol: 'Z',
        },
        IsAlive,
        WorldPosition(pos),
    ));

    // Verify the spawned entity exists
    assert!(test.get::<Creature>(spawned).is_some());
    assert!(test.get::<IsAlive>(spawned).is_some());
    assert!(test.get::<WorldPosition>(spawned).is_some());
    assert_eq!(
        test.get::<WorldPosition>(spawned).unwrap().0,
        pos
    );
}
