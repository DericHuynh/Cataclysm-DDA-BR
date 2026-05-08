//! Message types for decoupled system communication.
//!
//! Systems communicate through Bevy Messages (buffered, broadcast), not
//! direct mutation.  Adding a new reaction is a new message reader — no
//! existing code changes.
//!
//! In Bevy 0.17+, there is a split:
//! - `#[derive(Message)]` — buffered, broadcast (replaces old `Event`)
//! - `#[derive(Event)]` — observer-based, triggered on specific entities
//!
//! All types in this module are globally broadcast → they derive `Message`.

use crate::core::coords::WorldPos;
use crate::core::id::*;
use crate::Damage;
use bevy_ecs::entity::Entity;
use bevy_ecs::message::Message;

// ---------------------------------------------------------------------------
// Damage / Death
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub struct DamageEvent {
    pub target: Entity,
    pub damage: Damage,
    pub source: Option<Entity>,
}

#[derive(Message, Debug, Clone)]
pub struct DeathEvent {
    pub entity: Entity,
    pub cause: DeathCause,
    pub position: WorldPos,
}

// ---------------------------------------------------------------------------
// Sensory events — AI reacts to these
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub struct SoundEvent {
    pub position: WorldPos,
    pub volume: u32,
    pub description: String,
}

#[derive(Message, Debug, Clone)]
pub struct SightEvent {
    pub observer: Entity,
    pub seen: Entity,
    pub position: WorldPos,
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub struct SpawnEvent {
    pub template_id: MonsterId,
    pub position: WorldPos,
    pub faction: FactionId,
}

// ---------------------------------------------------------------------------
// Definition hot-reload (T1)
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub struct DefChangedEvent {
    pub category: DefCategory,
    /// Numeric indices of changed definitions.
    pub ids: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Trade / Inventory
// ---------------------------------------------------------------------------

#[derive(Message, Debug, Clone)]
pub struct ItemMoveEvent {
    /// The item entity being moved.
    pub item: Entity,
    /// Where the item was (entity container or WorldPos on ground).
    pub from: MoveLocation,
    /// Where the item is going (entity container or WorldPos on ground).
    pub to: MoveLocation,
    /// How many items in the stack were moved.
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveLocation {
    /// Item is on the ground at this world position.
    Ground(crate::core::coords::WorldPos),
    /// Item is inside a container entity.
    Container(Entity),
    /// Item is wielded by an entity.
    Wielded(Entity),
    /// Item is worn by an entity.
    Worn(Entity),
}

#[derive(Message, Debug, Clone)]
pub struct EquipEvent {
    pub wielder: Entity,
    pub item: Entity,
}

#[derive(Message, Debug, Clone)]
pub struct UnequipEvent {
    pub wielder: Entity,
    pub item: Entity,
}

#[derive(Message, Debug, Clone)]
pub struct UseItemEvent {
    pub user: Entity,
    pub item: Entity,
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

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
