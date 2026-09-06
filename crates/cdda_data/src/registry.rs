use cdda_core_types::core::id::DefId;
use cdda_defs_raw::raw_defs::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::for_each_raw_def_kind;

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
    pub items: HashMap<DefId<ItemDef>, Arc<ItemDef>>,
    pub monsters: HashMap<DefId<MonsterDef>, Arc<MonsterDef>>,
    pub terrain: HashMap<DefId<TerrainDef>, Arc<TerrainDef>>,
    pub furniture: HashMap<DefId<FurnitureDef>, Arc<FurnitureDef>>,
    pub recipes: HashMap<DefId<RecipeDef>, Arc<RecipeDef>>,
    pub item_groups: HashMap<DefId<ItemGroupDef>, Arc<ItemGroupDef>>,
    pub mapgen: HashMap<String, Vec<Arc<MapgenDef>>>,
    pub nested_mapgen: HashMap<String, Arc<MapgenDef>>,
    pub palettes: HashMap<DefId<MapgenPaletteDef>, Arc<MapgenPaletteDef>>,
    pub overmap_terrains: HashMap<DefId<OvermapTerrainDef>, Arc<OvermapTerrainDef>>,
    pub overmap_specials: HashMap<DefId<OvermapSpecialDef>, Arc<OvermapSpecialDef>>,
    pub overmap_connections: HashMap<DefId<OvermapConnectionDef>, Arc<OvermapConnectionDef>>,
    pub overmap_locations: HashMap<DefId<OvermapLocationDef>, Arc<OvermapLocationDef>>,
    pub overmap_land_use_codes: HashMap<DefId<OvermapLandUseCodeDef>, Arc<OvermapLandUseCodeDef>>,
    pub fields: HashMap<DefId<FieldDef>, Arc<FieldDef>>,
    pub vehicle_parts: HashMap<DefId<VehiclePartDef>, Arc<VehiclePartDef>>,
    pub vehicle_part_locations: HashMap<DefId<VehiclePartLocationDef>, Arc<VehiclePartLocationDef>>,
    pub vehicle_part_categories:
        HashMap<DefId<VehiclePartCategoryDef>, Arc<VehiclePartCategoryDef>>,
    pub mutations: HashMap<DefId<MutationDef>, Arc<MutationDef>>,
    pub mutation_categories: HashMap<DefId<MutationCategoryDef>, Arc<MutationCategoryDef>>,
    pub trait_groups: HashMap<DefId<TraitGroupDef>, Arc<TraitGroupDef>>,
    pub bionics: HashMap<DefId<BionicDef>, Arc<BionicDef>>,
    pub effects: HashMap<DefId<EffectDef>, Arc<EffectDef>>,
    pub factions: HashMap<DefId<FactionDef>, Arc<FactionDef>>,
    pub scenarios: HashMap<DefId<ScenarioDef>, Arc<ScenarioDef>>,
    pub materials: HashMap<DefId<MaterialDef>, Arc<MaterialDef>>,
    pub skills: HashMap<DefId<SkillDef>, Arc<SkillDef>>,
    pub traps: HashMap<DefId<TrapDef>, Arc<TrapDef>>,
    pub start_locations: HashMap<DefId<StartLocationDef>, Arc<StartLocationDef>>,
    pub json_flags: HashMap<DefId<JsonFlagDef>, Arc<JsonFlagDef>>,
    pub ascii_art: HashMap<DefId<AsciiArtDef>, Arc<AsciiArtDef>>,
    pub construction_groups: HashMap<DefId<ConstructionGroupDef>, Arc<ConstructionGroupDef>>,
    pub item_actions: HashMap<DefId<ItemActionDef>, Arc<ItemActionDef>>,
    pub techniques: HashMap<DefId<TechniqueDef>, Arc<TechniqueDef>>,
    pub ammunition_types: HashMap<DefId<AmmunitionTypeDef>, Arc<AmmunitionTypeDef>>,
    pub morale_types: HashMap<DefId<MoraleTypeDef>, Arc<MoraleTypeDef>>,
    pub scent_types: HashMap<DefId<ScentTypeDef>, Arc<ScentTypeDef>>,
    pub movement_modes: HashMap<DefId<MovementModeDef>, Arc<MovementModeDef>>,
    pub mood_faces: HashMap<DefId<MoodFaceDef>, Arc<MoodFaceDef>>,
    pub achievements: HashMap<DefId<AchievementDef>, Arc<AchievementDef>>,
    pub body_parts: HashMap<DefId<BodyPartDef>, Arc<BodyPartDef>>,
    pub dreams: HashMap<DefId<DreamDef>, Arc<DreamDef>>,
    pub emits: HashMap<DefId<EmitDef>, Arc<EmitDef>>,
    pub event_statistics: HashMap<DefId<EventStatisticDef>, Arc<EventStatisticDef>>,
    pub harvests: HashMap<DefId<HarvestDef>, Arc<HarvestDef>>,
    pub item_migrations: HashMap<DefId<ItemMigrationDef>, Arc<ItemMigrationDef>>,
    pub monster_groups: HashMap<DefId<MonsterGroupDef>, Arc<MonsterGroupDef>>,
    pub mutation_types: HashMap<DefId<MutationTypeDef>, Arc<MutationTypeDef>>,
    pub nested_categories: HashMap<DefId<NestedCategoryDef>, Arc<NestedCategoryDef>>,
    pub practices: HashMap<DefId<PracticeDef>, Arc<PracticeDef>>,
    pub professions: HashMap<DefId<ProfessionDef>, Arc<ProfessionDef>>,
    pub proficiencies: HashMap<DefId<ProficiencyDef>, Arc<ProficiencyDef>>,
    pub scores: HashMap<DefId<ScoreDef>, Arc<ScoreDef>>,
    pub species: HashMap<DefId<SpeciesDef>, Arc<SpeciesDef>>,
    pub sub_body_parts: HashMap<DefId<SubBodyPartDef>, Arc<SubBodyPartDef>>,
    pub uncrafts: HashMap<DefId<UncraftDef>, Arc<UncraftDef>>,
    pub vitamins: HashMap<DefId<VitaminDef>, Arc<VitaminDef>>,
    pub talk_topics: HashMap<DefId<TalkTopicDef>, Arc<TalkTopicDef>>,
    pub widgets: HashMap<DefId<WidgetDef>, Arc<WidgetDef>>,
    pub effects_on_condition: HashMap<DefId<EffectOnConditionDef>, Arc<EffectOnConditionDef>>,
    pub constructions: HashMap<DefId<ConstructionDef>, Arc<ConstructionDef>>,
    pub snippets: HashMap<DefId<SnippetDef>, Arc<SnippetDef>>,
    pub npcs: HashMap<DefId<NpcDef>, Arc<NpcDef>>,
    pub npc_classes: HashMap<DefId<NpcClassDef>, Arc<NpcClassDef>>,
    pub requirements: HashMap<DefId<RequirementDef>, Arc<RequirementDef>>,
    pub spells: HashMap<DefId<SpellDef>, Arc<SpellDef>>,
    pub vehicles: HashMap<DefId<VehicleDef>, Arc<VehicleDef>>,
    pub city_buildings: HashMap<DefId<CityBuildingDef>, Arc<CityBuildingDef>>,
    pub mission_definitions: HashMap<DefId<MissionDefinitionDef>, Arc<MissionDefinitionDef>>,
    pub event_transformations: HashMap<DefId<EventTransformationDef>, Arc<EventTransformationDef>>,
    pub martial_arts: HashMap<DefId<MartialArtDef>, Arc<MartialArtDef>>,
    pub monster_attacks: HashMap<DefId<MonsterAttackDef>, Arc<MonsterAttackDef>>,
    pub weakpoint_sets: HashMap<DefId<WeakpointSetDef>, Arc<WeakpointSetDef>>,
    pub recipe_groups: HashMap<DefId<RecipeGroupDef>, Arc<RecipeGroupDef>>,
    pub monster_flags: HashMap<DefId<MonsterFlagDef>, Arc<MonsterFlagDef>>,
    pub activity_types: HashMap<DefId<ActivityTypeDef>, Arc<ActivityTypeDef>>,
    pub ammo_effects: HashMap<DefId<AmmoEffectDef>, Arc<AmmoEffectDef>>,
    pub tool_qualities: HashMap<DefId<ToolQualityDef>, Arc<ToolQualityDef>>,
    pub faults: HashMap<DefId<FaultDef>, Arc<FaultDef>>,
    pub map_extras: HashMap<DefId<MapExtraDef>, Arc<MapExtraDef>>,
    pub fault_fixes: HashMap<DefId<FaultFixDef>, Arc<FaultFixDef>>,
    pub ter_furn_transforms: HashMap<DefId<TerFurnTransformDef>, Arc<TerFurnTransformDef>>,
    pub connect_groups: HashMap<DefId<ConnectGroupDef>, Arc<ConnectGroupDef>>,
    pub attack_vectors: HashMap<DefId<AttackVectorDef>, Arc<AttackVectorDef>>,
    pub region_terrain_furnitures:
        HashMap<DefId<RegionTerrainFurnitureDef>, Arc<RegionTerrainFurnitureDef>>,
    pub item_categories: HashMap<DefId<ItemCategoryDef>, Arc<ItemCategoryDef>>,
    pub oter_visions: HashMap<DefId<OterVisionDef>, Arc<OterVisionDef>>,
    pub profession_item_substitutions:
        HashMap<DefId<ProfessionItemSubstitutionsDef>, Arc<ProfessionItemSubstitutionsDef>>,
    pub character_mods: HashMap<DefId<CharacterModDef>, Arc<CharacterModDef>>,
    pub weapon_categories: HashMap<DefId<WeaponCategoryDef>, Arc<WeaponCategoryDef>>,
    pub rotatable_symbols: HashMap<DefId<RotatableSymbolDef>, Arc<RotatableSymbolDef>>,
    pub oter_id_migrations: HashMap<DefId<OterIdMigrationDef>, Arc<OterIdMigrationDef>>,
    pub climbing_aids: HashMap<DefId<ClimbingAidDef>, Arc<ClimbingAidDef>>,
    pub conducts: HashMap<DefId<ConductDef>, Arc<ConductDef>>,
    pub weather_types: HashMap<DefId<WeatherTypeDef>, Arc<WeatherTypeDef>>,
    pub proficiency_categories: HashMap<DefId<ProficiencyCategoryDef>, Arc<ProficiencyCategoryDef>>,
    pub faction_missions: HashMap<DefId<FactionMissionDef>, Arc<FactionMissionDef>>,
    pub fault_groups: HashMap<DefId<FaultGroupDef>, Arc<FaultGroupDef>>,
    pub jmath_functions: HashMap<DefId<JmathFunctionDef>, Arc<JmathFunctionDef>>,
    pub body_graphs: HashMap<DefId<BodyGraphDef>, Arc<BodyGraphDef>>,
    pub limb_scores: HashMap<DefId<LimbScoreDef>, Arc<LimbScoreDef>>,
    pub construction_categories:
        HashMap<DefId<ConstructionCategoryDef>, Arc<ConstructionCategoryDef>>,
    pub recipe_categories: HashMap<DefId<RecipeCategoryDef>, Arc<RecipeCategoryDef>>,
    pub addiction_types: HashMap<DefId<AddictionTypeDef>, Arc<AddictionTypeDef>>,
    pub region_settings: HashMap<DefId<RegionSettingsDef>, Arc<RegionSettingsDef>>,
    pub gates: HashMap<DefId<GateDef>, Arc<GateDef>>,
    pub damage_types: HashMap<DefId<DamageTypeDef>, Arc<DamageTypeDef>>,
    pub anatomies: HashMap<DefId<AnatomyDef>, Arc<AnatomyDef>>,
    pub end_screens: HashMap<DefId<EndScreenDef>, Arc<EndScreenDef>>,
    /// Data-authored HTN compound tasks (`"type": "htn_compound"`). Consumed
    /// by the HTN domain compiler (`cdda_sim::ai::htn`), not by def-world
    /// spawning — the planner bakes them into a `cdda_htn` domain.
    pub htn_compounds: HashMap<DefId<HtnCompoundDef>, Arc<HtnCompoundDef>>,
}

