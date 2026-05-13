//! # cdda_events — Observer-based event types for immediate reactions.
//!
//! This crate provides **observer-based** `Event` and `EntityEvent` types
//! that are triggered via `commands.trigger()` and handled immediately by
//! observer systems.  These are for **immediate, reactive** communication
//! (UI feedback, entity damage reactions, death handling).
//!
//! For **buffered, batch-processed** communication, see the `Message`
//! types in `cdda_components` (e.g. `ItemMoveEvent`, `SoundEvent`).
//!
//! ## EntityEvent types (targeted at a specific entity)
//!
//! Use `#[derive(EntityEvent)]`.  Trigger with `commands.entity(target).trigger(...)`
//! or `commands.trigger(...)` for global observers.
//!
//! ## Global Event types
//!
//! Use `#[derive(Event)]`.  Trigger with `commands.trigger(...)`.
//! Handled by global observers registered via `app.add_observer(...)`.

use bevy_ecs::entity::Entity;
use bevy_ecs::event::{EntityEvent, Event};
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::core::Damage;

// ---------------------------------------------------------------------------
// Supporting enums
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

// ---------------------------------------------------------------------------
// Global Event types — broadcast to all interested observers
// ---------------------------------------------------------------------------

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
#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Copy)]
pub struct GameEventDispatch(pub GameEvent);

// ---------------------------------------------------------------------------
// EntityEvent types — targeted at a specific entity
// ---------------------------------------------------------------------------

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
    /// The entity that unequipped the item.
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
