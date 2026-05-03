//! # Definition registry
//!
//! [`DefRegistry`] is the single authoritative read-only store of all game
//! definitions.  Each category is stored as a `Vec<T>` indexed by [`DefIdx`]
//! for O(1) lookup.  String↔numeric maps are built at load time and used
//! only at I/O boundaries.

use std::collections::HashMap;

use crate::id::*;
use crate::templates::*;

/// The single authoritative read-only store of all game definitions.
///
/// Each category is stored as `Vec<T>` indexed by [`DefIdx`].0 for O(1)
/// lookup.  String maps are built at load time and used only at I/O
/// boundaries.
#[derive(Debug, Clone)]
#[allow(dead_code)] // string maps not yet wired to ACL
pub struct DefRegistry {
    pub items: Vec<ItemTemplate>,
    pub monsters: Vec<MonsterTemplate>,
    pub terrain: Vec<TerrainTemplate>,
    pub furniture: Vec<FurnitureTemplate>,
    pub recipes: Vec<RecipeTemplate>,
    pub item_groups: Vec<ItemGroupTemplate>,
    pub mapgen_palettes: Vec<MapgenPaletteTemplate>,
    pub overmap_terrains: Vec<OvermapTerrainTemplate>,
    pub overmap_specials: Vec<OvermapSpecialTemplate>,
    pub overmap_connections: Vec<OvermapConnectionTemplate>,
    pub overmap_locations: Vec<OvermapLocationTemplate>,
    pub overmap_land_use_codes: Vec<OvermapLandUseCodeTemplate>,
    pub fields: Vec<FieldTemplate>,
    pub vehicle_parts: Vec<VehiclePartTemplate>,
    pub vehicle_part_locations: Vec<VehiclePartLocationTemplate>,
    pub vehicle_part_categories: Vec<VehiclePartCategoryTemplate>,
    pub mutations: Vec<MutationTemplate>,
    pub mutation_categories: Vec<MutationCategoryTemplate>,
    pub trait_groups: Vec<TraitGroupTemplate>,
    pub bionics: Vec<BionicTemplate>,
    pub effects: Vec<EffectTemplate>,
    pub factions: Vec<FactionTemplate>,
    pub skills: Vec<SkillTemplate>,
    pub materials: Vec<MaterialTemplate>,
    pub traps: Vec<TrapTemplate>,
    pub start_locations: Vec<StartLocationTemplate>,
    pub scenarios: Vec<ScenarioTemplate>,

    // String↔numeric maps — built at load, used at I/O boundaries.
    pub(crate) item_ids: HashMap<String, DefIdx>,
    pub(crate) item_names: Vec<String>,
    pub(crate) monster_ids: HashMap<String, DefIdx>,
    pub(crate) monster_names: Vec<String>,
    pub(crate) terrain_ids: HashMap<String, DefIdx>,
    pub(crate) terrain_names: Vec<String>,
    pub(crate) furniture_ids: HashMap<String, DefIdx>,
    pub(crate) furniture_names: Vec<String>,
    pub(crate) recipe_ids: HashMap<String, DefIdx>,
    pub(crate) recipe_names: Vec<String>,
    pub(crate) field_ids: HashMap<String, DefIdx>,
    pub(crate) field_names: Vec<String>,
    pub(crate) skill_ids: HashMap<String, DefIdx>,
    pub(crate) skill_names: Vec<String>,
}
