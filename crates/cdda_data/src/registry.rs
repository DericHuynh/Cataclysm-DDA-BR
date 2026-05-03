use cdda_core::defs::*;
use cdda_core::types::DefId;
use std::collections::HashMap;
use std::sync::Arc;

/// The single authoritative read-only store of all game definitions.
///
/// Populated by the two-pass loader (`crate::loader`) and made available to
/// all other crates. After loading, this is immutable for the lifetime of
/// the game session.
///
/// Each field is a `HashMap` keyed by `DefId<T>` with `Arc`-wrapped values
/// for efficient sharing.
#[derive(Debug, Clone)]
pub struct DefRegistry {
    /// Item definitions.
    pub items: HashMap<DefId<ItemDef>, Arc<ItemDef>>,
    /// Monster definitions.
    pub monsters: HashMap<DefId<MonsterDef>, Arc<MonsterDef>>,
    /// Terrain definitions.
    pub terrain: HashMap<DefId<TerrainDef>, Arc<TerrainDef>>,
    /// Furniture definitions.
    pub furniture: HashMap<DefId<FurnitureDef>, Arc<FurnitureDef>>,
    /// Recipe definitions.
    pub recipes: HashMap<DefId<RecipeDef>, Arc<RecipeDef>>,
    /// Item group definitions.
    pub item_groups: HashMap<DefId<ItemGroupDef>, Arc<ItemGroupDef>>,
    /// Mapgen definitions (one OMT can have multiple mapgen variants).
    pub mapgen: HashMap<DefId<OvermapTerrainDef>, Vec<Arc<MapgenDef>>>,
    /// Mapgen palette definitions.
    pub palettes: HashMap<DefId<MapgenPaletteDef>, Arc<MapgenPaletteDef>>,
    /// Overmap terrain definitions.
    pub overmap_terrains: HashMap<DefId<OvermapTerrainDef>, Arc<OvermapTerrainDef>>,
    /// Overmap special definitions.
    pub overmap_specials: HashMap<DefId<OvermapSpecialDef>, Arc<OvermapSpecialDef>>,
    /// Overmap connection definitions.
    pub overmap_connections: HashMap<DefId<OvermapConnectionDef>, Arc<OvermapConnectionDef>>,
    /// Overmap location definitions.
    pub overmap_locations: HashMap<DefId<OvermapLocationDef>, Arc<OvermapLocationDef>>,
    /// Overmap land use code definitions.
    pub overmap_land_use_codes: HashMap<DefId<OvermapLandUseCodeDef>, Arc<OvermapLandUseCodeDef>>,
    /// Field type definitions.
    pub fields: HashMap<DefId<FieldDef>, Arc<FieldDef>>,
    /// Vehicle part definitions.
    pub vehicle_parts: HashMap<DefId<VehiclePartDef>, Arc<VehiclePartDef>>,
    /// Vehicle part location definitions.
    pub vehicle_part_locations: HashMap<DefId<VehiclePartLocationDef>, Arc<VehiclePartLocationDef>>,
    /// Vehicle part category definitions.
    pub vehicle_part_categories:
        HashMap<DefId<VehiclePartCategoryDef>, Arc<VehiclePartCategoryDef>>,
    /// Mutation definitions.
    pub mutations: HashMap<DefId<MutationDef>, Arc<MutationDef>>,
    /// Mutation category definitions.
    pub mutation_categories: HashMap<DefId<MutationCategoryDef>, Arc<MutationCategoryDef>>,
    /// Trait group definitions.
    pub trait_groups: HashMap<DefId<TraitGroupDef>, Arc<TraitGroupDef>>,
    /// Bionic definitions.
    pub bionics: HashMap<DefId<BionicDef>, Arc<BionicDef>>,
    /// Effect type definitions.
    pub effects: HashMap<DefId<EffectDef>, Arc<EffectDef>>,
    /// Faction definitions.
    pub factions: HashMap<DefId<FactionDef>, Arc<FactionDef>>,
    /// Scenario definitions.
    pub scenarios: HashMap<DefId<ScenarioDef>, Arc<ScenarioDef>>,
    /// Material definitions.
    pub materials: HashMap<DefId<MaterialDef>, Arc<MaterialDef>>,
    /// Skill definitions.
    pub skills: HashMap<DefId<SkillDef>, Arc<SkillDef>>,
    /// Trap definitions.
    pub traps: HashMap<DefId<TrapDef>, Arc<TrapDef>>,
    /// Start location definitions.
    pub start_locations: HashMap<DefId<StartLocationDef>, Arc<StartLocationDef>>,
}

