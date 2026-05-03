//! # cdda_core — Domain types and units
//!
//! The game's domain layer: value objects (Volume, Weight, WorldPos),
//! pure component templates, and numeric ID types.
//!
//! No IO, no Bevy, no serde. This is the lowest crate in the dependency graph.
//! Everything flows upward from here.

pub mod coords;
pub mod damage;
pub mod error;
pub mod flags;
pub mod id;
pub mod id_slab;
pub mod registry;
pub mod rng;
pub mod stats;
pub mod templates;
pub mod units;

// Re-export key types for ergonomic access.
// -- Coords and geometry
pub use coords::{BubblePos, OmPos, OmtPos, Pos, SubmapLocal, SubmapPos, WorldPos};
pub use coords::{Direction, Facing, ZLevel};
pub use coords::{VehicleMapPos, VehicleMountPos};

// -- Pure utilities
pub use damage::Damage;
pub use error::CoreError;
pub use flags::FlagSet;
pub use stats::Stats;
pub use units::{Energy, Length, Time, Volume, Weight};

// -- ID types
pub use id::{AmmoTypeId, BodyPartId, MaterialId, SpeciesId, VitaminId};
pub use id::{BionicId, EffectId, FactionId, SkillId};
pub use id::{DefCategory, DefIdx, GenId};
pub use id::{FieldId, ItemGroupId, MutationCategoryId, MutationId, TraitGroupId};
pub use id::{FurnitureId, ItemId, MonsterId, RecipeId, TerrainId};
pub use id::{MapgenPaletteId, OvermapSpecialId, OvermapTerrainId};
pub use id::{OvermapConnectionId, OvermapLandUseCodeId, OvermapLocationId};
pub use id::{ScenarioId, SpecialAttackId, StartLocationId, TechniqueId, TrapId};
pub use id::{VehiclePartCategoryId, VehiclePartId, VehiclePartLocationId};

// -- Templates
pub use templates::*;
