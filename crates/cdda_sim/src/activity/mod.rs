//! # Activity system — multi-turn player activities
//!
//! Implements the activity system from Cataclysm-DDA, translated to Bevy ECS.
//! Each character can have one activity of each type active simultaneously
//! (future: multiple via traits/mutations). Every simulation tick, per-activity
//! systems advance progress.
//!
//! ## Architecture
//!
//! * `ActivityProgress` — common progress tracking component (moves_total/left/phase).
//! * `Crafting`, `Aiming`, `Reading`, `Waiting`, `Reloading`, `Interacting` —
//!   per-activity-type data components.
//! * `tick_crafting`, `tick_aiming`, etc. — per-activity regular systems with
//!   typed queries (no `&mut World`).
//! * `ActivityTracker` — ECS component tracking weariness and calorie balance.

pub mod actor;
pub mod components;
pub mod plugin;
pub mod systems;
pub mod tracker;

pub use components::{
    ActivityPhase, ActivityProgress, ActivityTypeId, Aiming, Crafting, Interacting, Reading,
    Reloading, Waiting,
};
pub use tracker::ActivityTracker;
