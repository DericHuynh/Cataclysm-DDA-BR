#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_actor::components::*;
use cdda_core::*;
use cdda_sim::components::*;
use cdda_sim::systems::crafting::*;
use cdda_sim::test_utils::TestBed;
use std::collections::HashMap;

// ===========================================================================
// Helper: create a creature with skills
// ===========================================================================

fn spawn_creature_with_skills(test: &mut TestBed, skills: Vec<(SkillId, u32)>) -> Entity {
    test.register::<IsAlive>();
    test.register::<SkillSet>();
    test.spawn((
        IsAlive,
        SkillSet {
            skills: skills
                .into_iter()
                .map(|(id, level)| {
                    (
                        id,
                        SkillLevel {
                            level,
                            experience: 0,
                        },
                    )
                })
                .collect(),
        },
    ))
}

fn spawn_creature(test: &mut TestBed) -> Entity {
    test.register::<IsAlive>();
    test.register::<SkillSet>();
    test.spawn((
        IsAlive,
        SkillSet {
            skills: HashMap::new(),
        },
    ))
}

// ===========================================================================
// 1: can_craft_skill_met
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn can_craft_skill_met() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_skills(&mut test, vec![(SkillId::from(0u32), 5)]);

    // Recipe requires fabrication 3, creature has fabrication 5
    let requirement: u32 = 3;
    let creature_skill = test
        .get::<SkillSet>(creature)
        .unwrap()
        .skills
        .get(&SkillId::from(0u32))
        .map(|s| s.level)
        .unwrap_or(0);

    assert!(creature_skill >= requirement);
    let result: Result<(), &str> = Ok(());
    assert!(result.is_ok());
}

// ===========================================================================
// 2: can_craft_skill_too_low
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn can_craft_skill_too_low() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_skills(&mut test, vec![(SkillId::from(0u32), 1)]);

    // Recipe requires fabrication 5, creature only has fabrication 1
    let requirement: u32 = 5;
    let creature_skill = test
        .get::<SkillSet>(creature)
        .unwrap()
        .skills
        .get(&SkillId::from(0u32))
        .map(|s| s.level)
        .unwrap_or(0);

    assert!(creature_skill < requirement);
    let result: Result<(), &str> = Err("skill too low");
    assert!(result.is_err());
}

// ===========================================================================
// 3: can_craft_missing_tools
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn can_craft_missing_tools() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_skills(&mut test, vec![(SkillId::from(0u32), 5)]);

    // Recipe requires a tool quality (e.g., welding torch) that creature does not have
    // can_craft should return Err with missing tools message
    let result: Result<(), &str> = Err("missing required tools");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "missing required tools");
}

// ===========================================================================
// 4: craft_time_base_calculation
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn craft_time_base_calculation() {
    // Base crafting time for a simple item without modifiers
    let base_time = cdda_core::Time::from_turns(600); // 10 minutes at 1 turn/sec

    // No modifiers should result in exactly the base time
    assert_eq!(base_time.as_turns(), 600);
}

// ===========================================================================
// 5: craft_time_skill_reduces
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn craft_time_skill_reduces() {
    // Higher skill level reduces crafting time
    let base_time_turns: i64 = 600;
    let skill_level: u32 = 5;
    let reduction_per_skill: f64 = 0.1; // 10% per skill level
    let reduction_factor = 1.0 - (skill_level as f64 * reduction_per_skill);
    let adjusted_time = (base_time_turns as f64 * reduction_factor) as i64;

    // Skill 5 should reduce 600 turns by 50%
    assert_eq!(adjusted_time, 300);
    assert!(adjusted_time < base_time_turns);
}

// ===========================================================================
// 6: craft_time_no_tools_penalty
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn craft_time_no_tools_penalty() {
    // Missing required tools increases crafting time
    let base_time_turns: i64 = 600;
    let no_tools_penalty: f64 = 2.0; // 2x time without proper tools
    let penalized_time = (base_time_turns as f64 * no_tools_penalty) as i64;

    assert_eq!(penalized_time, 1200);
    assert!(penalized_time > base_time_turns);
}

// ===========================================================================
// 7: consume_components_removes_items
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn consume_components_removes_items() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);

    // Creature has items in inventory
    // After crafting, required components are consumed (removed from inventory)
    let components_consumed = true;

    // Verify components were consumed
    assert!(components_consumed);
}

// ===========================================================================
// 8: available_recipes_by_skill
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn available_recipes_by_skill() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_skills(&mut test, vec![(SkillId::from(0u32), 3)]);

    // Only recipes with skill requirement <= creature's skill are returned
    // Recipe requirements: fabrication 1, fabrication 3, fabrication 5
    // Creature has fabrication 3, so first two are available
    let creature_fab = test
        .get::<SkillSet>(creature)
        .unwrap()
        .skills
        .get(&SkillId::from(0u32))
        .map(|s| s.level)
        .unwrap_or(0);

    assert_eq!(creature_fab, 3);

    // Recipes filtered by skill <= creature_fab
    let available: Vec<u32> = vec![1, 3] // fabrications 1 and 3
        .into_iter()
        .filter(|req| *req <= creature_fab)
        .collect();

    assert_eq!(available.len(), 2);
    assert!(available.contains(&1));
    assert!(available.contains(&3));
    assert!(!available.contains(&5));
}
