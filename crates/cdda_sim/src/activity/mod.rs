//! # Activity system — multi-turn player activities
//!
//! Implements the activity system from Cataclysm-DDA, translated to Bevy ECS.
//! Each actor has one progress/type pair. The shared budget scheduler selects
//! either activity work or an action; lifecycle operations own interruption
//! and reject ambiguous combinations before any work is spent.
//!
//! ## Architecture
//!
//! The **data components** (`ActivityProgress`, `Crafting`, `Aiming`, `Reading`,
//! `Waiting`, `Reloading`, `Interacting`, `ActivityTracker`) live in
//! `cdda_components::activity` so the UI, combat, and inventory/body layers can
//! query them without depending on this crate's systems. This module keeps only
//! the **systems** (`tick_crafting`, `tick_aiming`, …) that advance them, plus
//! the plugin wiring — i.e. systems live in the crate whose main task is the
//! sim, while the data is shared across layers.
//!
//! * `tick_crafting`, `tick_aiming`, etc. — per-activity regular systems with
//!   typed queries (no `&mut World`).

pub mod actor;
pub mod plugin;
pub mod systems;

pub use cdda_components::activity::{
    ActivityPhase, ActivityProgress, ActivityTracker, ActivityTypeId, Aiming, Crafting,
    Interacting, Reading, Reloading, Waiting,
};

pub mod lifecycle;
