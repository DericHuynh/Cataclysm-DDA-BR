//! Event and Message types for decoupled system communication.
//!
//! ## Observer-based Events (immediate)
//!
//! Defined here.  These are triggered via `commands.trigger()` and handled
//! immediately by observer systems.  Use these for immediate, reactive
//! communication (UI feedback, entity damage reactions, death handling).
//!
//! ## Buffered Messages (batch)
//!
//! Defined here.  These use `MessageWriter`/`MessageReader` for batched,
//! frame-delayed communication.  Use these for bulk processing (inventory
//! moves, AI sensory input, spawning).

// ── Observer-based Event definitions ─────────────────────────────────────

use bevy_ecs::entity::Entity;
use bevy_ecs::event::{EntityEvent, Event};
use bevy_ecs::message::Message;
use bevy_ecs::prelude::Resource;
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::core::id::{DefCategory, DefId};
use cdda_core_types::core::Damage;

#[allow(unused_imports)]
use crate::{FactionDef, MonsterDef};

// ---------------------------------------------------------------------------
// Event supporting types
// ---------------------------------------------------------------------------

/// How a creature died.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    /// Killed by another entity in combat.
    Combat(Entity),
    /// Starved to death.
    Hunger,
    /// Died of dehydration.
    Thirst,
    /// Suffocated.
    Asphyxiation,
    /// Bled out.
    Bleeding,
    /// Fell from a height.
    Fall,
    /// Some other cause.
    Other,
}

/// Where an item can be located.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveLocation {
    /// Item is on the ground at this world position.
    Ground(WorldPos),
    /// Item is inside a container entity.
    Container(Entity),
    /// Item is wielded by an entity.
    Wielded(Entity),
    /// Item is worn by an entity.
    Worn(Entity),
}

/// Top-level game lifecycle events.
///
/// Triggered by the navigation system when a screen command targets
/// `TransitionTarget::Event(...)`.  Application-level observers react
/// to these (e.g. transitioning `AppState` on `StartNewGame`).
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    /// Start a new game (from the main menu).
    StartNewGame,
    /// Save the current game and quit to desktop.
    SaveAndQuit,
}

/// Resource used to dispatch GameEvents from the navigation system.
/// Inserted by `nav::dispatch`, consumed by `cdda_app`.
#[derive(Resource, Debug, Clone, Copy)]
pub struct GameEventDispatch(pub GameEvent);

/// Damage applied to a target entity.
///
/// # Target
/// The `target` entity receiving damage.
#[derive(EntityEvent, Debug, Clone)]
pub struct DamageEvent {
    /// The entity receiving damage.
    #[event_target]
    pub target: Entity,
    /// The damage profile.
    pub damage: Damage,
    /// The entity that caused the damage (if any).
    pub source: Option<Entity>,
}

/// An entity has died.
///
/// # Target
/// The `entity` that died (auto-detected by `EntityEvent` derive).
#[derive(EntityEvent, Debug, Clone)]
pub struct DeathEvent {
    /// The entity that died.
    pub entity: Entity,
    /// What caused the death.
    pub cause: DeathCause,
    /// Where the death occurred.
    pub position: WorldPos,
}

/// An item was equipped by a wielder.
///
/// # Target
/// The `wielder` entity that equipped the item.
#[derive(EntityEvent, Debug, Clone)]
pub struct EquipEvent {
    /// The entity that equipped the item.
    #[event_target]
    pub wielder: Entity,
    /// The item that was equipped.
    pub item: Entity,
}

/// An item was unequipped by a wielder.
///
/// # Target
/// The `wielder` entity that unequipped the item.
#[derive(EntityEvent, Debug, Clone)]
pub struct UnequipEvent {
    /// The entity that equipped the item.
    #[event_target]
    pub wielder: Entity,
    /// The item that was unequipped.
    pub item: Entity,
}

/// An entity used an item.
///
/// # Target
/// The `user` entity that used the item.
#[derive(EntityEvent, Debug, Clone)]
pub struct UseItemEvent {
    /// The entity that used the item.
    #[event_target]
    pub user: Entity,
    /// The item that was used.
    pub item: Entity,
}

/// Legacy whole-stack move request; submission does not imply completion.
///
/// The inventory adapter validates live source/count, actor, ownership, capacity
/// and AP synchronously, then publishes ItemMoveResult. Prefer ActionIntent.
#[derive(Message, Debug, Clone)]
pub struct ItemMoveEvent {
    /// The item entity being moved.
    pub item: Entity,
    /// Where the item was (entity container or WorldPos on ground).
    pub from: super::events::MoveLocation,
    /// Where the item is going (entity container or WorldPos on ground).
    pub to: super::events::MoveLocation,
    /// Expected whole-stack count; partial movement is unsupported.
    pub count: u32,
}

/// Terminal verdict for a legacy move request. Rejection has no mutation or AP cost.
#[derive(Message, Debug, Clone)]
pub struct ItemMoveResult {
    pub request: ItemMoveEvent,
    pub accepted: bool,
    pub reason: Option<String>,
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
