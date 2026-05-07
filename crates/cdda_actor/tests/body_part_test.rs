//! Integration tests for actor body parts: relationships, HP, and status markers.
//!
//! Tests cover:
//! - `BodyPartSlot`, `BodyPartDef`, `BodyPartHp` field creation and mutation
//! - `BodyPartOf`/`CreatureBodyParts` relationship (auto-population, iteration)
//! - HP damage, damage multipliers
//! - `BodyPartBroken` and `BodyPartSevered` tag markers (independent)
//! - Removal of body parts and empty creature scenarios
//!
//! All tests use `TestBed` from `cdda_sim::test_utils`.

use bevy_ecs::entity::Entity;
use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Spawn a definition entity for a body part (e.g. "torso_def").
fn spawn_def(test: &mut TestBed, name: &str) -> Entity {
    test.spawn((cdda_sim::def_components::BodyPartName(name.to_string()),))
}

/// Spawn a body part entity linked to a creature.
fn spawn_body_part(test: &mut TestBed, creature: Entity, def_entity: Entity, slot: &str) -> Entity {
    test.register::<cdda_actor::components::BodyPartOf>();
    test.register::<cdda_actor::components::BodyPartDef>();
    test.register::<cdda_actor::components::BodyPartSlot>();
    test.register::<cdda_actor::components::BodyPartHp>();
    test.spawn((
        cdda_actor::components::BodyPartOf(creature),
        cdda_actor::components::BodyPartDef(def_entity),
        cdda_actor::components::BodyPartSlot(slot.to_string()),
        cdda_actor::components::BodyPartHp {
            max: 100.0,
            current: 100.0,
            damage_multiplier: 1.0,
        },
    ))
}

// ===========================================================================
// Body part component basics
// ===========================================================================

#[test]
fn body_part_has_slot_and_def() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartSlot>();
    test.register::<cdda_actor::components::BodyPartDef>();

    let def_entity = test.spawn(());
    let e = test.spawn((
        cdda_actor::components::BodyPartSlot("torso".to_string()),
        cdda_actor::components::BodyPartDef(def_entity),
    ));
    let slot = test.get::<cdda_actor::components::BodyPartSlot>(e).unwrap();
    let def = test.get::<cdda_actor::components::BodyPartDef>(e).unwrap();
    assert_eq!(slot.0, "torso");
    assert_eq!(def.0, def_entity);
}

#[test]
fn body_part_of_creature() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartOf>();

    let creature = test.spawn(());
    let def = test.spawn(());
    let part = test.spawn((
        cdda_actor::components::BodyPartOf(creature),
        cdda_actor::components::BodyPartDef(def),
        cdda_actor::components::BodyPartSlot("arm_l".to_string()),
    ));
    let rel = test
        .get::<cdda_actor::components::BodyPartOf>(part)
        .unwrap();
    assert_eq!(rel.0, creature);
}

// ===========================================================================
// CreatureBodyParts relationship
// ===========================================================================

#[test]
fn creature_body_parts_auto_populated() {
    let mut test = TestBed::new();

    let creature = test.spawn(());
    let def = spawn_def(&mut test, "torso_def");
    let part = spawn_body_part(&mut test, creature, def, "torso");

    let parts = test.get::<cdda_actor::components::CreatureBodyParts>(creature);
    assert!(parts.is_some());
    let ids: Vec<Entity> = parts.unwrap().iter().collect();
    assert_eq!(ids, vec![part]);
}

#[test]
fn multiple_body_parts() {
    let mut test = TestBed::new();

    let creature = test.spawn(());
    let def_torso = spawn_def(&mut test, "torso_def");
    let def_arm_l = spawn_def(&mut test, "arm_l_def");
    let def_arm_r = spawn_def(&mut test, "arm_r_def");
    let def_leg_l = spawn_def(&mut test, "leg_l_def");
    let def_leg_r = spawn_def(&mut test, "leg_r_def");
    let def_head = spawn_def(&mut test, "head_def");

    let torso = spawn_body_part(&mut test, creature, def_torso, "torso");
    let arm_l = spawn_body_part(&mut test, creature, def_arm_l, "arm_l");
    let arm_r = spawn_body_part(&mut test, creature, def_arm_r, "arm_r");
    let leg_l = spawn_body_part(&mut test, creature, def_leg_l, "leg_l");
    let leg_r = spawn_body_part(&mut test, creature, def_leg_r, "leg_r");
    let head = spawn_body_part(&mut test, creature, def_head, "head");

    let parts = test
        .get::<cdda_actor::components::CreatureBodyParts>(creature)
        .unwrap();
    let ids: Vec<Entity> = parts.iter().collect();
    assert_eq!(ids.len(), 6);
    assert!(ids.contains(&torso));
    assert!(ids.contains(&arm_l));
    assert!(ids.contains(&arm_r));
    assert!(ids.contains(&leg_l));
    assert!(ids.contains(&leg_r));
    assert!(ids.contains(&head));
}

// ===========================================================================
// Body part HP
// ===========================================================================

