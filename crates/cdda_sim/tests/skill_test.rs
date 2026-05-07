//! Tests for [`SkillSet`] and [`SkillLevel`] — creature skill tracking.
//!
//! Skills are stored in a [`HashMap<SkillId, SkillLevel>`] on the [`SkillSet`] component.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use cdda_actor::components::{SkillLevel, SkillSet};
use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Empty / basic creation
// ---------------------------------------------------------------------------

#[test]
fn skill_set_empty() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let e = test.spawn((SkillSet {
        skills: HashMap::new(),
    },));
    let s = test.get::<SkillSet>(e).unwrap();
    assert!(s.skills.is_empty());
}

#[test]
fn skill_set_one_skill() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let mut skills = HashMap::new();
    skills.insert(
        cdda_core::SkillId::from(0u32),
        SkillLevel {
            level: 5,
            experience: 1000,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    let s = test.get::<SkillSet>(e).unwrap();
    assert_eq!(s.skills.len(), 1);
    let sl = s.skills.get(&cdda_core::SkillId::from(0u32)).unwrap();
    assert_eq!(sl.level, 5);
    assert_eq!(sl.experience, 1000);
}

#[test]
fn skill_set_multiple() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let mut skills = HashMap::new();
    skills.insert(
        cdda_core::SkillId::from(0u32),
        SkillLevel {
            level: 3,
            experience: 500,
        },
    );
    skills.insert(
        cdda_core::SkillId::from(1u32),
        SkillLevel {
            level: 5,
            experience: 1000,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    let s = test.get::<SkillSet>(e).unwrap();
    assert_eq!(s.skills.len(), 2);
}

// ---------------------------------------------------------------------------
// SkillLevel — direct struct access
// ---------------------------------------------------------------------------

#[test]
fn skill_level_access() {
    let sl = SkillLevel {
        level: 7,
        experience: 3000,
    };
    assert_eq!(sl.level, 7);
    assert_eq!(sl.experience, 3000);
}

#[test]
fn skill_level_zero() {
    let sl = SkillLevel {
        level: 0,
        experience: 0,
    };
    assert_eq!(sl.level, 0);
    assert_eq!(sl.experience, 0);
}

#[test]
fn skill_level_high() {
    let sl = SkillLevel {
        level: 10,
        experience: 8000,
    };
    assert_eq!(sl.level, 10);
    assert_eq!(sl.experience, 8000);
}

#[test]
fn skill_level_max() {
    let sl = SkillLevel {
        level: 20,
        experience: 0,
    };
    assert_eq!(sl.level, 20);
}

// ---------------------------------------------------------------------------
// HashMap get / get_mut / missing / clear
// ---------------------------------------------------------------------------

#[test]
fn skill_set_get_skill() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let melee_id = cdda_core::SkillId::from(0u32);
    let mut skills = HashMap::new();
    skills.insert(
        melee_id,
        SkillLevel {
            level: 5,
            experience: 1000,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    let s = test.get::<SkillSet>(e).unwrap();
    let retrieved = s.skills.get(&melee_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().level, 5);
}

#[test]
fn skill_set_update_level() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let skill_id = cdda_core::SkillId::from(0u32);
    let mut skills = HashMap::new();
    skills.insert(
        skill_id,
        SkillLevel {
            level: 3,
            experience: 500,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    {
        let mut s = test.world_mut().get_mut::<SkillSet>(e).unwrap();
        if let Some(sl) = s.skills.get_mut(&skill_id) {
            sl.level = 7;
        }
    }
    let s = test.get::<SkillSet>(e).unwrap();
    assert_eq!(s.skills.get(&skill_id).unwrap().level, 7);
}

#[test]
fn skill_set_missing_skill() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let mut skills = HashMap::new();
    skills.insert(
        cdda_core::SkillId::from(0u32),
        SkillLevel {
            level: 5,
            experience: 1000,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    let s = test.get::<SkillSet>(e).unwrap();
    let missing = s.skills.get(&cdda_core::SkillId::from(99u32));
    assert!(missing.is_none());
}

#[test]
fn skill_set_clear() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let mut skills = HashMap::new();
    skills.insert(
        cdda_core::SkillId::from(0u32),
        SkillLevel {
            level: 5,
            experience: 1000,
        },
    );
    skills.insert(
        cdda_core::SkillId::from(1u32),
        SkillLevel {
            level: 3,
            experience: 500,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    {
        let mut s = test.world_mut().get_mut::<SkillSet>(e).unwrap();
        s.skills.clear();
    }
    let s = test.get::<SkillSet>(e).unwrap();
    assert!(s.skills.is_empty());
}

// ---------------------------------------------------------------------------
// Experience — stored separately per skill
// ---------------------------------------------------------------------------

#[test]
fn skill_experience_independent() {
    let mut test = TestBed::new();
    test.register::<SkillSet>();

    let mut skills = HashMap::new();
    skills.insert(
        cdda_core::SkillId::from(0u32),
        SkillLevel {
            level: 3,
            experience: 500,
        },
    );
    skills.insert(
        cdda_core::SkillId::from(1u32),
        SkillLevel {
            level: 5,
            experience: 8000,
        },
    );
    let e = test.spawn((SkillSet { skills },));
    let s = test.get::<SkillSet>(e).unwrap();
    let sl0 = s
        .skills
        .get(&cdda_core::SkillId::from(0u32))
        .unwrap();
    let sl1 = s
        .skills
        .get(&cdda_core::SkillId::from(1u32))
        .unwrap();
    assert_eq!(sl0.experience, 500);
    assert_eq!(sl1.experience, 8000);
    assert_ne!(sl0.experience, sl1.experience);
}
