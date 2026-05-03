//! Game definition types — each module maps to one or more JSON `"type"` values.
//!
//! These structs are the typed representations of CDDA's JSON data definitions.
//! They are populated by `cdda_data::loader` and resolved by `cdda_data::resolve`.

pub mod bionic;
pub mod cdda_types;
pub mod effect;
pub mod faction;
pub mod field;
pub mod furniture;
pub mod item;
pub mod item_group;
pub mod mapgen;
pub mod material;
pub mod monster;
pub mod mutation;
pub mod overmap_terrain;
pub mod recipe;
pub mod scenario;
pub mod skill;
pub mod start_location;
pub mod terrain;
pub mod trap;
pub mod vehicle_part;

pub use cdda_types::*;

// Re-exports for convenience
pub use bionic::{BionicDef, BionicGroupDef};
pub use effect::EffectDef;
pub use faction::FactionDef;
pub use field::FieldDef;
pub use furniture::FurnitureDef;
pub use item::{CountMode, ItemDef, PocketDef, PocketType};
pub use item_group::{ItemGroupDef, ItemGroupEntry, ItemGroupSubtype};
pub use mapgen::{MapgenDef, MapgenPaletteDef};
pub use material::MaterialDef;
pub use monster::MonsterDef;
pub use mutation::{MutationCategoryDef, MutationDef, TraitGroupDef};
pub use overmap_terrain::{
    OvermapConnectionDef, OvermapLandUseCodeDef, OvermapLocationDef, OvermapSpecialDef,
    OvermapTerrainDef,
};
pub use recipe::RecipeDef;
pub use scenario::ScenarioDef;
pub use skill::SkillDef;
pub use start_location::StartLocationDef;
pub use terrain::TerrainDef;
pub use trap::TrapDef;
pub use vehicle_part::{VehiclePartCategoryDef, VehiclePartDef, VehiclePartLocationDef};
