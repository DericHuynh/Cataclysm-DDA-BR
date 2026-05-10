//! # Activity system — multi-turn player activities
//!
//! Implements the activity system from Cataclysm-DDA, translated to Bevy ECS.
//! Each character can have one active `PlayerActivity` component. Every simulation
//! turn, `tick_activities` advances the activity, calling the appropriate actor.
//!
//! ## Architecture
//!
//! * `ActivityTypeDef` (in `core::raw_defs`) — JSON definition loaded from data files.
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
