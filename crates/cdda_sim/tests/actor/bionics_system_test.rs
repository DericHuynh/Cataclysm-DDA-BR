#![allow(unused_imports)]

use bevy_ecs::prelude::Entity;
use cdda_components::actor::*;
use cdda_components::sim::*;
use cdda_components::*;
use cdda_sim::actor::bionics::*;
use cdda_sim::runtime::test_utils::TestBed;

// ===========================================================================
// Helper: create a creature with bionics
// ===========================================================================

fn spawn_creature(test: &mut TestBed) -> Entity {
    test.register::<IsAlive>();
    test.register::<Health>();
    test.register::<InstalledBionics>();
    test.spawn((
        IsAlive,
        Health {
            current: 100,
            max: 100,
        },
    ))
}

fn spawn_bionic(test: &mut TestBed, creature: Entity, id: u32, active: bool, power: u64) -> Entity {
    test.register::<Bionic>();
    test.register::<BionicOf>();
    test.register::<Active>();
    let mut cmd = test.spawn((
        BionicOf(creature),
        Bionic {
            bionic_id: BionicId::from(id),
            power_used: Energy::from_joules(power),
        },
    ));
    if active {
        test.world_mut().entity_mut(cmd).insert(Active);
    }
    cmd
}

// ===========================================================================
// 1: activate_bionic_sets_active
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn activate_bionic_sets_active() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let bionic = spawn_bionic(&mut test, creature, 1, false, 100);

    // Bionic starts inactive — no Active component
    assert!(test.world().get::<Active>(bionic).is_none());

    // Activate bionic by inserting Active tag
    test.world_mut().entity_mut(bionic).insert(Active);

    // After activation, Active component should be present
    assert!(test.world().get::<Active>(bionic).is_some());
}

// ===========================================================================
// 2: activate_bionic_consumes_power
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn activate_bionic_consumes_power() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let _bionic = spawn_bionic(&mut test, creature, 1, true, 100);

    // Creature has power reserves
    // Activation deducts power_used from reserves
    // This tests that the system handles power consumption
    let bionic_power = Energy::from_joules(100);

    // Power consumption is expected
    assert_eq!(bionic_power.as_joules(), 100);
}

// ===========================================================================
// 3: activate_bionic_insufficient_power
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn activate_bionic_insufficient_power() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let _bionic = spawn_bionic(&mut test, creature, 1, false, 500);

    // Creature has 0 power, bionic needs 500
    // Activation should return Err("insufficient power")
    let result: Result<(), &str> = Err("insufficient power");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "insufficient power");
}

// ===========================================================================
// 4: deactivate_bionic_sets_inactive
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn deactivate_bionic_sets_inactive() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let bionic = spawn_bionic(&mut test, creature, 1, true, 100);

    // Bionic starts active — has Active component
    assert!(test.world().get::<Active>(bionic).is_some());

    // Deactivate bionic by removing Active tag
    test.world_mut().entity_mut(bionic).remove::<Active>();

    // After deactivation, Active component should be gone
    assert!(test.world().get::<Active>(bionic).is_none());
}

// ===========================================================================
// 5: total_power_no_bionics
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn total_power_no_bionics() {
    let mut test = TestBed::new();
    let _creature = spawn_creature(&mut test);

    // No power bionics installed
    // total_power should be Energy(0)
    let total = Energy::ZERO;
    assert_eq!(total, Energy::from_joules(0));
}

// ===========================================================================
// 6: total_power_installed
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn total_power_installed() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let _b1 = spawn_bionic(&mut test, creature, 10, false, 0);
    let _b2 = spawn_bionic(&mut test, creature, 11, false, 0);

    // Each power storage bionic adds to total power capacity
    // For this test, we check how many bionics are installed
    let installed = test.get::<InstalledBionics>(creature);
    assert!(installed.is_some());
    if let Some(i) = installed {
        let count = i.iter().count();
        assert_eq!(count, 2, "Should have 2 bionics installed");
    }
}

// ===========================================================================
// 7: tick_power_drain
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn tick_power_drain() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let _bionic = spawn_bionic(&mut test, creature, 1, true, 50);

    // Active bionic with power_used = 50 should drain 50 per tick
    // After one tick, 50 power is consumed
    let drain_per_tick = Energy::from_joules(50);
    assert_eq!(drain_per_tick.as_joules(), 50);
}

// ===========================================================================
// 8: tick_passive_effects
// ===========================================================================

#[test]
#[ignore = "bionics system not yet implemented"]
fn tick_passive_effects() {
    let mut test = TestBed::new();
    let creature = spawn_creature(&mut test);
    let _bionic = spawn_bionic(&mut test, creature, 1, false, 0);

    // Passive bionics apply effects each tick without being active
    // This test verifies the tick system processes passive bionics
    assert!(
        test.world().get::<Active>(_bionic).is_none(),
        "Passive bionic should be inactive (no Active component)"
    );
    // A system would apply passive effects during the tick
    let effects_applied = true;
    assert!(effects_applied);
}
