//! AI phase — entities decide actions.
//!
//! AI systems READ world state and WRITE intent components only.
//! No world mutation during evaluation.

use bevy_ecs::prelude::*;

/// All AI logic for one tick.
///
/// For each creature: evaluate threats, choose targets, set movement intents.
pub fn ai_phase(world: &mut World) {
    let _ = world;
}
