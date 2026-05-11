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

pub use cdda_activity as activity;
pub use cdda_actor as actor;
pub use cdda_ai as ai;
pub use cdda_combat as combat;
pub mod context;
pub mod core;
pub mod crafting;
pub use cdda_data as data;
pub use cdda_equipment as equipment;
pub mod input;
pub mod inventory;
pub use cdda_item as item;
pub use cdda_map as map;
pub use cdda_replay as replay;
pub use cdda_sim as sim;
pub mod startup;
pub mod worldgen;

// Re-export key types — all point to cdda_core_types for the canonical definitions
pub use cdda_components::stats::Stats;
pub use cdda_core_types::core::coords::{
    BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos,
};
pub use cdda_core_types::core::coords::{Direction, Facing, ZLevel};
pub use cdda_core_types::core::coords::{VehicleMapPos, VehicleMountPos};
pub use cdda_core_types::core::damage::Damage;
pub use cdda_core_types::core::error::CoreError;
pub use cdda_core_types::core::flags::FlagSet;
pub use cdda_core_types::core::id::{
    AmmoTypeId, BodyPartId, DamageTypeId, MaterialId, SpeciesId, VitaminId,
};
pub use cdda_core_types::core::id::{BionicId, EffectId, FactionId, SkillId};
pub use cdda_core_types::core::id::{DefCategory, DefIdx, GenId};
pub use cdda_core_types::core::id::{
    FieldId, ItemGroupId, MutationCategoryId, MutationId, TraitGroupId,
};
pub use cdda_core_types::core::id::{FurnitureId, ItemId, MonsterId, RecipeId, TerrainId};
pub use cdda_core_types::core::id::{MapgenPaletteId, OvermapSpecialId, OvermapTerrainId};
pub use cdda_core_types::core::id::{OvermapConnectionId, OvermapLandUseCodeId, OvermapLocationId};
pub use cdda_core_types::core::id::{ProfessionId, ProficiencyId, QualityId};
pub use cdda_core_types::core::id::{
    ScenarioId, SpecialAttackId, StartLocationId, TechniqueId, TrapId,
};
pub use cdda_core_types::core::id::{VehiclePartCategoryId, VehiclePartId, VehiclePartLocationId};
pub use cdda_core_types::core::units::{Energy, Length, Time, Volume, Weight};
pub use cdda_core_types::rng;
pub use cdda_core_types::rng::SeededRng;
pub use cdda_core_types::sim_id::SimId;
pub use cdda_core_types::wyrand::WyRand;
pub use cdda_components::messages::TurnAdvanced;
pub use cdda_components::schedule::{GameSet, SimSet};
