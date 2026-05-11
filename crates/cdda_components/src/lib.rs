//! # cdda_components — ECS component definitions
//!
//! All Bevy ECS components for actors, items, definitions, spatial data,
//! and stats.  Re-exports from cdda_core_types so that existing crate
//! paths (e.g. `crate::core::coords::WorldPos`) resolve correctly.

pub use cdda_core_types::{core, rng, sim_id, wyrand};

// Re-export commonly-used ID types at root level for backward compatibility.
pub use cdda_core_types::core::id::{
    AmmoTypeId, BionicId, BodyPartId, DamageTypeId, EffectId, FactionId, FieldId, FurnitureId,
    ItemGroupId, ItemId, MaterialId, MonsterId, MutationCategoryId, MutationId,
    OvermapConnectionId, OvermapLandUseCodeId, OvermapLocationId, OvermapSpecialId,
    OvermapTerrainId, ProfessionId, ProficiencyId, QualityId, RecipeId, ScenarioId, SkillId,
    SpecialAttackId, SpeciesId, StartLocationId, TechniqueId, TerrainId, TraitGroupId, TrapId,
    VehiclePartCategoryId, VehiclePartId, VehiclePartLocationId, VitaminId,
};
pub use cdda_core_types::core::units::{Energy, Length, Time, Volume, Weight};
pub use cdda_core_types::core::Damage;
pub use cdda_core_types::core::WorldPos;

pub mod actor;
pub mod context;
pub mod def;
pub mod dev;
pub mod events;
pub mod input;
pub mod item;
pub mod messages;
pub mod recipe;
pub mod schedule;
pub mod sim;
pub mod stats;
