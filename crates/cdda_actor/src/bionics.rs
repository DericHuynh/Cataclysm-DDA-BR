//! Bionics system — CBM activation, deactivation, and power management.
//!
//! Bionics are individual entities related to creatures via
//! `BionicOf`/`InstalledBionics`. Each bionic has a power cost,
//! activation state, and may produce passive effects while active.
//! Power is stored as an `Energy` resource or component on the
//! creature.

use bevy_ecs::prelude::*;
use cdda_components::Energy;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Activate a bionic on a creature.
///
/// Checks available power, sets `Bionic.active = true`.
/// Returns `Err` if insufficient power or the bionic is already active.
pub fn activate_bionic(world: &mut World, creature: Entity, bionic: Entity) -> Result<(), String> {
    let _ = (world, creature, bionic);
    todo!("bionic activation: check power, set active flag, start passive effects")
}

/// Deactivate an active bionic.
///
/// Sets `Bionic.active = false`, stops any passive effects.
pub fn deactivate_bionic(world: &mut World, creature: Entity, bionic: Entity) {
    let _ = (world, creature, bionic);
    todo!("bionic deactivation: clear active flag, remove passive effect entities")
}

/// Calculate total stored power for a creature.
///
/// Sums all power storage bionics and base capacity.
pub fn total_power(world: &World, entity: Entity) -> Energy {
    let _ = (world, entity);
    todo!("sum power storage: iterate InstalledBionics, sum Bionic.power_used")
}

/// Tick all bionics — process power drain and passive effects.
///
/// Active bionics consume power each turn. Bionics with zero power
/// are automatically deactivated. Passive bionics apply their effects
/// as `StatusEffect` entities.
pub fn tick_bionics(world: &mut World) {
    let _ = world;
}
