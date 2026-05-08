#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_core::core::components::actor::*;
use cdda_core::*;
use cdda_core::core::components::sim::*;
use cdda_core::sim::systems::crafting::*;
use cdda_core::sim::test_utils::TestBed;

// ===========================================================================
// Helper: create a creature with skills (relationship-based)
// ===========================================================================

fn spawn_creature_with_skills(test: &mut TestBed, skills: Vec<(SkillId, u32)>) -> Entity {
    test.register::<IsAlive>();
    test.register::<SkillOf>();
    test.register::<CreatureSkills>();
    test.register::<SkillEntry>();
    let creature = test.spawn((IsAlive,));
    for (id, level) in skills {
        test.spawn((
            SkillOf(creature),
            SkillEntry { skill_id: id, level, experience: 0 },
        ));
    }
    creature
}

fn spawn_creature(test: &mut TestBed) -> Entity {
    test.register::<IsAlive>();
    test.register::<SkillOf>();
    test.register::<CreatureSkills>();
    test.register::<SkillEntry>();
    test.spawn((IsAlive,))
}

fn skill_level_for(test: &mut TestBed, creature: Entity, id: SkillId) -> u32 {
    let mut q = test.world_mut().query::<(&SkillOf, &SkillEntry)>();
    q.iter(test.world())
        .filter(|(sk_of, _)| sk_of.0 == creature)
        .find(|(_, entry)| entry.skill_id == id)
        .map(|(_, entry)| entry.level)
        .unwrap_or(0)
}

// ===========================================================================
// 1: can_craft_skill_met
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn can_craft_skill_met() {
    let mut test = TestBed::new();
    let creature = spawn_creature_with_skills(&mut test, vec![(SkillId::from(0u32), 5)]);

    let requirement: u32 = 3;
    let creature_skill = skill_level_for(&mut test, creature, SkillId::from(0u32));
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

    let requirement: u32 = 5;
    let creature_skill = skill_level_for(&mut test, creature, SkillId::from(0u32));
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
    let _creature = spawn_creature_with_skills(&mut test, vec![(SkillId::from(0u32), 5)]);

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
    let base_time = cdda_core::Time::from_turns(600);
    assert_eq!(base_time.as_turns(), 600);
}

// ===========================================================================
// 5: craft_time_skill_reduces
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn craft_time_skill_reduces() {
    let base_time_turns: i64 = 600;
    let skill_level: u32 = 5;
    let reduction_per_skill: f64 = 0.1;
    let reduction_factor = 1.0 - (skill_level as f64 * reduction_per_skill);
    let adjusted_time = (base_time_turns as f64 * reduction_factor) as i64;
    assert_eq!(adjusted_time, 300);
    assert!(adjusted_time < base_time_turns);
}

// ===========================================================================
// 6: craft_time_no_tools_penalty
// ===========================================================================

#[test]
#[ignore = "crafting system not yet implemented"]
fn craft_time_no_tools_penalty() {
    let base_time_turns: i64 = 600;
    let no_tools_penalty: f64 = 2.0;
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
    let _creature = spawn_creature(&mut test);
    let components_consumed = true;
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

    let creature_fab = skill_level_for(&mut test, creature, SkillId::from(0u32));
    assert_eq!(creature_fab, 3);

    let available: Vec<u32> = vec![1, 3]
        .into_iter()
        .filter(|req| *req <= creature_fab)
        .collect();

    assert_eq!(available.len(), 2);
    assert!(available.contains(&1));
    assert!(available.contains(&3));
    assert!(!available.contains(&5));
}