impl DefRegistry {
    pub fn empty() -> Self {
        Self {
            items: HashMap::new(),
            monsters: HashMap::new(),
            terrain: HashMap::new(),
            furniture: HashMap::new(),
            recipes: HashMap::new(),
            item_groups: HashMap::new(),
            mapgen: HashMap::new(),
            nested_mapgen: HashMap::new(),
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
            json_flags: HashMap::new(),
            ascii_art: HashMap::new(),
            construction_groups: HashMap::new(),
            item_actions: HashMap::new(),
            techniques: HashMap::new(),
            ammunition_types: HashMap::new(),
            morale_types: HashMap::new(),
            scent_types: HashMap::new(),
            movement_modes: HashMap::new(),
            mood_faces: HashMap::new(),
            achievements: HashMap::new(),
            body_parts: HashMap::new(),
            dreams: HashMap::new(),
            emits: HashMap::new(),
            event_statistics: HashMap::new(),
            harvests: HashMap::new(),
            item_migrations: HashMap::new(),
            monster_groups: HashMap::new(),
            mutation_types: HashMap::new(),
            nested_categories: HashMap::new(),
            practices: HashMap::new(),
            professions: HashMap::new(),
            proficiencies: HashMap::new(),
            scores: HashMap::new(),
            species: HashMap::new(),
            sub_body_parts: HashMap::new(),
            uncrafts: HashMap::new(),
            vitamins: HashMap::new(),
            talk_topics: HashMap::new(),
            widgets: HashMap::new(),
            effects_on_condition: HashMap::new(),
            constructions: HashMap::new(),
            snippets: HashMap::new(),
            npcs: HashMap::new(),
            npc_classes: HashMap::new(),
            requirements: HashMap::new(),
            spells: HashMap::new(),
            vehicles: HashMap::new(),
            city_buildings: HashMap::new(),
            mission_definitions: HashMap::new(),
            event_transformations: HashMap::new(),
            martial_arts: HashMap::new(),
            monster_attacks: HashMap::new(),
            weakpoint_sets: HashMap::new(),
            recipe_groups: HashMap::new(),
            monster_flags: HashMap::new(),
            activity_types: HashMap::new(),
            ammo_effects: HashMap::new(),
            tool_qualities: HashMap::new(),
            faults: HashMap::new(),
            map_extras: HashMap::new(),
            fault_fixes: HashMap::new(),
            ter_furn_transforms: HashMap::new(),
            connect_groups: HashMap::new(),
            attack_vectors: HashMap::new(),
            region_terrain_furnitures: HashMap::new(),
            item_categories: HashMap::new(),
            oter_visions: HashMap::new(),
            profession_item_substitutions: HashMap::new(),
            character_mods: HashMap::new(),
            weapon_categories: HashMap::new(),
            rotatable_symbols: HashMap::new(),
            oter_id_migrations: HashMap::new(),
            climbing_aids: HashMap::new(),
            conducts: HashMap::new(),
            weather_types: HashMap::new(),
            proficiency_categories: HashMap::new(),
            faction_missions: HashMap::new(),
            fault_groups: HashMap::new(),
            jmath_functions: HashMap::new(),
            body_graphs: HashMap::new(),
            limb_scores: HashMap::new(),
            construction_categories: HashMap::new(),
            recipe_categories: HashMap::new(),
            addiction_types: HashMap::new(),
            region_settings: HashMap::new(),
            gates: HashMap::new(),
            damage_types: HashMap::new(),
            anatomies: HashMap::new(),
            end_screens: HashMap::new(),
            htn_compounds: HashMap::new(),
        }
    }
}

