/// Macro that enumerates every CDDA data-definition kind.
///
/// # Modes
///
/// 1. `for_each_raw_def_kind!(call $t)` — item context.
///    Calls `$t!(Name, DefType, "json_type", field_name, strategy)` once per def kind.
///
/// 2. `for_each_raw_def_kind!(list $mapper)` — expression context.
///    Expands to `vec![ $mapper!(Name, DefType, "json_type", field_name, strategy), … ]`.
///
/// # Strategies
///
/// | Strategy    | ID resolution                                     |
/// |-------------|---------------------------------------------------|
/// | `id`        | standard: uses `"id"` / `"result"` / `"abstract"` |
/// | `name`      | uses `"name"` as fallback (e.g. MONSTER_FACTION)  |
/// | `item`      | uses `"item"` as fallback                         |
/// | `synthetic` | id injected during ingestion                      |
/// | `custom`    | custom resolver needed                            |
///
/// Type paths are prefixed with `$crate::raw_defs::`.
#[macro_export]
macro_rules! for_each_raw_def_kind {
    // ── item-context mode ──────────────────────────────────────────────
    (call $t:ident) => {
        $t!(Item, $crate::raw_defs::ItemDef, "ITEM", items, id);
        $t!(Monster, $crate::raw_defs::MonsterDef, "MONSTER", monsters, id);
        $t!(Terrain, $crate::raw_defs::TerrainDef, "terrain", terrain, id);
        $t!(Furniture, $crate::raw_defs::FurnitureDef, "furniture", furniture, id);
        $t!(Recipe, $crate::raw_defs::RecipeDef, "recipe", recipes, id);
        $t!(ItemGroup, $crate::raw_defs::ItemGroupDef, "item_group", item_groups, id);
        $t!(MapgenPalette, $crate::raw_defs::MapgenPaletteDef, "palette", palettes, id);
        $t!(OvermapTerrain, $crate::raw_defs::OvermapTerrainDef, "overmap_terrain", overmap_terrains, id);
        $t!(OvermapSpecial, $crate::raw_defs::OvermapSpecialDef, "overmap_special", overmap_specials, id);
        $t!(OvermapConnection, $crate::raw_defs::OvermapConnectionDef, "overmap_connection", overmap_connections, id);
        $t!(OvermapLocation, $crate::raw_defs::OvermapLocationDef, "overmap_location", overmap_locations, id);
        $t!(OvermapLandUseCode, $crate::raw_defs::OvermapLandUseCodeDef, "overmap_land_use_code", overmap_land_use_codes, id);
        $t!(Field, $crate::raw_defs::FieldDef, "field_type", fields, id);
        $t!(VehiclePart, $crate::raw_defs::VehiclePartDef, "vehicle_part", vehicle_parts, id);
        $t!(VehiclePartLocation, $crate::raw_defs::VehiclePartLocationDef, "vehicle_part_location", vehicle_part_locations, id);
        $t!(VehiclePartCategory, $crate::raw_defs::VehiclePartCategoryDef, "vehicle_part_category", vehicle_part_categories, id);
        $t!(Mutation, $crate::raw_defs::MutationDef, "mutation", mutations, id);
        $t!(MutationCategory, $crate::raw_defs::MutationCategoryDef, "mutation_category", mutation_categories, id);
        $t!(TraitGroup, $crate::raw_defs::TraitGroupDef, "trait_group", trait_groups, id);
        $t!(Bionic, $crate::raw_defs::BionicDef, "bionic", bionics, id);
        $t!(Effect, $crate::raw_defs::EffectDef, "effect_type", effects, id);
        $t!(Faction, $crate::raw_defs::FactionDef, "faction", factions, id);
        $t!(Scenario, $crate::raw_defs::ScenarioDef, "scenario", scenarios, id);
        $t!(Material, $crate::raw_defs::MaterialDef, "material", materials, id);
        $t!(Skill, $crate::raw_defs::SkillDef, "skill", skills, id);
        $t!(Trap, $crate::raw_defs::TrapDef, "trap", traps, id);
        $t!(StartLocation, $crate::raw_defs::StartLocationDef, "start_location", start_locations, id);
        $t!(JsonFlag, $crate::raw_defs::JsonFlagDef, "json_flag", json_flags, id);
        $t!(AsciiArt, $crate::raw_defs::AsciiArtDef, "ascii_art", ascii_art, id);
        $t!(ConstructionGroup, $crate::raw_defs::ConstructionGroupDef, "construction_group", construction_groups, id);
        $t!(ItemAction, $crate::raw_defs::ItemActionDef, "item_action", item_actions, id);
        $t!(Technique, $crate::raw_defs::TechniqueDef, "technique", techniques, id);
        $t!(AmmunitionType, $crate::raw_defs::AmmunitionTypeDef, "ammunition_type", ammunition_types, id);
        $t!(MoraleType, $crate::raw_defs::MoraleTypeDef, "morale_type", morale_types, id);
        $t!(ScentType, $crate::raw_defs::ScentTypeDef, "scent_type", scent_types, id);
        $t!(MovementMode, $crate::raw_defs::MovementModeDef, "movement_mode", movement_modes, id);
        $t!(MoodFace, $crate::raw_defs::MoodFaceDef, "mood_face", mood_faces, id);
        $t!(Achievement, $crate::raw_defs::AchievementDef, "achievement", achievements, id);
        $t!(BodyPart, $crate::raw_defs::BodyPartDef, "body_part", body_parts, id);
        $t!(Dream, $crate::raw_defs::DreamDef, "dream", dreams, synthetic);
        $t!(Emit, $crate::raw_defs::EmitDef, "emit", emits, id);
        $t!(EventStatistic, $crate::raw_defs::EventStatisticDef, "event_statistic", event_statistics, id);
        $t!(Harvest, $crate::raw_defs::HarvestDef, "harvest", harvests, id);
        $t!(ItemMigration, $crate::raw_defs::ItemMigrationDef, "MIGRATION", item_migrations, id);
        $t!(MonsterGroup, $crate::raw_defs::MonsterGroupDef, "monstergroup", monster_groups, id);
        $t!(MutationType, $crate::raw_defs::MutationTypeDef, "mutation_type", mutation_types, id);
        $t!(NestedCategory, $crate::raw_defs::NestedCategoryDef, "nested_category", nested_categories, id);
        $t!(Practice, $crate::raw_defs::PracticeDef, "practice", practices, id);
        $t!(Profession, $crate::raw_defs::ProfessionDef, "profession", professions, id);
        $t!(Proficiency, $crate::raw_defs::ProficiencyDef, "proficiency", proficiencies, id);
        $t!(Score, $crate::raw_defs::ScoreDef, "score", scores, id);
        $t!(Species, $crate::raw_defs::SpeciesDef, "SPECIES", species, id);
        $t!(SubBodyPart, $crate::raw_defs::SubBodyPartDef, "sub_body_part", sub_body_parts, id);
        $t!(Uncraft, $crate::raw_defs::UncraftDef, "uncraft", uncrafts, id);
        $t!(Vitamin, $crate::raw_defs::VitaminDef, "vitamin", vitamins, id);
        $t!(TalkTopic, $crate::raw_defs::TalkTopicDef, "talk_topic", talk_topics, id);
        $t!(Widget, $crate::raw_defs::WidgetDef, "widget", widgets, id);
        $t!(EffectOnCondition, $crate::raw_defs::EffectOnConditionDef, "effect_on_condition", effects_on_condition, id);
        $t!(Construction, $crate::raw_defs::ConstructionDef, "construction", constructions, id);
        $t!(Snippet, $crate::raw_defs::SnippetDef, "snippet", snippets, custom);
        $t!(Npc, $crate::raw_defs::NpcDef, "npc", npcs, id);
        $t!(NpcClass, $crate::raw_defs::NpcClassDef, "npc_class", npc_classes, id);
        $t!(Requirement, $crate::raw_defs::RequirementDef, "requirement", requirements, id);
        $t!(Spell, $crate::raw_defs::SpellDef, "SPELL", spells, id);
        $t!(Vehicle, $crate::raw_defs::VehicleDef, "vehicle", vehicles, id);
        $t!(CityBuilding, $crate::raw_defs::CityBuildingDef, "city_building", city_buildings, id);
        $t!(MissionDefinition, $crate::raw_defs::MissionDefinitionDef, "mission_definition", mission_definitions, id);
        $t!(EventTransformation, $crate::raw_defs::EventTransformationDef, "event_transformation", event_transformations, id);
        $t!(MartialArt, $crate::raw_defs::MartialArtDef, "martial_art", martial_arts, id);
        $t!(MonsterAttack, $crate::raw_defs::MonsterAttackDef, "monster_attack", monster_attacks, id);
        $t!(WeakpointSet, $crate::raw_defs::WeakpointSetDef, "weakpoint_set", weakpoint_sets, id);
        $t!(RecipeGroup, $crate::raw_defs::RecipeGroupDef, "recipe_group", recipe_groups, id);
        $t!(MonsterFlag, $crate::raw_defs::MonsterFlagDef, "monster_flag", monster_flags, id);
        $t!(ActivityType, $crate::raw_defs::ActivityTypeDef, "activity_type", activity_types, id);
        $t!(AmmoEffect, $crate::raw_defs::AmmoEffectDef, "ammo_effect", ammo_effects, id);
        $t!(ToolQuality, $crate::raw_defs::ToolQualityDef, "tool_quality", tool_qualities, id);
        $t!(Fault, $crate::raw_defs::FaultDef, "fault", faults, id);
        $t!(MapExtra, $crate::raw_defs::MapExtraDef, "map_extra", map_extras, id);
        $t!(FaultFix, $crate::raw_defs::FaultFixDef, "fault_fix", fault_fixes, id);
        $t!(TerFurnTransform, $crate::raw_defs::TerFurnTransformDef, "ter_furn_transform", ter_furn_transforms, id);
        $t!(ConnectGroup, $crate::raw_defs::ConnectGroupDef, "connect_group", connect_groups, id);
        $t!(AttackVector, $crate::raw_defs::AttackVectorDef, "attack_vector", attack_vectors, id);
        $t!(RegionTerrainFurniture, $crate::raw_defs::RegionTerrainFurnitureDef, "region_terrain_furniture", region_terrain_furnitures, id);
        $t!(ItemCategory, $crate::raw_defs::ItemCategoryDef, "ITEM_CATEGORY", item_categories, id);
        $t!(OterVision, $crate::raw_defs::OterVisionDef, "oter_vision", oter_visions, id);
        $t!(ProfessionItemSubstitutions, $crate::raw_defs::ProfessionItemSubstitutionsDef, "profession_item_substitutions", profession_item_substitutions, item);
        $t!(CharacterMod, $crate::raw_defs::CharacterModDef, "character_mod", character_mods, id);
        $t!(WeaponCategory, $crate::raw_defs::WeaponCategoryDef, "weapon_category", weapon_categories, id);
        $t!(RotatableSymbol, $crate::raw_defs::RotatableSymbolDef, "rotatable_symbol", rotatable_symbols, synthetic);
        $t!(OterIdMigration, $crate::raw_defs::OterIdMigrationDef, "oter_id_migration", oter_id_migrations, custom);
        $t!(ClimbingAid, $crate::raw_defs::ClimbingAidDef, "climbing_aid", climbing_aids, id);
        $t!(Conduct, $crate::raw_defs::ConductDef, "conduct", conducts, id);
        $t!(WeatherType, $crate::raw_defs::WeatherTypeDef, "weather_type", weather_types, id);
        $t!(ProficiencyCategory, $crate::raw_defs::ProficiencyCategoryDef, "proficiency_category", proficiency_categories, id);
        $t!(FactionMission, $crate::raw_defs::FactionMissionDef, "faction_mission", faction_missions, id);
        $t!(FaultGroup, $crate::raw_defs::FaultGroupDef, "fault_group", fault_groups, id);
        $t!(JmathFunction, $crate::raw_defs::JmathFunctionDef, "jmath_function", jmath_functions, id);
        $t!(BodyGraph, $crate::raw_defs::BodyGraphDef, "body_graph", body_graphs, id);
        $t!(LimbScore, $crate::raw_defs::LimbScoreDef, "limb_score", limb_scores, id);
        $t!(ConstructionCategory, $crate::raw_defs::ConstructionCategoryDef, "construction_category", construction_categories, id);
        $t!(RecipeCategory, $crate::raw_defs::RecipeCategoryDef, "recipe_category", recipe_categories, id);
        $t!(AddictionType, $crate::raw_defs::AddictionTypeDef, "addiction_type", addiction_types, id);
        $t!(RegionSettings, $crate::raw_defs::RegionSettingsDef, "region_settings", region_settings, id);
        $t!(Gate, $crate::raw_defs::GateDef, "gate", gates, id);
        $t!(DamageType, $crate::raw_defs::DamageTypeDef, "damage_type", damage_types, id);
        $t!(Anatomy, $crate::raw_defs::AnatomyDef, "anatomy", anatomies, id);
        $t!(EndScreen, $crate::raw_defs::EndScreenDef, "end_screen", end_screens, id);
    };

    // ── list-context mode ──────────────────────────────────────────────
    (list $mapper:ident) => {
        vec![
            $mapper!(Item, $crate::raw_defs::ItemDef, "ITEM", items, id),
            $mapper!(Monster, $crate::raw_defs::MonsterDef, "MONSTER", monsters, id),
            $mapper!(Terrain, $crate::raw_defs::TerrainDef, "terrain", terrain, id),
            $mapper!(Furniture, $crate::raw_defs::FurnitureDef, "furniture", furniture, id),
            $mapper!(Recipe, $crate::raw_defs::RecipeDef, "recipe", recipes, id),
            $mapper!(ItemGroup, $crate::raw_defs::ItemGroupDef, "item_group", item_groups, id),
            $mapper!(MapgenPalette, $crate::raw_defs::MapgenPaletteDef, "palette", palettes, id),
            $mapper!(OvermapTerrain, $crate::raw_defs::OvermapTerrainDef, "overmap_terrain", overmap_terrains, id),
            $mapper!(OvermapSpecial, $crate::raw_defs::OvermapSpecialDef, "overmap_special", overmap_specials, id),
            $mapper!(OvermapConnection, $crate::raw_defs::OvermapConnectionDef, "overmap_connection", overmap_connections, id),
            $mapper!(OvermapLocation, $crate::raw_defs::OvermapLocationDef, "overmap_location", overmap_locations, id),
            $mapper!(OvermapLandUseCode, $crate::raw_defs::OvermapLandUseCodeDef, "overmap_land_use_code", overmap_land_use_codes, id),
            $mapper!(Field, $crate::raw_defs::FieldDef, "field_type", fields, id),
            $mapper!(VehiclePart, $crate::raw_defs::VehiclePartDef, "vehicle_part", vehicle_parts, id),
            $mapper!(VehiclePartLocation, $crate::raw_defs::VehiclePartLocationDef, "vehicle_part_location", vehicle_part_locations, id),
            $mapper!(VehiclePartCategory, $crate::raw_defs::VehiclePartCategoryDef, "vehicle_part_category", vehicle_part_categories, id),
            $mapper!(Mutation, $crate::raw_defs::MutationDef, "mutation", mutations, id),
            $mapper!(MutationCategory, $crate::raw_defs::MutationCategoryDef, "mutation_category", mutation_categories, id),
            $mapper!(TraitGroup, $crate::raw_defs::TraitGroupDef, "trait_group", trait_groups, id),
            $mapper!(Bionic, $crate::raw_defs::BionicDef, "bionic", bionics, id),
            $mapper!(Effect, $crate::raw_defs::EffectDef, "effect_type", effects, id),
            $mapper!(Faction, $crate::raw_defs::FactionDef, "faction", factions, id),
            $mapper!(Scenario, $crate::raw_defs::ScenarioDef, "scenario", scenarios, id),
            $mapper!(Material, $crate::raw_defs::MaterialDef, "material", materials, id),
            $mapper!(Skill, $crate::raw_defs::SkillDef, "skill", skills, id),
            $mapper!(Trap, $crate::raw_defs::TrapDef, "trap", traps, id),
            $mapper!(StartLocation, $crate::raw_defs::StartLocationDef, "start_location", start_locations, id),
            $mapper!(JsonFlag, $crate::raw_defs::JsonFlagDef, "json_flag", json_flags, id),
            $mapper!(AsciiArt, $crate::raw_defs::AsciiArtDef, "ascii_art", ascii_art, id),
            $mapper!(ConstructionGroup, $crate::raw_defs::ConstructionGroupDef, "construction_group", construction_groups, id),
            $mapper!(ItemAction, $crate::raw_defs::ItemActionDef, "item_action", item_actions, id),
            $mapper!(Technique, $crate::raw_defs::TechniqueDef, "technique", techniques, id),
            $mapper!(AmmunitionType, $crate::raw_defs::AmmunitionTypeDef, "ammunition_type", ammunition_types, id),
            $mapper!(MoraleType, $crate::raw_defs::MoraleTypeDef, "morale_type", morale_types, id),
            $mapper!(ScentType, $crate::raw_defs::ScentTypeDef, "scent_type", scent_types, id),
            $mapper!(MovementMode, $crate::raw_defs::MovementModeDef, "movement_mode", movement_modes, id),
            $mapper!(MoodFace, $crate::raw_defs::MoodFaceDef, "mood_face", mood_faces, id),
            $mapper!(Achievement, $crate::raw_defs::AchievementDef, "achievement", achievements, id),
            $mapper!(BodyPart, $crate::raw_defs::BodyPartDef, "body_part", body_parts, id),
            $mapper!(Dream, $crate::raw_defs::DreamDef, "dream", dreams, synthetic),
            $mapper!(Emit, $crate::raw_defs::EmitDef, "emit", emits, id),
            $mapper!(EventStatistic, $crate::raw_defs::EventStatisticDef, "event_statistic", event_statistics, id),
            $mapper!(Harvest, $crate::raw_defs::HarvestDef, "harvest", harvests, id),
            $mapper!(ItemMigration, $crate::raw_defs::ItemMigrationDef, "MIGRATION", item_migrations, id),
            $mapper!(MonsterGroup, $crate::raw_defs::MonsterGroupDef, "monstergroup", monster_groups, id),
            $mapper!(MutationType, $crate::raw_defs::MutationTypeDef, "mutation_type", mutation_types, id),
            $mapper!(NestedCategory, $crate::raw_defs::NestedCategoryDef, "nested_category", nested_categories, id),
            $mapper!(Practice, $crate::raw_defs::PracticeDef, "practice", practices, id),
            $mapper!(Profession, $crate::raw_defs::ProfessionDef, "profession", professions, id),
            $mapper!(Proficiency, $crate::raw_defs::ProficiencyDef, "proficiency", proficiencies, id),
            $mapper!(Score, $crate::raw_defs::ScoreDef, "score", scores, id),
            $mapper!(Species, $crate::raw_defs::SpeciesDef, "SPECIES", species, id),
            $mapper!(SubBodyPart, $crate::raw_defs::SubBodyPartDef, "sub_body_part", sub_body_parts, id),
            $mapper!(Uncraft, $crate::raw_defs::UncraftDef, "uncraft", uncrafts, id),
            $mapper!(Vitamin, $crate::raw_defs::VitaminDef, "vitamin", vitamins, id),
            $mapper!(TalkTopic, $crate::raw_defs::TalkTopicDef, "talk_topic", talk_topics, id),
            $mapper!(Widget, $crate::raw_defs::WidgetDef, "widget", widgets, id),
            $mapper!(EffectOnCondition, $crate::raw_defs::EffectOnConditionDef, "effect_on_condition", effects_on_condition, id),
            $mapper!(Construction, $crate::raw_defs::ConstructionDef, "construction", constructions, id),
            $mapper!(Snippet, $crate::raw_defs::SnippetDef, "snippet", snippets, custom),
            $mapper!(Npc, $crate::raw_defs::NpcDef, "npc", npcs, id),
            $mapper!(NpcClass, $crate::raw_defs::NpcClassDef, "npc_class", npc_classes, id),
            $mapper!(Requirement, $crate::raw_defs::RequirementDef, "requirement", requirements, id),
            $mapper!(Spell, $crate::raw_defs::SpellDef, "SPELL", spells, id),
            $mapper!(Vehicle, $crate::raw_defs::VehicleDef, "vehicle", vehicles, id),
            $mapper!(CityBuilding, $crate::raw_defs::CityBuildingDef, "city_building", city_buildings, id),
            $mapper!(MissionDefinition, $crate::raw_defs::MissionDefinitionDef, "mission_definition", mission_definitions, id),
            $mapper!(EventTransformation, $crate::raw_defs::EventTransformationDef, "event_transformation", event_transformations, id),
            $mapper!(MartialArt, $crate::raw_defs::MartialArtDef, "martial_art", martial_arts, id),
            $mapper!(MonsterAttack, $crate::raw_defs::MonsterAttackDef, "monster_attack", monster_attacks, id),
            $mapper!(WeakpointSet, $crate::raw_defs::WeakpointSetDef, "weakpoint_set", weakpoint_sets, id),
            $mapper!(RecipeGroup, $crate::raw_defs::RecipeGroupDef, "recipe_group", recipe_groups, id),
            $mapper!(MonsterFlag, $crate::raw_defs::MonsterFlagDef, "monster_flag", monster_flags, id),
            $mapper!(ActivityType, $crate::raw_defs::ActivityTypeDef, "activity_type", activity_types, id),
            $mapper!(AmmoEffect, $crate::raw_defs::AmmoEffectDef, "ammo_effect", ammo_effects, id),
            $mapper!(ToolQuality, $crate::raw_defs::ToolQualityDef, "tool_quality", tool_qualities, id),
            $mapper!(Fault, $crate::raw_defs::FaultDef, "fault", faults, id),
            $mapper!(MapExtra, $crate::raw_defs::MapExtraDef, "map_extra", map_extras, id),
            $mapper!(FaultFix, $crate::raw_defs::FaultFixDef, "fault_fix", fault_fixes, id),
            $mapper!(TerFurnTransform, $crate::raw_defs::TerFurnTransformDef, "ter_furn_transform", ter_furn_transforms, id),
            $mapper!(ConnectGroup, $crate::raw_defs::ConnectGroupDef, "connect_group", connect_groups, id),
            $mapper!(AttackVector, $crate::raw_defs::AttackVectorDef, "attack_vector", attack_vectors, id),
            $mapper!(RegionTerrainFurniture, $crate::raw_defs::RegionTerrainFurnitureDef, "region_terrain_furniture", region_terrain_furnitures, id),
            $mapper!(ItemCategory, $crate::raw_defs::ItemCategoryDef, "ITEM_CATEGORY", item_categories, id),
            $mapper!(OterVision, $crate::raw_defs::OterVisionDef, "oter_vision", oter_visions, id),
            $mapper!(ProfessionItemSubstitutions, $crate::raw_defs::ProfessionItemSubstitutionsDef, "profession_item_substitutions", profession_item_substitutions, item),
            $mapper!(CharacterMod, $crate::raw_defs::CharacterModDef, "character_mod", character_mods, id),
            $mapper!(WeaponCategory, $crate::raw_defs::WeaponCategoryDef, "weapon_category", weapon_categories, id),
            $mapper!(RotatableSymbol, $crate::raw_defs::RotatableSymbolDef, "rotatable_symbol", rotatable_symbols, synthetic),
            $mapper!(OterIdMigration, $crate::raw_defs::OterIdMigrationDef, "oter_id_migration", oter_id_migrations, custom),
            $mapper!(ClimbingAid, $crate::raw_defs::ClimbingAidDef, "climbing_aid", climbing_aids, id),
            $mapper!(Conduct, $crate::raw_defs::ConductDef, "conduct", conducts, id),
            $mapper!(WeatherType, $crate::raw_defs::WeatherTypeDef, "weather_type", weather_types, id),
            $mapper!(ProficiencyCategory, $crate::raw_defs::ProficiencyCategoryDef, "proficiency_category", proficiency_categories, id),
            $mapper!(FactionMission, $crate::raw_defs::FactionMissionDef, "faction_mission", faction_missions, id),
            $mapper!(FaultGroup, $crate::raw_defs::FaultGroupDef, "fault_group", fault_groups, id),
            $mapper!(JmathFunction, $crate::raw_defs::JmathFunctionDef, "jmath_function", jmath_functions, id),
            $mapper!(BodyGraph, $crate::raw_defs::BodyGraphDef, "body_graph", body_graphs, id),
            $mapper!(LimbScore, $crate::raw_defs::LimbScoreDef, "limb_score", limb_scores, id),
            $mapper!(ConstructionCategory, $crate::raw_defs::ConstructionCategoryDef, "construction_category", construction_categories, id),
            $mapper!(RecipeCategory, $crate::raw_defs::RecipeCategoryDef, "recipe_category", recipe_categories, id),
            $mapper!(AddictionType, $crate::raw_defs::AddictionTypeDef, "addiction_type", addiction_types, id),
            $mapper!(RegionSettings, $crate::raw_defs::RegionSettingsDef, "region_settings", region_settings, id),
            $mapper!(Gate, $crate::raw_defs::GateDef, "gate", gates, id),
            $mapper!(DamageType, $crate::raw_defs::DamageTypeDef, "damage_type", damage_types, id),
            $mapper!(Anatomy, $crate::raw_defs::AnatomyDef, "anatomy", anatomies, id),
            $mapper!(EndScreen, $crate::raw_defs::EndScreenDef, "end_screen", end_screens, id),
        ]
    };
}
