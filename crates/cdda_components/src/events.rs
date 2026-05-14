//! Event and Message types for decoupled system communication.
//!
//! ## Observer-based Events (immediate)
//!
//! Re-exported from the `cdda_events` crate.  These are triggered via
//! `commands.trigger()` and handled immediately by observer systems.
//! Use these for immediate, reactive communication (UI feedback,
//! entity damage reactions, death handling).
//!
//! ## Buffered Messages (batch)
//!
//! Defined here.  These use `MessageWriter`/`MessageReader` for
//! batched, frame-delayed communication.  Use these for bulk
//! processing (inventory moves, AI sensory input, spawning).

// ── Observer-based Event re-exports ──────────────────────────────────────

pub use cdda_events::{DamageEvent, DeathEvent, EquipEvent, UnequipEvent, UseItemEvent};
pub use cdda_events::{DeathCause, GameEvent, MoveLocation};

// ── Buffered Message types ───────────────────────────────────────────────

use bevy_ecs::entity::Entity;
use bevy_ecs::message::Message;
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::core::id::DefCategory;
use cdda_core_types::core::DefId;

/// Phantom types for event type parameters.
#[derive(Debug, Clone, Copy)]
pub struct MonsterDef;
#[derive(Debug, Clone, Copy)]
pub struct FactionDef;

/// An item moved between locations (ground, container, wielded, worn).
///
/// Buffered message — processed in batch by `process_item_move_events`.
#[derive(Message, Debug, Clone)]
pub struct ItemMoveEvent {
    /// The item entity being moved.
    pub item: Entity,
    /// Where the item was (entity container or WorldPos on ground).
    pub from: super::events::MoveLocation,
    /// Where the item is going (entity container or WorldPos on ground).
    pub to: super::events::MoveLocation,
    /// How many items in the stack were moved.
    pub count: u32,
}

/// A sound was produced in the world (AI reacts to these).
///
/// Buffered message — processed in batch by AI sensory systems.
#[derive(Message, Debug, Clone)]
pub struct SoundEvent {
    pub position: WorldPos,
    pub volume: u32,
    pub description: String,
}

/// An entity was seen by an observer.
///
/// Buffered message — processed in batch by AI sensory systems.
#[derive(Message, Debug, Clone)]
pub struct SightEvent {
    pub observer: Entity,
    pub seen: Entity,
    pub position: WorldPos,
}

/// A new creature should be spawned into the world.
///
/// Buffered message — processed in batch by the spawning system.
#[derive(Message, Debug, Clone)]
pub struct SpawnEvent {
    pub template_id: DefId<MonsterDef>,
    pub position: WorldPos,
    pub faction: DefId<FactionDef>,
}

/// One or more definitions changed (e.g. hot-reload).
///
/// Buffered message — processed in batch to reload assets.
#[derive(Message, Debug, Clone)]
pub struct DefChangedEvent {
    pub category: DefCategory,
}
