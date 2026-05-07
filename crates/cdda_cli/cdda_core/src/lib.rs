//! # cdda_core — Domain types and units
//!
//! The game's domain layer: value objects (Volume, Weight, WorldPos),
//! pure component templates, and numeric ID types.
//!
//! Thin `bevy_ecs` dependency for `SystemSet` labels, common `Message`
//! types, `SimId`, and `WyRand`. Serde is allowed — domain types must
//! be serializable for save/load.
//! This is the lowest crate in the dependency graph.

pub mod coords;
pub mod damage;
pub mod def_kinds;
pub mod error;
pub mod flags;
pub mod id;
pub mod id_slab;
pub mod id_str;
pub mod messages;
pub mod rng;
pub mod schedule;
pub mod sim_id;
pub mod stats;
pub mod units;
pub mod wyrand;

// Re-export key types
pub use coords::{BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos};
pub use coords::{Direction, Facing, ZLevel};
pub use coords::{VehicleMapPos, VehicleMountPos};
pub use damage::Damage;
pub use error::CoreError;
pub use flags::FlagSet;
pub use messages::TurnAdvanced;
pub use schedule::{GameSet, SimSet};
pub use sim_id::SimId;
pub use stats::Stats;
pub use units::{Energy, Length, Time, Volume, Weight};
pub use wyrand::WyRand;

// ID types
pub use id::{AmmoTypeId, BodyPartId, DamageTypeId, MaterialId, SpeciesId, VitaminId};
pub use id::{BionicId, EffectId, FactionId, SkillId};
pub use id::{DefCategory, DefIdx, GenId};
pub use id::{FieldId, ItemGroupId, MutationCategoryId, MutationId, TraitGroupId};
pub use id::{FurnitureId, ItemId, MonsterId, RecipeId, TerrainId};
pub use id::{MapgenPaletteId, OvermapSpecialId, OvermapTerrainId};
pub use id::{OvermapConnectionId, OvermapLandUseCodeId, OvermapLocationId};
pub use id::{ProfessionId, ProficiencyId, QualityId};
pub use id::{ScenarioId, SpecialAttackId, StartLocationId, TechniqueId, TrapId};
pub use id::{VehiclePartCategoryId, VehiclePartId, VehiclePartLocationId};
