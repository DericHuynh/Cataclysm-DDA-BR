//! Tests for [`SkillOf`], [`CreatureSkills`], and [`SkillEntry`].
//!
//! Skills are relationship-based: each skill is a separate entity with a
//! [`SkillOf(creature)`] relationship component, so they can be independently
//! queried, modified, or deleted without touching the creature entity.

use bevy_ecs::prelude::*;
use cdda_core::actor::components::{CreatureSkills, SkillEntry, SkillOf};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return all `SkillEntry` components that belong to `creature`.
fn skills_for(test: &mut TestBed, creature: Entity) -> Vec<SkillEntry> {
    let mut q = test.world_mut().query::<(&SkillOf, &SkillEntry)>();
    q.iter(test.world())
        .filter(|(sk_of, _)| sk_of.0 == creature)
        .map(|(_, entry)| entry.clone())
        .collect()
}

fn skill_level_for(test: &mut TestBed, creature: Entity, id: cdda_core::SkillId) -> Option<u32> {
    skills_for(test, creature)
        .into_iter()
        .find(|e| e.skill_id == id)
        .map(|e| e.level)
}

// ---------------------------------------------------------------------------
// Empty / basic creation
// ---------------------------------------------------------------------------

#[test]
fn skill_set_empty() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    let count = skills_for(&mut test, creature).len();
    assert_eq!(count, 0);
}

#[test]
fn skill_set_one_skill() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    let _skill = test.spawn((
        SkillOf(creature),
        SkillEntry {
            skill_id: cdda_core::SkillId::from(0u32),
            level: 5,
            experience: 1000,
        },
    ));

    let skills = skills_for(&mut test, creature);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].level, 5);
    assert_eq!(skills[0].experience, 1000);
}

#[test]
fn skill_set_multiple() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 3, experience: 500 },
    ));
    test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(1u32), level: 5, experience: 1000 },
    ));

    assert_eq!(skills_for(&mut test, creature).len(), 2);
}

// ---------------------------------------------------------------------------
// SkillEntry — direct struct access
// ---------------------------------------------------------------------------

#[test]
fn skill_entry_access() {
    let entry = SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 7, experience: 3000 };
    assert_eq!(entry.level, 7);
    assert_eq!(entry.experience, 3000);
}

#[test]
fn skill_entry_zero() {
    let entry = SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 0, experience: 0 };
    assert_eq!(entry.level, 0);
    assert_eq!(entry.experience, 0);
}

#[test]
fn skill_entry_high() {
    let entry = SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 10, experience: 8000 };
    assert_eq!(entry.level, 10);
    assert_eq!(entry.experience, 8000);
}

#[test]
fn skill_entry_max() {
    let entry = SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 20, experience: 0 };
    assert_eq!(entry.level, 20);
}

// ---------------------------------------------------------------------------
// Query get / missing / clear
// ---------------------------------------------------------------------------

#[test]
fn skill_set_get_skill() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    let melee_id = cdda_core::SkillId::from(0u32);
    test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: melee_id, level: 5, experience: 1000 },
    ));

    let level = skill_level_for(&mut test, creature, melee_id);
    assert!(level.is_some());
    assert_eq!(level.unwrap(), 5);
}

#[test]
fn skill_set_update_level() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    let skill_id = cdda_core::SkillId::from(0u32);
    let skill_entity = test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id, level: 3, experience: 500 },
    ));

    // Update level by reinserting the component (standard Bevy pattern)
    test.world_mut().entity_mut(skill_entity).insert(SkillEntry {
        skill_id,
        level: 7,
        experience: 500,
    });

    assert_eq!(skill_level_for(&mut test, creature, skill_id).unwrap(), 7);
}

#[test]
fn skill_set_missing_skill() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 5, experience: 1000 },
    ));

    let missing = skill_level_for(&mut test, creature, cdda_core::SkillId::from(99u32));
    assert!(missing.is_none());
}

#[test]
fn skill_set_clear() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    let s0 = test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 5, experience: 1000 },
    ));
    let s1 = test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(1u32), level: 3, experience: 500 },
    ));

    // "Clear" = despawn all skill entities for this creature
    test.world_mut().despawn(s0);
    test.world_mut().despawn(s1);

    assert_eq!(skills_for(&mut test, creature).len(), 0);
}

// ---------------------------------------------------------------------------
// Experience — stored per skill entity
// ---------------------------------------------------------------------------

#[test]
fn skill_experience_independent() {
    let mut test = TestBed::new();
    test.register::<SkillOf>()
        .register::<CreatureSkills>()
        .register::<SkillEntry>();

    let creature = test.spawn(());
    test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(0u32), level: 3, experience: 500 },
    ));
    test.spawn((
        SkillOf(creature),
        SkillEntry { skill_id: cdda_core::SkillId::from(1u32), level: 5, experience: 8000 },
    ));

    let skills = skills_for(&mut test, creature);
    let e0 = skills.iter().find(|e| e.skill_id == cdda_core::SkillId::from(0u32)).unwrap();
    let e1 = skills.iter().find(|e| e.skill_id == cdda_core::SkillId::from(1u32)).unwrap();
    assert_eq!(e0.experience, 500);
    assert_eq!(e1.experience, 8000);
    assert_ne!(e0.experience, e1.experience);
}
