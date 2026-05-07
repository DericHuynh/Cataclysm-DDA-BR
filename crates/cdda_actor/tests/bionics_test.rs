//! Integration tests for actor bionics: relationships, components, lifecycle.
//!
//! Tests cover:
//! - `Bionic` component field creation and mutation (active flag, id, power)
//! - `BionicOf`/`InstalledBionics` relationship (auto-population, iteration)
//! - Installing multiple bionics, removing bionics, reassigning bionics
//! - Creature entity with `PlayerData` and `Stats`
//!
//! All tests use `TestBed` from `cdda_sim::test_utils`.

use bevy_ecs::entity::Entity;
use cdda_sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helper: create a creature entity with IsAlive and Health
// ---------------------------------------------------------------------------

fn spawn_creature(test: &mut TestBed, name: &str) -> Entity {
    test.register::<cdda_actor::components::IsAlive>();
    test.register::<cdda_actor::components::Health>();
    test.register::<cdda_actor::components::Creature>();
    test.spawn((
        cdda_actor::components::IsAlive,
        cdda_actor::components::Health {
            current: 100,
            max: 100,
        },
        cdda_actor::components::Creature {
            def_id: "test_creature".into(),
            name: name.to_string(),
            species: 0u32.into(),
            symbol: '@',
        },
    ))
}

fn spawn_bionic(test: &mut TestBed, creature: Entity, id: u32, active: bool, power: u64) -> Entity {
    test.register::<cdda_actor::components::Bionic>();
    test.register::<cdda_actor::components::BionicOf>();
    test.spawn((
        cdda_actor::components::BionicOf(creature),
        cdda_actor::components::Bionic {
            bionic_id: id.into(),
            active,
            power_used: cdda_core::Energy(power),
        },
    ))
}

// ===========================================================================
// Bionic component basics
// ===========================================================================

#[test]
fn bionic_has_id_and_active_flag() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Bionic>();

    let e = test.spawn((cdda_actor::components::Bionic {
        bionic_id: cdda_core::BionicId::from(5u32),
        active: false,
        power_used: cdda_core::Energy(0),
    },));
    let b = test.get::<cdda_actor::components::Bionic>(e).unwrap();
    assert_eq!(b.bionic_id, cdda_core::BionicId::from(5u32));
    assert!(!b.active);
    assert_eq!(b.power_used, cdda_core::Energy(0));
}

#[test]
fn bionic_active_toggle() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Bionic>();

    let e = test.spawn((cdda_actor::components::Bionic {
        bionic_id: cdda_core::BionicId::from(2u32),
        active: true,
        power_used: cdda_core::Energy(0),
    },));
    assert!(
        test.get::<cdda_actor::components::Bionic>(e)
            .unwrap()
            .active
    );

    // Toggle by reinserting (immutable component pattern)
    test.world_mut()
        .entity_mut(e)
        .insert(cdda_actor::components::Bionic {
            bionic_id: cdda_core::BionicId::from(2u32),
            active: false,
            power_used: cdda_core::Energy(0),
        });
    assert!(
        !test
            .get::<cdda_actor::components::Bionic>(e)
            .unwrap()
            .active
    );
}

// ===========================================================================
// BionicOf / InstalledBionics relationship
// ===========================================================================

#[test]
fn bionic_of_relationship() {
    let mut test = TestBed::new();

    let creature = test.spawn((cdda_actor::components::IsAlive,));
    let bionic = test.spawn((
        cdda_actor::components::BionicOf(creature),
        cdda_actor::components::Bionic {
            bionic_id: cdda_core::BionicId::from(3u32),
            active: true,
            power_used: cdda_core::Energy(0),
        },
    ));

    let rel = test
        .get::<cdda_actor::components::BionicOf>(bionic)
        .unwrap();
    assert_eq!(rel.0, creature);
}

#[test]
fn installed_bionics_auto_populated() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Bionic>();

    let creature = test.spawn((cdda_actor::components::IsAlive,));
    let bionic = spawn_bionic(&mut test, creature, 0, false, 0);

    let installed = test.get::<cdda_actor::components::InstalledBionics>(creature);
    assert!(installed.is_some());
    let ids: Vec<Entity> = installed.unwrap().iter().collect();
    assert_eq!(ids, vec![bionic]);
}

