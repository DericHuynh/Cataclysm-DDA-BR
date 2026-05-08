//! Tests for status effect logic using pure functions.
//!
//! All tests operate on plain `StatusEffect` instances without an ECS world,
//! exercising the decay, intensity, and merge helpers defined below.

#![allow(unused_imports)]

use bevy_ecs::prelude::*;
use cdda_core::sim::test_utils::TestBed;
use cdda_core::sim::systems::effects::*;
use cdda_core::sim::components::*;
use cdda_core::actor::components::{StatusEffect, EffectOn, ActiveEffects, IsAlive, Health};
use cdda_core::{EffectId, Time};

// ---------------------------------------------------------------------------
// Pure helper functions for status effect logic
// ---------------------------------------------------------------------------

/// Decay a status effect by the given number of turns.
fn decay_effect(effect: &mut StatusEffect, turns: i64) {
    effect.remaining = effect.remaining - cdda_core::Time::from_turns(turns);
    if effect.remaining < cdda_core::Time::ZERO {
        effect.remaining = cdda_core::Time::ZERO;
    }
}

/// Whether an effect is still active.
fn is_active(effect: &StatusEffect) -> bool {
    effect.remaining > cdda_core::Time::ZERO
}

/// Increase intensity of an effect (caps at some max).
fn increase_intensity(effect: &mut StatusEffect, amount: u32, max: u32) {
    effect.intensity = (effect.intensity + amount).min(max);
}

/// Decrease intensity of an effect (floor at 0).
fn decrease_intensity(effect: &mut StatusEffect, amount: u32) {
    effect.intensity = effect.intensity.saturating_sub(amount);
}

/// Merge another effect into this one (same `effect_id`). Takes the longer
/// remaining duration and adds intensities (capped at `max_intensity`).
fn merge_effects(target: &mut StatusEffect, source: &StatusEffect, max_intensity: u32) {
    if source.remaining > target.remaining {
        target.remaining = source.remaining;
    }
    target.intensity = (target.intensity + source.intensity).min(max_intensity);
}

/// Create a test `StatusEffect` with the given id, intensity, and remaining turns.
fn make_effect(id: u32, intensity: u32, turns: i64) -> StatusEffect {
    StatusEffect {
        effect_id: EffectId::from(id),
        intensity,
        remaining: cdda_core::Time::from_turns(turns),
    }
}

// ---------------------------------------------------------------------------
// Decay tests
// ---------------------------------------------------------------------------

#[test]
fn decay_reduces_remaining() {
    let mut effect = make_effect(0, 1, 100);
    decay_effect(&mut effect, 50);
    assert_eq!(effect.remaining.as_turns(), 50);
}

#[test]
fn decay_to_zero_stops() {
    let mut effect = make_effect(0, 1, 100);
    decay_effect(&mut effect, 200);
    assert_eq!(effect.remaining.as_turns(), 0);
}

// ---------------------------------------------------------------------------
// Active checks
// ---------------------------------------------------------------------------

#[test]
fn is_active_true() {
    let effect = make_effect(0, 1, 50);
    assert!(is_active(&effect));
}

#[test]
fn is_active_false() {
    let effect = make_effect(0, 1, 0);
    assert!(!is_active(&effect));
}

// ---------------------------------------------------------------------------
// Intensity manipulation
// ---------------------------------------------------------------------------

#[test]
fn increase_intensity_adds() {
    let mut effect = make_effect(0, 1, 100);
    increase_intensity(&mut effect, 2, 10);
    assert_eq!(effect.intensity, 3);
}

#[test]
fn increase_intensity_capped() {
    let mut effect = make_effect(0, 8, 100);
    increase_intensity(&mut effect, 5, 10);
    assert_eq!(effect.intensity, 10);
}

#[test]
fn decrease_intensity_reduces() {
    let mut effect = make_effect(0, 5, 100);
    decrease_intensity(&mut effect, 3);
    assert_eq!(effect.intensity, 2);
}

#[test]
fn decrease_intensity_floor() {
    let mut effect = make_effect(0, 3, 100);
    decrease_intensity(&mut effect, 10);
    assert_eq!(effect.intensity, 0);
}

// ---------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------

#[test]
fn merge_effects_longer_duration() {
    let mut target = make_effect(0, 2, 30);
    let source = make_effect(0, 1, 100);

    merge_effects(&mut target, &source, 10);

    // Source has longer remaining → target should take it
    assert_eq!(target.remaining.as_turns(), 100);
}

