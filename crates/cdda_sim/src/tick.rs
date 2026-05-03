//! Deterministic serial tick loop.
//!
//! Systems run in a fixed order using `IntoSystem::into_system(f).run(world)`.
//! Parallelism (Section 10.2 of TARGET_ARCHITECTURE.md) is deferred.

use bevy_ecs::system::{IntoSystem, System};

/// Advance one simulation tick.
///
/// Phase order (matching TARGET_ARCHITECTURE.md Section 10):
/// 1. AI — entities decide actions (read-only world, write intents)
/// 2. Movement — resolve movement intents
/// 3. Combat — resolve combat actions, emit DamageEvent/DeathEvent/SoundEvent
/// 4. Effects — apply status effects, decay, regeneration
/// 5. Spawning — spawn new entities from SpawnEvents
pub fn run_tick(world: &mut bevy_ecs::world::World) {
    // --- Phase 1: AI ---
    let mut sys = IntoSystem::into_system(crate::systems::ai::ai_phase);
    let _ = sys.run((), world);
    sys.apply_deferred(world);

    // --- Phase 2: Movement ---
    let mut sys = IntoSystem::into_system(crate::systems::movement::movement_phase);
    let _ = sys.run((), world);
    sys.apply_deferred(world);

    // --- Phase 3: Combat ---
    let mut sys = IntoSystem::into_system(crate::systems::combat::combat_phase);
    let _ = sys.run((), world);
    sys.apply_deferred(world);

    // --- Phase 4: Effects ---
    let mut sys = IntoSystem::into_system(crate::systems::effects::effects_phase);
    let _ = sys.run((), world);
    sys.apply_deferred(world);

    // --- Phase 5: Spawning ---
    let mut sys = IntoSystem::into_system(crate::systems::spawning::spawning_phase);
    let _ = sys.run((), world);
    sys.apply_deferred(world);
}