impl DefRegistry {
    /// Create an empty registry.
    pub fn empty() -> Self {
        Self {
            items: HashMap::new(),
            monsters: HashMap::new(),
            terrain: HashMap::new(),
            furniture: HashMap::new(),
            recipes: HashMap::new(),
            item_groups: HashMap::new(),
            mapgen: HashMap::new(),
            palettes: HashMap::new(),
            overmap_terrains: HashMap::new(),
            overmap_specials: HashMap::new(),
            overmap_connections: HashMap::new(),
            overmap_locations: HashMap::new(),
            overmap_land_use_codes: HashMap::new(),
            fields: HashMap::new(),
            vehicle_parts: HashMap::new(),
            vehicle_part_locations: HashMap::new(),
            vehicle_part_categories: HashMap::new(),
            mutations: HashMap::new(),
            mutation_categories: HashMap::new(),
            trait_groups: HashMap::new(),
            bionics: HashMap::new(),
            effects: HashMap::new(),
            factions: HashMap::new(),
            scenarios: HashMap::new(),
            materials: HashMap::new(),
            skills: HashMap::new(),
            traps: HashMap::new(),
            start_locations: HashMap::new(),
        }
    }

    /// Total number of definitions across all categories.
    pub fn total_count(&self) -> usize {
        self.items.len()
            + self.monsters.len()
            + self.terrain.len()
            + self.furniture.len()
            + self.recipes.len()
            + self.item_groups.len()
            + self.mapgen.values().map(|v| v.len()).sum::<usize>()
            + self.palettes.len()
            + self.overmap_terrains.len()
            + self.overmap_specials.len()
            + self.overmap_connections.len()
            + self.overmap_locations.len()
            + self.overmap_land_use_codes.len()
            + self.fields.len()
            + self.vehicle_parts.len()
            + self.vehicle_part_locations.len()
            + self.vehicle_part_categories.len()
            + self.mutations.len()
            + self.mutation_categories.len()
            + self.trait_groups.len()
            + self.bionics.len()
            + self.effects.len()
            + self.factions.len()
            + self.scenarios.len()
            + self.materials.len()
            + self.skills.len()
            + self.traps.len()
            + self.start_locations.len()
    }

    /// Number of populated (non-empty) categories.
    pub fn category_count(&self) -> usize {
        let mut count = 0usize;

        if !self.items.is_empty() {
            count += 1;
        }
        if !self.monsters.is_empty() {
            count += 1;
        }
        if !self.terrain.is_empty() {
            count += 1;
        }
        if !self.furniture.is_empty() {
            count += 1;
        }
        if !self.recipes.is_empty() {
            count += 1;
        }
        if !self.item_groups.is_empty() {
            count += 1;
        }
        if !self.mapgen.is_empty() {
            count += 1;
        }
        if !self.palettes.is_empty() {
            count += 1;
        }
        if !self.overmap_terrains.is_empty() {
            count += 1;
        }
        if !self.overmap_specials.is_empty() {
            count += 1;
        }
        if !self.overmap_connections.is_empty() {
            count += 1;
        }
        if !self.overmap_locations.is_empty() {
            count += 1;
        }
        if !self.overmap_land_use_codes.is_empty() {
            count += 1;
        }
        if !self.fields.is_empty() {
            count += 1;
        }
        if !self.vehicle_parts.is_empty() {
            count += 1;
        }
        if !self.vehicle_part_locations.is_empty() {
            count += 1;
        }
        if !self.vehicle_part_categories.is_empty() {
            count += 1;
        }
        if !self.mutations.is_empty() {
            count += 1;
        }
        if !self.mutation_categories.is_empty() {
            count += 1;
        }
        if !self.trait_groups.is_empty() {
            count += 1;
        }
        if !self.bionics.is_empty() {
            count += 1;
        }
        if !self.effects.is_empty() {
            count += 1;
        }
        if !self.factions.is_empty() {
            count += 1;
        }
        if !self.scenarios.is_empty() {
            count += 1;
        }
        if !self.materials.is_empty() {
            count += 1;
        }
        if !self.skills.is_empty() {
            count += 1;
        }
        if !self.traps.is_empty() {
            count += 1;
        }
        if !self.start_locations.is_empty() {
            count += 1;
        }

        count
    }
}
