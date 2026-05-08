//! # cdda_core — Domain types and units
//!
//! The game's domain layer: value objects (Volume, Weight, WorldPos),
//! pure component templates, and numeric ID types.
//!
//! Thin `bevy_ecs` dependency for `SystemSet` labels, common `Message`
//! types, `SimId`, and `WyRand`. Serde is allowed — domain types must
//! be serializable for save/load.
//! This is the lowest crate in the dependency graph.

#![allow(unexpected_cfgs)]

pub mod actor;
pub mod core;
pub mod data;
pub mod input;
pub mod item;
pub mod map;
pub mod messages;
pub mod render;
pub mod replay;
pub mod rng;
pub mod schedule;
pub mod screen;
pub mod sim;
pub mod sim_id;
pub mod wyrand;

// Re-export key types — all point to core:: for the canonical definitions
pub use core::coords::{BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos};
pub use core::coords::{Direction, Facing, ZLevel};
pub use core::coords::{VehicleMapPos, VehicleMountPos};
pub use core::damage::Damage;
pub use core::error::CoreError;
pub use core::flags::FlagSet;
pub use core::id::{AmmoTypeId, BodyPartId, DamageTypeId, MaterialId, SpeciesId, VitaminId};
pub use core::id::{BionicId, EffectId, FactionId, SkillId};
pub use core::id::{DefCategory, DefIdx, GenId};
pub use core::id::{FieldId, ItemGroupId, MutationCategoryId, MutationId, TraitGroupId};
pub use core::id::{FurnitureId, ItemId, MonsterId, RecipeId, TerrainId};
pub use core::id::{MapgenPaletteId, OvermapSpecialId, OvermapTerrainId};
pub use core::id::{OvermapConnectionId, OvermapLandUseCodeId, OvermapLocationId};
pub use core::id::{ProfessionId, ProficiencyId, QualityId};
pub use core::id::{ScenarioId, SpecialAttackId, StartLocationId, TechniqueId, TrapId};
pub use core::id::{VehiclePartCategoryId, VehiclePartId, VehiclePartLocationId};
pub use core::stats::Stats;
pub use core::units::{Energy, Length, Time, Volume, Weight};
pub use messages::TurnAdvanced;
pub use schedule::{GameSet, SimSet};
pub use sim_id::SimId;
pub use wyrand::WyRand;
