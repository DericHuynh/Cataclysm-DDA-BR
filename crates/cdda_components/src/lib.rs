//! # cdda_components — ECS component definitions
//!
//! All Bevy ECS components for actors, items, definitions, spatial data,
//! and stats.  Re-exports from cdda_core_types so that existing crate
//! paths (e.g. `crate::core::coords::WorldPos`) resolve correctly.

pub use cdda_core_types::{core, sim_id};

// Re-export commonly-used ID types at root level for backward compatibility.
pub use cdda_core_types::core::id::DefCategory;
pub use cdda_core_types::core::units::{Energy, Length, Time, Volume, Weight};
pub use cdda_core_types::core::Damage;
pub use cdda_core_types::core::DefId;
pub use cdda_core_types::core::WorldPos;

pub mod activity;
pub mod actor;
pub mod ai;
pub mod def;
pub mod def_markers;
pub mod dev;
pub mod events;
pub mod intent;
pub mod item;
pub mod messages;
pub mod recipe;
pub mod schedule;
pub mod sim;
pub mod stats;
pub mod tokens;

pub use def_markers::*;
pub use tokens::AmmoTypeId;
pub use tokens::BodyPartId;
pub use tokens::ComestibleId;
pub use tokens::ItemTypeId;
pub use tokens::SkillId;
