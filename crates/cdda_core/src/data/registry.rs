use crate::data::raw_defs::*;
use crate::data::raw_types::DefId;
use std::collections::HashMap;
use std::sync::Arc;

/// The single authoritative read-only store of all game definitions.
///
/// Populated by the two-pass loader (`crate::data::loader`) and made available to
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
        }
    }

    pub fn total_count(&self) -> usize {
        self.items.len()
            + self.monsters.len()
            + self.terrain.len()
            + self.furniture.len()
            + self.recipes.len()
            + self.item_groups.len()
            + self.mapgen.values().map(|v| v.len()).sum::<usize>()
            + self.nested_mapgen.len()
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
            + self.json_flags.len()
            + self.ascii_art.len()
            + self.construction_groups.len()
            + self.item_actions.len()
            + self.techniques.len()
            + self.ammunition_types.len()
            + self.morale_types.len()
            + self.scent_types.len()
            + self.movement_modes.len()
            + self.mood_faces.len()
            + self.achievements.len()
            + self.body_parts.len()
            + self.dreams.len()
            + self.emits.len()
            + self.event_statistics.len()
            + self.harvests.len()
            + self.item_migrations.len()
            + self.monster_groups.len()
            + self.mutation_types.len()
            + self.nested_categories.len()
            + self.practices.len()
            + self.professions.len()
            + self.proficiencies.len()
            + self.scores.len()
            + self.species.len()
            + self.sub_body_parts.len()
            + self.uncrafts.len()
            + self.vitamins.len()
            + self.talk_topics.len()
            + self.widgets.len()
            + self.effects_on_condition.len()
            + self.constructions.len()
            + self.snippets.len()
            + self.npcs.len()
            + self.npc_classes.len()
            + self.requirements.len()
            + self.spells.len()
            + self.vehicles.len()
            + self.city_buildings.len()
            + self.mission_definitions.len()
            + self.event_transformations.len()
            + self.martial_arts.len()
            + self.monster_attacks.len()
            + self.weakpoint_sets.len()
            + self.recipe_groups.len()
            + self.monster_flags.len()
            + self.activity_types.len()
            + self.ammo_effects.len()
            + self.tool_qualities.len()
            + self.faults.len()
            + self.map_extras.len()
            + self.fault_fixes.len()
            + self.ter_furn_transforms.len()
            + self.connect_groups.len()
            + self.attack_vectors.len()
            + self.region_terrain_furnitures.len()
            + self.item_categories.len()
            + self.oter_visions.len()
            + self.profession_item_substitutions.len()
            + self.character_mods.len()
            + self.weapon_categories.len()
            + self.rotatable_symbols.len()
            + self.oter_id_migrations.len()
            + self.climbing_aids.len()
            + self.conducts.len()
            + self.weather_types.len()
            + self.proficiency_categories.len()
            + self.faction_missions.len()
            + self.fault_groups.len()
            + self.jmath_functions.len()
            + self.body_graphs.len()
            + self.limb_scores.len()
            + self.construction_categories.len()
            + self.recipe_categories.len()
            + self.addiction_types.len()
            + self.region_settings.len()
            + self.gates.len()
            + self.damage_types.len()
            + self.anatomies.len()
            + self.end_screens.len()
    }

    pub fn category_count(&self) -> usize {
        let mut count = 0;
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
        if !self.json_flags.is_empty() {
            count += 1;
        }
        if !self.ascii_art.is_empty() {
            count += 1;
        }
        if !self.construction_groups.is_empty() {
            count += 1;
        }
        if !self.item_actions.is_empty() {
            count += 1;
        }
        if !self.techniques.is_empty() {
            count += 1;
        }
        if !self.ammunition_types.is_empty() {
            count += 1;
        }
        if !self.morale_types.is_empty() {
            count += 1;
        }
        if !self.scent_types.is_empty() {
            count += 1;
        }
        if !self.movement_modes.is_empty() {
            count += 1;
        }
        if !self.mood_faces.is_empty() {
            count += 1;
        }
        if !self.achievements.is_empty() {
            count += 1;
        }
        if !self.body_parts.is_empty() {
            count += 1;
        }
        if !self.dreams.is_empty() {
            count += 1;
        }
        if !self.emits.is_empty() {
            count += 1;
        }
        if !self.event_statistics.is_empty() {
            count += 1;
        }
        if !self.harvests.is_empty() {
            count += 1;
        }
        if !self.item_migrations.is_empty() {
            count += 1;
        }
        if !self.monster_groups.is_empty() {
            count += 1;
        }
        if !self.mutation_types.is_empty() {
            count += 1;
        }
        if !self.nested_categories.is_empty() {
            count += 1;
        }
        if !self.practices.is_empty() {
            count += 1;
        }
        if !self.professions.is_empty() {
            count += 1;
        }
        if !self.proficiencies.is_empty() {
            count += 1;
        }
        if !self.scores.is_empty() {
            count += 1;
        }
        if !self.species.is_empty() {
            count += 1;
        }
        if !self.sub_body_parts.is_empty() {
            count += 1;
        }
        if !self.uncrafts.is_empty() {
            count += 1;
        }
        if !self.vitamins.is_empty() {
            count += 1;
        }
        if !self.talk_topics.is_empty() {
            count += 1;
        }
        if !self.widgets.is_empty() {
            count += 1;
        }
        if !self.effects_on_condition.is_empty() {
            count += 1;
        }
        if !self.constructions.is_empty() {
            count += 1;
        }
        if !self.snippets.is_empty() {
            count += 1;
        }
        if !self.npcs.is_empty() {
            count += 1;
        }
        if !self.npc_classes.is_empty() {
            count += 1;
        }
        if !self.requirements.is_empty() {
            count += 1;
        }
        if !self.spells.is_empty() {
            count += 1;
        }
        if !self.vehicles.is_empty() {
            count += 1;
        }
        if !self.city_buildings.is_empty() {
            count += 1;
        }
        if !self.mission_definitions.is_empty() {
            count += 1;
        }
        if !self.event_transformations.is_empty() {
            count += 1;
        }
        if !self.martial_arts.is_empty() {
            count += 1;
        }
        if !self.monster_attacks.is_empty() {
            count += 1;
        }
        if !self.weakpoint_sets.is_empty() {
            count += 1;
        }
        if !self.recipe_groups.is_empty() {
            count += 1;
        }
        if !self.monster_flags.is_empty() {
            count += 1;
        }
        if !self.activity_types.is_empty() {
            count += 1;
        }
        if !self.ammo_effects.is_empty() {
            count += 1;
        }
        if !self.tool_qualities.is_empty() {
            count += 1;
        }
        if !self.faults.is_empty() {
            count += 1;
        }
        if !self.map_extras.is_empty() {
            count += 1;
        }
        if !self.fault_fixes.is_empty() {
            count += 1;
        }
        if !self.ter_furn_transforms.is_empty() {
            count += 1;
        }
        if !self.connect_groups.is_empty() {
            count += 1;
        }
        if !self.attack_vectors.is_empty() {
            count += 1;
        }
        if !self.region_terrain_furnitures.is_empty() {
            count += 1;
        }
        if !self.item_categories.is_empty() {
            count += 1;
        }
        if !self.oter_visions.is_empty() {
            count += 1;
        }
        if !self.profession_item_substitutions.is_empty() {
            count += 1;
        }
        if !self.character_mods.is_empty() {
            count += 1;
        }
        if !self.weapon_categories.is_empty() {
            count += 1;
        }
        if !self.rotatable_symbols.is_empty() {
            count += 1;
        }
        if !self.oter_id_migrations.is_empty() {
            count += 1;
        }
        if !self.climbing_aids.is_empty() {
            count += 1;
        }
        if !self.conducts.is_empty() {
            count += 1;
        }
        if !self.weather_types.is_empty() {
            count += 1;
        }
        if !self.proficiency_categories.is_empty() {
            count += 1;
        }
        if !self.faction_missions.is_empty() {
            count += 1;
        }
        if !self.fault_groups.is_empty() {
            count += 1;
        }
        if !self.jmath_functions.is_empty() {
            count += 1;
        }
        if !self.body_graphs.is_empty() {
            count += 1;
        }
        if !self.limb_scores.is_empty() {
            count += 1;
        }
        if !self.construction_categories.is_empty() {
            count += 1;
        }
        if !self.recipe_categories.is_empty() {
            count += 1;
        }
        if !self.addiction_types.is_empty() {
            count += 1;
        }
        if !self.region_settings.is_empty() {
            count += 1;
        }
        if !self.gates.is_empty() {
            count += 1;
        }
        if !self.damage_types.is_empty() {
            count += 1;
        }
        if !self.anatomies.is_empty() {
            count += 1;
        }
        if !self.end_screens.is_empty() {
            count += 1;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
