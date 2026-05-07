//! Tests for [`Mutations`] and [`MutationState`] — creature mutation tracking.
//!
//! Mutations are stored in a [`Vec<MutationState>`] on the [`Mutations`] component.
//! Each mutation has an id and a visible flag.

use bevy_ecs::prelude::*;
use cdda_actor::components::{Mutations, MutationState};
use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Empty / basic creation
// ---------------------------------------------------------------------------

#[test]
fn mutations_empty() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: Vec::new(),
    },));
    let m = test.get::<Mutations>(e).unwrap();
    assert!(m.active.is_empty());
}

#[test]
fn mutations_single() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: vec![MutationState {
            id: cdda_core::MutationId::from(0u32),
            visible: true,
        }],
    },));
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active.len(), 1);
    assert!(m.active[0].visible);
}

#[test]
fn mutations_multiple() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: vec![
            MutationState {
                id: cdda_core::MutationId::from(0u32),
                visible: true,
            },
            MutationState {
                id: cdda_core::MutationId::from(1u32),
                visible: false,
            },
        ],
    },));
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active.len(), 2);
}

// ---------------------------------------------------------------------------
// Visibility flag
// ---------------------------------------------------------------------------

#[test]
fn mutations_visible_flag() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: vec![
            MutationState {
                id: cdda_core::MutationId::from(0u32),
                visible: true,
            },
            MutationState {
                id: cdda_core::MutationId::from(1u32),
                visible: false,
            },
        ],
    },));
    let m = test.get::<Mutations>(e).unwrap();
    assert!(m.active[0].visible);
    assert!(!m.active[1].visible);
}

// ---------------------------------------------------------------------------
// Mutation — add / remove / retain
// ---------------------------------------------------------------------------

#[test]
fn mutations_add_one() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: vec![MutationState {
            id: cdda_core::MutationId::from(0u32),
            visible: true,
        }],
    },));
    {
        let mut m = test.world_mut().get_mut::<Mutations>(e).unwrap();
        m.active.push(MutationState {
            id: cdda_core::MutationId::from(1u32),
            visible: false,
        });
    }
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active.len(), 2);
    assert_eq!(m.active[1].id, cdda_core::MutationId::from(1u32));
}

#[test]
fn mutations_remove_one() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: vec![
            MutationState {
                id: cdda_core::MutationId::from(0u32),
                visible: true,
            },
            MutationState {
                id: cdda_core::MutationId::from(1u32),
                visible: false,
            },
        ],
    },));
    {
        let mut m = test.world_mut().get_mut::<Mutations>(e).unwrap();
        m.active.retain(|s| s.visible);
    }
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active.len(), 1);
    assert_eq!(m.active[0].id, cdda_core::MutationId::from(0u32));
}

#[test]
fn mutations_retain_order() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let a_id = cdda_core::MutationId::from(0u32);
    let b_id = cdda_core::MutationId::from(1u32);
    let c_id = cdda_core::MutationId::from(2u32);

    let e = test.spawn((Mutations {
        active: vec![
            MutationState {
                id: a_id,
                visible: true,
            },
            MutationState {
                id: b_id,
                visible: true,
            },
            MutationState {
                id: c_id,
                visible: true,
            },
        ],
    },));
    {
        let mut m = test.world_mut().get_mut::<Mutations>(e).unwrap();
        m.active.push(MutationState {
            id: cdda_core::MutationId::from(3u32),
            visible: true,
        });
    }
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active[0].id, a_id);
    assert_eq!(m.active[1].id, b_id);
    assert_eq!(m.active[2].id, c_id);
    assert_eq!(m.active[3].id, cdda_core::MutationId::from(3u32));
}

// ---------------------------------------------------------------------------
// Mutation id storage behaviour
// ---------------------------------------------------------------------------

#[test]
fn mutation_id_stored() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let id = cdda_core::MutationId::from(42u32);
    let e = test.spawn((Mutations {
        active: vec![MutationState { id, visible: false }],
    },));
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active[0].id, cdda_core::MutationId::from(42u32));
}

#[test]
fn mutations_no_duplicate_check() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let id = cdda_core::MutationId::from(5u32);
    let e = test.spawn((Mutations {
        active: vec![
            MutationState { id, visible: true },
            MutationState { id, visible: true },
        ],
    },));
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active.len(), 2);
    assert_eq!(m.active[0].id, m.active[1].id);
}

#[test]
fn mutations_replace_all() {
    let mut test = TestBed::new();
    test.register::<Mutations>();

    let e = test.spawn((Mutations {
        active: vec![MutationState {
            id: cdda_core::MutationId::from(0u32),
            visible: true,
        }],
    },));
    {
        let mut m = test.world_mut().get_mut::<Mutations>(e).unwrap();
        m.active = vec![
            MutationState {
                id: cdda_core::MutationId::from(10u32),
                visible: false,
            },
            MutationState {
                id: cdda_core::MutationId::from(11u32),
                visible: true,
            },
        ];
    }
    let m = test.get::<Mutations>(e).unwrap();
    assert_eq!(m.active.len(), 2);
    assert_eq!(m.active[0].id, cdda_core::MutationId::from(10u32));
    assert_eq!(m.active[1].id, cdda_core::MutationId::from(11u32));
    assert!(!m.active[0].visible);
    assert!(m.active[1].visible);
}