#[test]
fn body_part_hp_initialized() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartHp>();

    let e = test.spawn((cdda_actor::components::BodyPartHp {
        max: 100.0,
        current: 100.0,
        damage_multiplier: 1.0,
    },));
    let hp = test.get::<cdda_actor::components::BodyPartHp>(e).unwrap();
    assert_eq!(hp.max, 100.0);
    assert_eq!(hp.current, 100.0);
    assert_eq!(hp.damage_multiplier, 1.0);
}

#[test]
fn body_part_hp_damage() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartHp>();

    let e = test.spawn((cdda_actor::components::BodyPartHp {
        max: 100.0,
        current: 100.0,
        damage_multiplier: 1.0,
    },));
    // Reduce current HP from 100 to 60
    test.world_mut()
        .entity_mut(e)
        .insert(cdda_actor::components::BodyPartHp {
            max: 100.0,
            current: 60.0,
            damage_multiplier: 1.0,
        });
    let hp = test.get::<cdda_actor::components::BodyPartHp>(e).unwrap();
    assert_eq!(hp.current, 60.0);
    assert!(hp.current < hp.max);
}

#[test]
fn body_part_hp_damage_multiplier() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartHp>();

    // damage_multiplier = 0.5 means only half damage is applied.
    // Simulate: take 40 raw damage → apply 20 after multiplier
    let raw_damage = 40.0;
    let multiplier = 0.5;
    let effective_damage = raw_damage * multiplier;

    let e = test.spawn((cdda_actor::components::BodyPartHp {
        max: 100.0,
        current: 100.0,
        damage_multiplier: multiplier,
    },));
    let new_current = 100.0 - effective_damage;
    test.world_mut()
        .entity_mut(e)
        .insert(cdda_actor::components::BodyPartHp {
            max: 100.0,
            current: new_current,
            damage_multiplier: multiplier,
        });
    let hp = test.get::<cdda_actor::components::BodyPartHp>(e).unwrap();
    assert_eq!(hp.current, 80.0);
    assert_eq!(hp.damage_multiplier, 0.5);
}

// ===========================================================================
// Body part status markers (Broken / Severed)
// ===========================================================================

#[test]
fn body_part_broken_marker() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartBroken>();

    let e = test.spawn((
        cdda_actor::components::BodyPartSlot("arm_l".to_string()),
        cdda_actor::components::BodyPartBroken,
    ));
    assert!(test
        .world()
        .entity(e)
        .contains::<cdda_actor::components::BodyPartBroken>());
}

#[test]
fn body_part_severed_marker() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartSevered>();

    let e = test.spawn((
        cdda_actor::components::BodyPartSlot("arm_l".to_string()),
        cdda_actor::components::BodyPartSevered,
    ));
    assert!(test
        .world()
        .entity(e)
        .contains::<cdda_actor::components::BodyPartSevered>());
}

#[test]
fn body_part_broken_and_not_severed() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::BodyPartBroken>();
    test.register::<cdda_actor::components::BodyPartSevered>();

    // Broken without Severed
    let e = test.spawn((
        cdda_actor::components::BodyPartSlot("arm_l".to_string()),
        cdda_actor::components::BodyPartBroken,
        // BodyPartSevered deliberately NOT inserted
    ));
    let entity_ref = test.world().entity(e);
    assert!(entity_ref.contains::<cdda_actor::components::BodyPartBroken>());
    assert!(!entity_ref.contains::<cdda_actor::components::BodyPartSevered>());
}

// ===========================================================================
// Body part removal
// ===========================================================================

#[test]
fn body_part_removed() {
    let mut test = TestBed::new();

    let creature = test.spawn(());
    let def = spawn_def(&mut test, "torso_def");
    let part = spawn_body_part(&mut test, creature, def, "torso");

    // Verify it's present
    {
        let parts = test
            .get::<cdda_actor::components::CreatureBodyParts>(creature)
            .unwrap();
        assert!(parts.iter().any(|e| e == part));
    }

    // Remove (despawn) the body part.
    // The part's on_remove hook cleans up CreatureBodyParts;
    // when the last part goes, the component may be removed entirely.
    test.world_mut().despawn(part);

    let parts = test.get::<cdda_actor::components::CreatureBodyParts>(creature);
    match parts {
        Some(cbp) => {
            assert!(!cbp.iter().any(|e| e == part));
            assert_eq!(cbp.iter().count(), 0);
        }
        None => {
            // Component removed because no body parts remain — also correct.
        }
    }
}

#[test]
fn no_body_parts_empty() {
    let mut test = TestBed::new();

    let creature = test.spawn(());

    // Creature with no body parts — CreatureBodyParts should be absent
    // (or present but empty; the relationship hook inserts a marker
    // component when the first BodyPartOf is added, but if none are
    // ever added the component never appears).
    let parts = test.get::<cdda_actor::components::CreatureBodyParts>(creature);
    assert!(
        parts.is_none(),
        "A creature with no body parts should not have a CreatureBodyParts component"
    );
}