impl DefRegistry {
    pub fn total_count(&self) -> usize {
        let mut total = 0usize;
        macro_rules! count_len {
            ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {
                total += self.$field.len();
            };
        }
        for_each_raw_def_kind!(call count_len);
        // `mapgen` / `nested_mapgen` are special: `String`-keyed, not `DefId<T>`
        // maps, so they are not in the `for_each_raw_def_kind!` table.
        total += self.mapgen.values().map(Vec::len).sum::<usize>();
        total += self.nested_mapgen.len();
        total
    }

    pub fn category_count(&self) -> usize {
        let mut count = 0usize;
        macro_rules! count_present {
            ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {
                count += usize::from(!self.$field.is_empty());
            };
        }
        for_each_raw_def_kind!(call count_present);
        count += usize::from(!self.mapgen.is_empty());
        count += usize::from(!self.nested_mapgen.is_empty());
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// The macro-driven `total_count` / `category_count` must reflect the
    /// per-category maps. Populate several distinct categories and assert the
    /// aggregates add them in (guards against a table/field desync). The entries
    /// are built via serde (minimal JSON) rather than field-by-field literals,
    /// so the test does not break when a def struct gains a field.
    #[test]
    fn counts_reflect_populated_fields() {
        let mut reg = DefRegistry::empty();
        assert_eq!(reg.total_count(), 0);
        assert_eq!(reg.category_count(), 0);

        let item: ItemDef = serde_json::from_str(r#"{ "id": "a", "volume": "1 ml" }"#).unwrap();
        reg.items.insert(DefId::from("a"), Arc::new(item));

        // 1 item in `items` → 1 total, 1 category.
        assert_eq!(reg.total_count(), 1);
        assert_eq!(reg.category_count(), 1);

        // Populate a second category: mapgen (special-cased, not table-driven).
        let mg: MapgenDef = serde_json::from_str(r#"{ "om_terrain": "omt_a" }"#).unwrap();
        reg.mapgen.insert("omt_a".to_string(), vec![Arc::new(mg)]);
        assert_eq!(reg.total_count(), 2);
        assert_eq!(reg.category_count(), 2);
    }
}

/// Translate source HTN definitions into the runtime compiler's native input.
impl cdda_catalog::htn::HtnSource for DefRegistry {
    fn htn_program(&self) -> cdda_catalog::htn::HtnProgram {
        use cdda_catalog::htn::*;
        HtnProgram {
            items: self
                .items
                .keys()
                .map(|id| id.as_str().to_string())
                .collect(),
            item_categories: self
                .item_categories
                .keys()
                .map(|id| id.as_str().to_string())
                .collect(),
            htn_compounds: self
                .htn_compounds
                .iter()
                .map(|(id, def)| {
                    (
                        id.as_str().to_string(),
                        Arc::new(Compound {
                            parameters: def.parameters.clone(),
                            methods: def
                                .methods
                                .iter()
                                .map(|m| Method {
                                    id: m.id.clone(),
                                    when: m
                                        .when
                                        .iter()
                                        .map(|p| Predicate {
                                            predicate: p.predicate.clone(),
                                            args: p.args.clone(),
                                        })
                                        .collect(),
                                    steps: m
                                        .steps
                                        .iter()
                                        .map(|s| Step {
                                            operator: s.operator.clone(),
                                            task: s.task.clone(),
                                            args: s.args.clone(),
                                        })
                                        .collect(),
                                })
                                .collect(),
                        }),
                    )
                })
                .collect(),
        }
    }
}
