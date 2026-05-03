//! Event types for decoupled system communication.
//!
//! Systems communicate through Bevy events, not direct mutation.
//! Adding a new reaction is a new event reader — no existing code changes.

use bevy_ecs::{entity::Entity, event::Event};
use cdda_core::coords::WorldPos;
use cdda_core::id::*;

// ---------------------------------------------------------------------------
// Turn state resource
// ---------------------------------------------------------------------------

/// The phase of the game tick loop.
/// Not an Event — a Resource checked by the main tick system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnState {
    WaitingForInput,
    PlayerActed,
    Simulating,
    Animating,
}

// ---------------------------------------------------------------------------
// Damage / Death
// ---------------------------------------------------------------------------

#[derive(Event, Debug, Clone)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: i32,
    pub kind: DamageKind,
    pub source: Option<Entity>,
}

#[derive(Event, Debug, Clone)]
pub struct DeathEvent {
    pub entity: Entity,
    pub cause: DeathCause,
    pub position: WorldPos,
}

// ---------------------------------------------------------------------------
// Sensory events — AI reacts to these
// ---------------------------------------------------------------------------

#[derive(Event, Debug, Clone)]
pub struct SoundEvent {
    pub position: WorldPos,
    pub volume: u32,
    pub description: String,
}

#[derive(Event, Debug, Clone)]
pub struct SightEvent {
    pub observer: Entity,
    pub seen: Entity,
    pub position: WorldPos,
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

#[derive(Event, Debug, Clone)]
pub struct SpawnEvent {
    pub template_id: MonsterId,
    pub position: WorldPos,
    pub faction: FactionId,
}

// ---------------------------------------------------------------------------
// Definition hot-reload (T1)
// ---------------------------------------------------------------------------

#[derive(Event, Debug, Clone)]
pub struct DefChangedEvent {
    pub category: DefCategory,
    /// Numeric indices of changed definitions.
    pub ids: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    Bash,
    Cut,
    Stab,
    Bullet,
    Fire,
    Acid,
    Electric,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    Combat(Entity),
    Hunger,
    Thirst,
    Asphyxiation,
    Bleeding,
    Fall,
    Other,
}
