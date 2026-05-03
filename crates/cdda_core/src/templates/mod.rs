//! # Template types
//!
//! Domain types that describe *definitions* in the JSON data.  These are the
//! pure data that drives item, monster, terrain, etc. behaviour.  They are
//! **not** ECS components yet — they are the blueprint from which components
//! are spawned.
//!
//! ## Design rules
//!
//! * No `serde`, no `schemars` — these are domain types, not JSON mirrors.
//! * No `Option`-for-everything — optional behaviour uses `Option<SubBehaviour>`.
//! * Every struct gets `#[derive(Debug, Clone, PartialEq)]`.
//! * All referenced types come from `crate::id`, `crate::units`, or local
//!   sub-modules.

pub mod bionic;
pub mod effect;
pub mod faction;
pub mod field;
pub mod furniture;
pub mod item;
pub mod item_group;
pub mod mapgen_palette;
pub mod material;
pub mod monster;
pub mod mutation;
pub mod mutation_category;
pub mod overmap_connection;
pub mod overmap_land_use_code;
pub mod overmap_location;
pub mod overmap_special;
pub mod overmap_terrain;
pub mod recipe;
pub mod scenario;
pub mod skill;
pub mod start_location;
pub mod terrain;
pub mod trait_group;
pub mod trap;
pub mod vehicle_part;
pub mod vehicle_part_category;
pub mod vehicle_part_location;

pub use bionic::BionicTemplate;
pub use effect::EffectTemplate;
pub use faction::FactionTemplate;
pub use field::{FieldTag, FieldTemplate};
pub use furniture::FurnitureTemplate;
pub use item::{
    AmmoBehavior, ArmorBehavior, BookBehavior, ContainerBehavior, ContainerTag, CountMode,
    FoodBehavior, ItemBase, ItemTemplate, MagazineBehavior, Phase, ToolBehavior, ToolTag,
    WeaponBehavior,
};
pub use item_group::{ItemGroupEntry, ItemGroupSubtype, ItemGroupTemplate};
pub use mapgen_palette::MapgenPaletteTemplate;
pub use material::MaterialTemplate;
pub use monster::{ArmorSet, MonsterBase, MonsterStats, MonsterTemplate, Vision};
pub use mutation::MutationTemplate;
pub use mutation_category::MutationCategoryTemplate;
pub use overmap_connection::OvermapConnectionTemplate;
pub use overmap_land_use_code::{OvermapLandUseCodeTag, OvermapLandUseCodeTemplate};
pub use overmap_location::OvermapLocationTemplate;
pub use overmap_special::{OvermapOccurrence, OvermapSpecialTemplate};
pub use overmap_terrain::OvermapTerrainTemplate;
pub use recipe::RecipeTemplate;
pub use scenario::ScenarioTemplate;
pub use skill::SkillTemplate;
pub use start_location::StartLocationTemplate;
pub use terrain::TerrainTemplate;
pub use trait_group::TraitGroupTemplate;
pub use trap::{TrapTag, TrapTemplate};
pub use vehicle_part::VehiclePartTemplate;
pub use vehicle_part_category::VehiclePartCategoryTemplate;
pub use vehicle_part_location::VehiclePartLocationTemplate;
