use crate::raw_defs::*;
use crate::raw_types::DefId;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw_defs::cdda_types::MaterialList;
    use crate::raw_defs::item::Phase;
    use cdda_core::units::Volume;
    use std::sync::Arc;

    // -- helpers to create minimal defs without requiring Default ---------

    fn minimal_item(id: &str, name: &str) -> ItemDef {
        ItemDef {
            id: DefId::new(id),
            name: Some(name.into()),
            description: None,
            volume: Volume::ZERO,
            weight: None,
            count_mode: CountMode::default(),
            category: None,
            material: MaterialList::default(),
            symbol: String::new(),
            color: None,
            price: None,
            price_postapoc: None,
            flags: Vec::new(),
            stackable: None,
            phase: Phase::default(),
            longest_side: None,
            rigid: None,
            conductive: None,
            covers_head: None,
            melee_damage: None,
            pocket_data: None,
            qualities: None,
            capacity: None,
            extra: None,
            snippet_category: None,
            tool: None,
            variants: None,
            techniques: None,
            subtypes: None,
            default_ammo: None,
            max_charges: None,
            initial_charges: None,
            charges: None,
            stack_size: None,
            container: None,
            quench: None,
            ammo_type: None,
            tool_ammo: None,
            spoils_in: None,
            warmth: None,
            comestible_type: None,
            vitamins: None,
            calories: None,
            fun: None,
            material_thickness: None,
            to_hit: None,
            armor: None,
            use_action: None,
            charges_per_use: None,
            power_draw: None,
            revert_to: None,
            looks_like: None,
            abstract_: None,
            copy_from: None,
        }
    }

    fn minimal_monster(id: &str, name: &str, description: &str) -> MonsterDef {
        MonsterDef {
            id: DefId::new(id),
            name: Some(name.into()),
            description: Some(description.into()),
            default_faction: None,
            bodytype: None,
            species: Vec::new(),
            volume: None,
            weight: None,
            hp: 0,
            speed: 0,
            material: None,
            symbol: String::new(),
            color: None,
            aggression: 0,
            morale: 0,
            melee_skill: 0,
            melee_dice: 0,
            melee_dice_sides: 0,
            melee_damage: Vec::new(),
            vision_day: 0,
            vision_night: 0,
            armor: None,
            grab_strength: None,
            special_attacks: Vec::new(),
            death_drops: None,
            burn_into: None,
            fungalize_into: None,
            upgrades: None,
            weakpoint_sets: Vec::new(),
            families: Vec::new(),
            harvest: None,
            decay: None,
            flags: Vec::new(),
            categories: Vec::new(),
            path_settings: None,
            aggro_character: None,
            baby_flags: None,
            move_skills: None,
            looks_like: None,
            fear_triggers: None,
            anger_triggers: None,
            zombify_into: None,
            diff: None,
            death_function: None,
            reproduction: None,
            bleed_rate: None,
            dissect: None,
            dodge: None,
            abstract_: None,
            copy_from: None,
        }
    }

    // -----------------------------------------------------------------------
    // empty()
    // -----------------------------------------------------------------------

    /// An empty registry should have zero items in all categories.
    #[test]
    fn empty_registry_has_zero_items() {
        // Act
        let reg = DefRegistry::empty();

        // Assert
        assert_eq!(reg.items.len(), 0);
        assert_eq!(reg.monsters.len(), 0);
        assert_eq!(reg.total_count(), 0);
        assert_eq!(reg.category_count(), 0);
    }

    // -----------------------------------------------------------------------
    // total_count
    // -----------------------------------------------------------------------

    /// total_count should sum across all categories.
    #[test]
    fn total_count_sums_categories() {
        // Arrange
        let mut reg = DefRegistry::empty();
        reg.items
            .insert(DefId::new("a"), Arc::new(minimal_item("a", "Item A")));
        reg.monsters.insert(
            DefId::new("m1"),
            Arc::new(minimal_monster("m1", "Monster", "Desc")),
        );

        // Act
        let total = reg.total_count();

        // Assert
        assert_eq!(total, 2);
    }

    // -----------------------------------------------------------------------
    // category_count
    // -----------------------------------------------------------------------

    /// category_count should count only non-empty categories.
    #[test]
    fn category_count_counts_populated() {
        // Arrange
        let mut reg = DefRegistry::empty();
        reg.items
            .insert(DefId::new("a"), Arc::new(minimal_item("a", "A")));

        // Act
        let count = reg.category_count();

        // Assert
        assert_eq!(count, 1, "Only items category is populated");
    }

    /// An empty registry should have zero populated categories.
    #[test]
    fn category_count_zero_when_empty() {
        // Arrange
        let reg = DefRegistry::empty();

        // Act
        let count = reg.category_count();

        // Assert
        assert_eq!(count, 0);
    }

    // -----------------------------------------------------------------------
    // Insert & retrieve
    // -----------------------------------------------------------------------

    /// Items inserted by DefId should be retrievable by the same DefId.
    #[test]
    fn insert_and_retrieve_item_by_def_id() {
        // Arrange
        let mut reg = DefRegistry::empty();
        let item = Arc::new(minimal_item("crowbar", "Crowbar"));
        let key = DefId::new("crowbar");

        // Act
        reg.items.insert(key.clone(), item.clone());
        let retrieved = reg.items.get(&key);

        // Assert
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().name.as_ref().unwrap().singular(),
            "Crowbar"
        );
    }

    /// Last insert with same DefId should overwrite the previous value.
    #[test]
    fn insert_same_id_overwrites() {
        // Arrange
        let mut reg = DefRegistry::empty();
        let key = DefId::new("key");
        let first = Arc::new(minimal_item("key", "First"));
        let second = Arc::new(minimal_item("key", "Second"));

        // Act
        reg.items.insert(key.clone(), first);
        reg.items.insert(key.clone(), second);
        let retrieved = reg.items.get(&key);

        // Assert
        assert_eq!(
            retrieved.unwrap().name.as_ref().unwrap().singular(),
            "Second"
        );
    }

    // -----------------------------------------------------------------------
    // Clone
    // -----------------------------------------------------------------------

    /// A cloned registry should contain the same data independently.
    #[test]
    fn clone_registry_preserves_data() {
        // Arrange
        let mut reg = DefRegistry::empty();
        let key = DefId::new("clone_test");
        reg.items.insert(
            key.clone(),
            Arc::new(minimal_item("clone_test", "CloneItem")),
        );

        // Act
        let cloned = reg.clone();
        let retrieved = cloned.items.get(&key);

        // Assert
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().name.as_ref().unwrap().singular(),
            "CloneItem"
        );
    }

    /// Modifying a cloned registry should not affect the original.
    #[test]
    fn clone_is_independent() {
        // Arrange
        let mut original = DefRegistry::empty();
        let key1 = DefId::new("original_item");
        original.items.insert(
            key1.clone(),
            Arc::new(minimal_item("original_item", "Original")),
        );
        let mut cloned = original.clone();

        // Act — modify clone
        let key2 = DefId::new("clone_only");
        cloned.items.insert(
            key2.clone(),
            Arc::new(minimal_item("clone_only", "CloneOnly")),
        );

        // Assert
        assert_eq!(original.items.len(), 1);
        assert_eq!(cloned.items.len(), 2);
        assert!(original.items.get(&key2).is_none());
        assert!(cloned.items.get(&key2).is_some());
    }
}