#[test]
fn multiple_bionics() {
    let mut test = TestBed::new();

    let creature = test.spawn((cdda_actor::components::IsAlive,));
    let b1 = spawn_bionic(&mut test, creature, 10, true, 500);
    let b2 = spawn_bionic(&mut test, creature, 11, false, 250);

    let installed = test
        .get::<cdda_actor::components::InstalledBionics>(creature)
        .unwrap();
    let ids: Vec<Entity> = installed.iter().collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&b1));
    assert!(ids.contains(&b2));
}

#[test]
fn bionic_removed() {
    let mut test = TestBed::new();

    let creature = test.spawn((cdda_actor::components::IsAlive,));
    let b1 = spawn_bionic(&mut test, creature, 10, true, 500);
    let _b2 = spawn_bionic(&mut test, creature, 11, false, 250);

    // Verify both are present
    {
        let installed = test
            .get::<cdda_actor::components::InstalledBionics>(creature)
            .unwrap();
        assert_eq!(installed.iter().count(), 2);
    }

    // Despawn one bionic (linked_spawn does NOT cascade up, so creature survives)
    test.world_mut().despawn(b1);

    let installed = test
        .get::<cdda_actor::components::InstalledBionics>(creature)
        .unwrap();
    let ids: Vec<Entity> = installed.iter().collect();
    assert_eq!(ids.len(), 1);
    assert!(!ids.contains(&b1));
}

// ===========================================================================
// Bionic power usage
// ===========================================================================

#[test]
fn bionic_power_usage() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Bionic>();

    let e = test.spawn((cdda_actor::components::Bionic {
        bionic_id: cdda_core::BionicId::from(7u32),
        active: true,
        power_used: cdda_core::Energy(2500),
    },));
    let b = test.get::<cdda_actor::components::Bionic>(e).unwrap();
    assert_eq!(b.power_used, cdda_core::Energy(2500));
    assert_eq!(b.power_used.as_joules(), 2500);
}

// ===========================================================================
// Bionic reassignment (move from creature A to creature B)
// ===========================================================================

#[test]
fn bionic_reassignment() {
    let mut test = TestBed::new();

    let creature_a = test.spawn((cdda_actor::components::IsAlive,));
    let creature_b = test.spawn((cdda_actor::components::IsAlive,));

    let bionic = spawn_bionic(&mut test, creature_a, 1, false, 0);

    // Verify it's on creature_a
    {
        let installed = test
            .get::<cdda_actor::components::InstalledBionics>(creature_a)
            .unwrap();
        assert!(installed.iter().any(|e| e == bionic));
    }

    // Reassign: reinsert BionicOf pointing to creature_b
    test.world_mut()
        .entity_mut(bionic)
        .insert(cdda_actor::components::BionicOf(creature_b));

    // creature_a should no longer have it (component may be removed when empty)
    let installed_a = test.get::<cdda_actor::components::InstalledBionics>(creature_a);
    if let Some(installed) = installed_a {
        assert!(!installed.iter().any(|e| e == bionic));
    }
    // If the component is gone entirely, that also means the bionic was removed
    // (the relationship hook removes the target when the last relation is moved).

    // creature_b should now have it
    let installed_b = test
        .get::<cdda_actor::components::InstalledBionics>(creature_b)
        .unwrap();
    assert!(installed_b.iter().any(|e| e == bionic));
}

// ===========================================================================
// Creature entity with PlayerData and Stats
// ===========================================================================

#[test]
fn creature_has_playerdata() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::PlayerData>();

    let e = test.spawn((cdda_actor::components::PlayerData {
        name: "Alice".to_string(),
        gender: cdda_actor::components::Gender::Female,
        age: 25,
        height: 170,
        blood_type: "O+".to_string(),
        profession: None,
        scenario: None,
    },));
    let pd = test.get::<cdda_actor::components::PlayerData>(e).unwrap();
    assert_eq!(pd.name, "Alice");
    assert_eq!(pd.gender, cdda_actor::components::Gender::Female);
    assert_eq!(pd.age, 25);
}

#[test]
fn creature_stats_initialized() {
    let mut test = TestBed::new();
    test.register::<cdda_actor::components::Stats>();

    let e = test.spawn((cdda_actor::components::Stats(cdda_core::Stats::new(
        8, 8, 8, 8,
    )),));
    let s = test.get::<cdda_actor::components::Stats>(e).unwrap();
    assert_eq!(s.0.strength, 8);
    assert_eq!(s.0.dexterity, 8);
    assert_eq!(s.0.intelligence, 8);
    assert_eq!(s.0.perception, 8);
}
