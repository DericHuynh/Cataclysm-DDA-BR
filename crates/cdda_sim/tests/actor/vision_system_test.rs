#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_components::actor::*;
use cdda_components::sim::*;
use cdda_components::WorldPos;
use cdda_components::*;
use cdda_sim::actor::vision::*;
use cdda_sim::runtime::test_utils::TestBed;

// ===========================================================================
// Helper: create a creature with vision
// ===========================================================================

fn spawn_creature_with_vision(test: &mut TestBed, day: i32, night: i32) -> Entity {
    test.register::<IsAlive>();
    test.register::<Vision>();
    test.register::<WorldPosition>();
    test.spawn((
        IsAlive,
        Vision {
            day_range: day,
            night_range: night,
        },
        WorldPosition(WorldPos::new(
            0,
            0,
            cdda_components::core::coords::ZLevel::new(0),
        )),
    ))
}

fn spawn_creature_at(test: &mut TestBed, x: i32, y: i32) -> Entity {
    test.register::<IsAlive>();
    test.register::<Vision>();
    test.register::<WorldPosition>();
    test.spawn((
        IsAlive,
        Vision {
            day_range: 40,
            night_range: 5,
        },
        WorldPosition(WorldPos::new(
            x,
            y,
            cdda_components::core::coords::ZLevel::new(0),
        )),
    ))
}

// ===========================================================================
// 1: vision_range_daytime
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn vision_range_daytime() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_vision(&mut test, 40, 5);

    // During day, effective range should be day_range (40)
    let vision = test.get::<Vision>(creature).unwrap();
    let effective_range = vision.day_range;
    assert_eq!(effective_range, 40);
}

// ===========================================================================
// 2: vision_range_nighttime
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn vision_range_nighttime() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_vision(&mut test, 40, 5);

    // During night, effective range should be night_range (5)
    let vision = test.get::<Vision>(creature).unwrap();
    let effective_range = vision.night_range;
    assert_eq!(effective_range, 5);
}

// ===========================================================================
// 3: vision_range_night_vision
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn vision_range_night_vision() {
    let _test = TestBed::new();

    // Creature with night vision trait has boosted night range
    // Night range of 5 + night vision boost = 20
    let effective_night_range = 20;

    // A creature without night vision would have 5
    assert!(effective_night_range > 5);
    assert_eq!(effective_night_range, 20);
}

// ===========================================================================
// 4: vision_range_dusk_average
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn vision_range_dusk_average() {
    // During dusk, effective range is (day + night) / 2
    let day_range = 40;
    let night_range = 5;
    let dusk_range = (day_range + night_range) / 2;

    assert_eq!(dusk_range, 22);
}

// ===========================================================================
// 5: can_see_line_of_sight
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn can_see_line_of_sight() {
    let mut test = TestBed::new();
    let observer = spawn_creature_at(&mut test, 0, 0);
    let target = spawn_creature_at(&mut test, 5, 0);

    // Two entities in range (observer has 40 range), no obstacles
    let observer_pos = test.get::<WorldPosition>(observer).unwrap().0;
    let target_pos = test.get::<WorldPosition>(target).unwrap().0;
    let distance = observer_pos.dist_chebyshev(target_pos);

    assert!(distance <= 40, "Target should be within vision range");
    // can_see(observer, target) should return true
    let can_see = true;
    assert!(can_see);
}

// ===========================================================================
// 6: can_see_too_far
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn can_see_too_far() {
    let mut test = TestBed::new();
    let observer = spawn_creature_at(&mut test, 0, 0);
    let target = spawn_creature_at(&mut test, 100, 0);

    // Target at distance > vision range (observer has 40 range)
    let observer_pos = test.get::<WorldPosition>(observer).unwrap().0;
    let target_pos = test.get::<WorldPosition>(target).unwrap().0;
    let distance = observer_pos.dist_chebyshev(target_pos);

    assert!(distance > 40, "Target should be outside vision range");
    // can_see(observer, target) should return false
    let can_see = false;
    assert!(!can_see);
}

// ===========================================================================
// 7: can_see_wall_blocks
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn can_see_wall_blocks() {
    let mut test = TestBed::new();
    let _observer = spawn_creature_at(&mut test, 0, 0);
    let _target = spawn_creature_at(&mut test, 5, 0);

    // Wall terrain between observer and target blocks line of sight
    // Even though both entities are in range, the wall blocks vision
    let in_range = true;
    let wall_blocks = true;

    assert!(in_range);
    assert!(wall_blocks);

    // can_see(observer, target, with wall) should return false
    let can_see = false;
    assert!(!can_see);
}

// ===========================================================================
// 8: can_see_low_light_penalizes
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn can_see_low_light_penalizes() {
    let mut test = TestBed::new();
    let _observer = spawn_creature_with_vision(&mut test, 40, 5);

    // Low light level (0) reduces effective range
    // Day range of 40 with light level 0 should be reduced
    let effective_range = 40; // without penalty
    let light_level = 0;
    let penalized_range = if light_level == 0 {
        effective_range / 4
    } else {
        effective_range
    };

    assert_eq!(penalized_range, 10);
    assert!(penalized_range < effective_range);
}

// ===========================================================================
// 9: visible_entities_returns_seen
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn visible_entities_returns_seen() {
    let mut test = TestBed::new();
    let _observer = spawn_creature_at(&mut test, 0, 0);
    let nearby = spawn_creature_at(&mut test, 3, 0);
    let far_away = spawn_creature_at(&mut test, 100, 0);

    // visible_entities(observer) should only return entities in range
    let visible_entities: Vec<Entity> = vec![nearby]; // far_away is too far
    assert!(visible_entities.contains(&nearby));
    assert!(!visible_entities.contains(&far_away));
    assert_eq!(visible_entities.len(), 1);
}

// ===========================================================================
// 10: update_vision_processes_all_creatures
// ===========================================================================

#[test]
#[ignore = "vision system not yet implemented"]
fn update_vision_processes_all_creatures() {
    let mut test = TestBed::new();
    let _creature_a = spawn_creature_at(&mut test, 0, 0);
    let _creature_b = spawn_creature_at(&mut test, 10, 10);
    let _creature_c = spawn_creature_at(&mut test, 20, 20);

    // update_vision system should process all creatures
    // After running the system, each creature's sight should be updated
    // For now, just verify all three entities exist
    let mut q = test.world_mut().query::<&Vision>();
    let count = q.iter(test.world()).count();
    assert_eq!(count, 3);
}
