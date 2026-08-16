//! AI phase — entities decide actions.
//!
//! AI systems READ world state and WRITE intent components only.
//! No world mutation during evaluation.
//!
//! AI decisions use:
//! - `EntitySpatialIndex` for proximity/threat detection
//! - `SightEvent` and `SoundEvent` message queues for sensory input
//! - Creature stats (Health, CombatStats, MonsterStats, Vision, Faction)
//! - Personality traits (NpcPersonality for NPCs, MonsterFlags for monsters)

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::WorldPos;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The goal an AI entity decides to pursue this action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiGoal {
    /// Move toward and attack a specific target.
    Attack { target: Entity },
    /// Move in a random direction.
    Wander,
    /// Move away from a threat.
    Flee { from: Entity },
    /// Stay within a certain radius of a position.
    Guard { position: WorldPos },
    /// Pathfind toward a target, attacking when in range.
    Hunt { target: Entity },
}

// ---------------------------------------------------------------------------
// Public API
// --------------------------------------------------------------------------

/// Decide what action an AI entity should take this tick.
///
/// Reads world state and returns an `AiGoal` without mutating
/// anything.
pub fn decide_action(world: &World, entity: Entity) -> AiGoal {
    let _ = (world, entity);
    todo!("AI decision making: evaluate threats → choose goal (Attack/Wander/Flee/Guard/Hunt)")
}

/// Execute an AI decision: translate the `AiGoal` into actual
/// movement or combat actions via `attempt_move`, `resolve_melee_attack`,
/// etc.
pub fn execute_ai_action(world: &mut World, entity: Entity, goal: AiGoal) {
    let _ = (world, entity, goal);
    todo!("execute decided action: match goal → movement/combat system calls")
}

/// All AI logic for one tick.
///
/// For each creature: evaluate threats, choose targets, set movement intents.
pub fn ai_phase() {
    // STUB: no-op until AI implemented
}
