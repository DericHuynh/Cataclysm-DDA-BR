//! # Activity system — multi-turn player activities
//!
//! Implements the activity system from Cataclysm-DDA, translated to Bevy ECS.
//! Each character can have one active `PlayerActivity` component. Every simulation
//! turn, `tick_activities` advances the activity, calling the appropriate actor.
//!
//! ## Architecture
//!
//! * `PlayerActivity` — ECS component on the character entity; tracks progress.
//! * `ActivityTracker` — ECS component tracking weariness and calorie balance.
//! * `ActivityActor` — enum of all concrete activity implementations.
//! * `systems::tick_activities` — per-turn ECS system driving activity progress.

pub mod actor;
pub mod components;
pub mod plugin;
pub mod systems;
pub mod tracker;

pub use actor::{
    ActivityActor, AimActor, CraftActor, IdleActor, InteractActor, ReadActor, ReloadActor,
    WaitActor,
};
pub use components::{ActivityPhase, ActivityTypeId, PlayerActivity};
pub use systems::{cancel_activity, finish_activity, tick_one};
pub use tracker::ActivityTracker;

use std::sync::OnceLock;

/// Callback type for completing a crafting activity.
///
/// Registered by `cdda_core` to break the circular dependency between
/// `cdda_activity` and `cdda_crafting`.
type CompleteCraftFn = fn(&mut bevy_ecs::prelude::World, bevy_ecs::prelude::Entity, bevy_ecs::prelude::Entity);

/// Global hook for completing crafts. Set by `cdda_core` at startup.
pub static CRAFT_COMPLETE_HOOK: OnceLock<CompleteCraftFn> = OnceLock::new();
