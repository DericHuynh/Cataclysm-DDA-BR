//! Tests for [`MutationOf`], [`CreatureMutations`], and [`MutationEntry`].
//!
//! Mutations are relationship-based: each active mutation is a separate entity
//! with a [`MutationOf(creature)`] relationship, enabling independent addition,
//! removal, and querying without touching the creature entity.

use bevy_ecs::prelude::*;
use cdda_actor::components::{CreatureMutations, MutationEntry, MutationOf};
use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mutations_for(test: &mut TestBed, creature: Entity) -> Vec<MutationEntry> {
    let mut q = test.world_mut().query::<(&MutationOf, &MutationEntry)>();
    q.iter(test.world())
        .filter(|(m_of, _)| m_of.0 == creature)
        .map(|(_, entry)| entry.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Empty / basic creation
// ---------------------------------------------------------------------------

#[test]
fn mutations_empty() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    assert!(mutations_for(&mut test, creature).is_empty());
}

#[test]
fn mutations_single() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(0u32), visible: true },
    ));

    let mutations = mutations_for(&mut test, creature);
    assert_eq!(mutations.len(), 1);
    assert!(mutations[0].visible);
}

#[test]
fn mutations_multiple() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(0u32), visible: true },
    ));
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(1u32), visible: false },
    ));

    assert_eq!(mutations_for(&mut test, creature).len(), 2);
}

// ---------------------------------------------------------------------------
// Visibility flag
// ---------------------------------------------------------------------------

#[test]
fn mutations_visible_flag() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(0u32), visible: true },
    ));
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(1u32), visible: false },
    ));

    let mutations = mutations_for(&mut test, creature);
    let visible_count = mutations.iter().filter(|m| m.visible).count();
    let hidden_count = mutations.iter().filter(|m| !m.visible).count();
    assert_eq!(visible_count, 1);
    assert_eq!(hidden_count, 1);
}

// ---------------------------------------------------------------------------
// Add / remove
// ---------------------------------------------------------------------------

#[test]
fn mutations_add_one() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(0u32), visible: true },
    ));

    // Add a second mutation
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(1u32), visible: false },
    ));

    assert_eq!(mutations_for(&mut test, creature).len(), 2);
}

#[test]
fn mutations_remove_one() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(0u32), visible: true },
    ));
    let hidden = test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(1u32), visible: false },
    ));

    // Remove the hidden mutation
    test.world_mut().despawn(hidden);

    let mutations = mutations_for(&mut test, creature);
    assert_eq!(mutations.len(), 1);
    assert_eq!(mutations[0].id, cdda_core::MutationId::from(0u32));
}

#[test]
fn mutations_retain_order() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    let a_id = cdda_core::MutationId::from(0u32);
    let b_id = cdda_core::MutationId::from(1u32);
    let c_id = cdda_core::MutationId::from(2u32);
    let d_id = cdda_core::MutationId::from(3u32);

    test.spawn((MutationOf(creature), MutationEntry { id: a_id, visible: true }));
    test.spawn((MutationOf(creature), MutationEntry { id: b_id, visible: true }));
    test.spawn((MutationOf(creature), MutationEntry { id: c_id, visible: true }));
    test.spawn((MutationOf(creature), MutationEntry { id: d_id, visible: true }));

    let ids: Vec<_> = mutations_for(&mut test, creature).into_iter().map(|m| m.id).collect();
    assert!(ids.contains(&a_id));
    assert!(ids.contains(&b_id));
    assert!(ids.contains(&c_id));
    assert!(ids.contains(&d_id));
}

// ---------------------------------------------------------------------------
// Mutation id storage
// ---------------------------------------------------------------------------

#[test]
fn mutation_id_stored() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    let id = cdda_core::MutationId::from(42u32);
    test.spawn((
        MutationOf(creature),
        MutationEntry { id, visible: false },
    ));

    let mutations = mutations_for(&mut test, creature);
    assert_eq!(mutations[0].id, cdda_core::MutationId::from(42u32));
}

#[test]
fn mutations_no_duplicate_check() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    // ECS does not enforce uniqueness — callers must prevent duplicates.
    let creature = test.spawn(());
    let id = cdda_core::MutationId::from(5u32);
    test.spawn((MutationOf(creature), MutationEntry { id, visible: true }));
    test.spawn((MutationOf(creature), MutationEntry { id, visible: true }));

    assert_eq!(mutations_for(&mut test, creature).len(), 2);
}

#[test]
fn mutations_replace_all() {
    let mut test = TestBed::new();
    test.register::<MutationOf>()
        .register::<CreatureMutations>()
        .register::<MutationEntry>();

    let creature = test.spawn(());
    let old = test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(0u32), visible: true },
    ));

    // Replace: despawn old, spawn new set
    test.world_mut().despawn(old);
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(10u32), visible: false },
    ));
    test.spawn((
        MutationOf(creature),
        MutationEntry { id: cdda_core::MutationId::from(11u32), visible: true },
    ));

    let mutations = mutations_for(&mut test, creature);
    assert_eq!(mutations.len(), 2);
    let ids: Vec<_> = mutations.iter().map(|m| m.id).collect();
    assert!(ids.contains(&cdda_core::MutationId::from(10u32)));
    assert!(ids.contains(&cdda_core::MutationId::from(11u32)));
}