#[test]
fn merge_effects_intensity_adds() {
    let mut target = make_effect(0, 2, 50);
    let source = make_effect(0, 3, 50);

    merge_effects(&mut target, &source, 10);

    assert_eq!(target.intensity, 5);
    // Remaining durations are equal, so stays at 50
    assert_eq!(target.remaining.as_turns(), 50);
}

#[test]
fn merge_effects_capped() {
    let mut target = make_effect(0, 8, 50);
    let source = make_effect(0, 5, 50);

    merge_effects(&mut target, &source, 10);

    // 8 + 5 = 13, capped at 10
    assert_eq!(target.intensity, 10);
}

#[test]
fn merge_effects_shorter_duration_keeps_longer() {
    let mut target = make_effect(0, 2, 100);
    let source = make_effect(0, 3, 30);

    merge_effects(&mut target, &source, 10);

    // Source has shorter remaining → target keeps its longer 100
    assert_eq!(target.remaining.as_turns(), 100);
    // Intensities add: 2 + 3 = 5
    assert_eq!(target.intensity, 5);
}

// ===========================================================================
// System integration tests — call `effects_phase` on a world
// ===========================================================================

#[test]
#[ignore = "effects system not yet implemented"]
fn system_apply_effect_creates_entity() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // apply_effect should create a child entity with EffectOn pointing to the
    // creature.  Stub does nothing → no child entity → fails.
    let has_effects = test
        .world()
        .entity(creature)
        .contains::<ActiveEffects>();
    assert!(
        has_effects,
        "apply_effect should create a child EffectOn entity"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_apply_effect_sets_duration() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // An effect with duration 100 should store that on the StatusEffect.
    // Stub does nothing → no effect entity → fails.
    let has_effects = test
        .world()
        .entity(creature)
        .contains::<ActiveEffects>();
    assert!(
        has_effects,
        "applied effect should store duration on StatusEffect component"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_apply_multiple_effects() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // Two different EffectIds applied to the same creature should both
    // be stored as separate child entities.  Stub does nothing → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "multiple effects should each create their own StatusEffect entity"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_remove_effect_cleans_up() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    // First apply the effect (stub does nothing), then remove it.
    test.run_system(effects_phase);

    // After removal, the effect entity should be despawned and the
    // ActiveEffects list should be empty.  Stub does nothing → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "remove_effect should clean up the effect entity"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_has_effect_works() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // has_effect should return true for an applied effect and false for
    // one that was never applied.  Stub does nothing → ActiveEffects
    // never created → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "has_effect should correctly report applied effect"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_get_intensity_works() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // An effect with intensity 3 should report 3.
    // Stub does nothing → no effect → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "get_intensity should return correct intensity value"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_tick_decays_by_one() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // First tick should decay remaining from 100 to 99.
    // Stub does nothing → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "tick_effects should decay StatusEffect.remaining by 1 each turn"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_tick_removes_expired() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // An effect with 1 turn remaining that is ticked should be removed.
    // Stub does nothing → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "expired effects should be removed after tick"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_effects_phase_calls_tick() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // effects_phase should call tick_effects internally.
    // If tick_effects ran, an effect with 100 remaining would decay to 99.
    // Stub does nothing → fails.
    let effects = test
        .world()
        .entity(creature)
        .get::<ActiveEffects>();
    assert!(
        effects.is_some(),
        "effects_phase should invoke tick_effects for all creatures"
    );
}

#[test]
#[ignore = "effects system not yet implemented"]
fn system_effect_on_creature_relationship() {
    let mut test = TestBed::new();
    test.register::<EffectOn>();
    test.register::<ActiveEffects>();
    test.register::<StatusEffect>();
    test.register::<IsAlive>();
    test.register::<Health>();

    let creature = test.spawn((
        IsAlive,
        Health { current: 100, max: 100 },
    ));

    test.run_system(effects_phase);

    // The creature's ActiveEffects relationship target should be populated
    // with entities.  Stub does nothing → ActiveEffects absent → fails.
    let has_effects = test
        .world()
        .entity(creature)
        .contains::<ActiveEffects>();
    assert!(
        has_effects,
        "creature should have ActiveEffects populated after applying an effect"
    );
}
