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
/// Type paths are prefixed with `$crate::data::raw_defs::`.
#[macro_export]
macro_rules! for_each_raw_def_kind {
    // ── item-context mode ──────────────────────────────────────────────
    (call $t:ident) => {
        $t!(Item, $crate::data::raw_defs::ItemDef, "ITEM", items, id);
        $t!(Monster, $crate::data::raw_defs::MonsterDef, "MONSTER", monsters, id);
        $t!(Terrain, $crate::data::raw_defs::TerrainDef, "terrain", terrain, id);
        $t!(Furniture, $crate::data::raw_defs::FurnitureDef, "furniture", furniture, id);
        $t!(Recipe, $crate::data::raw_defs::RecipeDef, "recipe", recipes, id);
        $t!(ItemGroup, $crate::data::raw_defs::ItemGroupDef, "item_group", item_groups, id);
        $t!(MapgenPalette, $crate::data::raw_defs::MapgenPaletteDef, "palette", palettes, id);
        $t!(OvermapTerrain, $crate::data::raw_defs::OvermapTerrainDef, "overmap_terrain", overmap_terrains, id);
        $t!(OvermapSpecial, $crate::data::raw_defs::OvermapSpecialDef, "overmap_special", overmap_specials, id);
        $t!(OvermapConnection, $crate::data::raw_defs::OvermapConnectionDef, "overmap_connection", overmap_connections, id);
        $t!(OvermapLocation, $crate::data::raw_defs::OvermapLocationDef, "overmap_location", overmap_locations, id);
        $t!(OvermapLandUseCode, $crate::data::raw_defs::OvermapLandUseCodeDef, "overmap_land_use_code", overmap_land_use_codes, id);
        $t!(Field, $crate::data::raw_defs::FieldDef, "field_type", fields, id);
        $t!(VehiclePart, $crate::data::raw_defs::VehiclePartDef, "vehicle_part", vehicle_parts, id);
        $t!(VehiclePartLocation, $crate::data::raw_defs::VehiclePartLocationDef, "vehicle_part_location", vehicle_part_locations, id);
        $t!(VehiclePartCategory, $crate::data::raw_defs::VehiclePartCategoryDef, "vehicle_part_category", vehicle_part_categories, id);
        $t!(Mutation, $crate::data::raw_defs::MutationDef, "mutation", mutations, id);
        $t!(MutationCategory, $crate::data::raw_defs::MutationCategoryDef, "mutation_category", mutation_categories, id);
        $t!(TraitGroup, $crate::data::raw_defs::TraitGroupDef, "trait_group", trait_groups, id);
        $t!(Bionic, $crate::data::raw_defs::BionicDef, "bionic", bionics, id);
        $t!(Effect, $crate::data::raw_defs::EffectDef, "effect_type", effects, id);
        $t!(Faction, $crate::data::raw_defs::FactionDef, "faction", factions, id);
        $t!(Scenario, $crate::data::raw_defs::ScenarioDef, "scenario", scenarios, id);
        $t!(Material, $crate::data::raw_defs::MaterialDef, "material", materials, id);
        $t!(Skill, $crate::data::raw_defs::SkillDef, "skill", skills, id);
        $t!(Trap, $crate::data::raw_defs::TrapDef, "trap", traps, id);
        $t!(StartLocation, $crate::data::raw_defs::StartLocationDef, "start_location", start_locations, id);
        $t!(JsonFlag, $crate::data::raw_defs::JsonFlagDef, "json_flag", json_flags, id);
        $t!(AsciiArt, $crate::data::raw_defs::AsciiArtDef, "ascii_art", ascii_art, id);
        $t!(ConstructionGroup, $crate::data::raw_defs::ConstructionGroupDef, "construction_group", construction_groups, id);
        $t!(ItemAction, $crate::data::raw_defs::ItemActionDef, "item_action", item_actions, id);
        $t!(Technique, $crate::data::raw_defs::TechniqueDef, "technique", techniques, id);
        $t!(AmmunitionType, $crate::data::raw_defs::AmmunitionTypeDef, "ammunition_type", ammunition_types, id);
        $t!(MoraleType, $crate::data::raw_defs::MoraleTypeDef, "morale_type", morale_types, id);
        $t!(ScentType, $crate::data::raw_defs::ScentTypeDef, "scent_type", scent_types, id);
        $t!(MovementMode, $crate::data::raw_defs::MovementModeDef, "movement_mode", movement_modes, id);
        $t!(MoodFace, $crate::data::raw_defs::MoodFaceDef, "mood_face", mood_faces, id);
        $t!(Achievement, $crate::data::raw_defs::AchievementDef, "achievement", achievements, id);
        $t!(BodyPart, $crate::data::raw_defs::BodyPartDef, "body_part", body_parts, id);
        $t!(Dream, $crate::data::raw_defs::DreamDef, "dream", dreams, synthetic);
        $t!(Emit, $crate::data::raw_defs::EmitDef, "emit", emits, id);
        $t!(EventStatistic, $crate::data::raw_defs::EventStatisticDef, "event_statistic", event_statistics, id);
        $t!(Harvest, $crate::data::raw_defs::HarvestDef, "harvest", harvests, id);
        $t!(ItemMigration, $crate::data::raw_defs::ItemMigrationDef, "MIGRATION", item_migrations, id);
        $t!(MonsterGroup, $crate::data::raw_defs::MonsterGroupDef, "monstergroup", monster_groups, id);
        $t!(MutationType, $crate::data::raw_defs::MutationTypeDef, "mutation_type", mutation_types, id);
        $t!(NestedCategory, $crate::data::raw_defs::NestedCategoryDef, "nested_category", nested_categories, id);
        $t!(Practice, $crate::data::raw_defs::PracticeDef, "practice", practices, id);
        $t!(Profession, $crate::data::raw_defs::ProfessionDef, "profession", professions, id);
        $t!(Proficiency, $crate::data::raw_defs::ProficiencyDef, "proficiency", proficiencies, id);
        $t!(Score, $crate::data::raw_defs::ScoreDef, "score", scores, id);
        $t!(Species, $crate::data::raw_defs::SpeciesDef, "SPECIES", species, id);
        $t!(SubBodyPart, $crate::data::raw_defs::SubBodyPartDef, "sub_body_part", sub_body_parts, id);
        $t!(Uncraft, $crate::data::raw_defs::UncraftDef, "uncraft", uncrafts, id);
        $t!(Vitamin, $crate::data::raw_defs::VitaminDef, "vitamin", vitamins, id);
        $t!(TalkTopic, $crate::data::raw_defs::TalkTopicDef, "talk_topic", talk_topics, id);
        $t!(Widget, $crate::data::raw_defs::WidgetDef, "widget", widgets, id);
        $t!(EffectOnCondition, $crate::data::raw_defs::EffectOnConditionDef, "effect_on_condition", effects_on_condition, id);
        $t!(Construction, $crate::data::raw_defs::ConstructionDef, "construction", constructions, id);
        $t!(Snippet, $crate::data::raw_defs::SnippetDef, "snippet", snippets, custom);
        $t!(Npc, $crate::data::raw_defs::NpcDef, "npc", npcs, id);
        $t!(NpcClass, $crate::data::raw_defs::NpcClassDef, "npc_class", npc_classes, id);
        $t!(Requirement, $crate::data::raw_defs::RequirementDef, "requirement", requirements, id);
        $t!(Spell, $crate::data::raw_defs::SpellDef, "SPELL", spells, id);
        $t!(Vehicle, $crate::data::raw_defs::VehicleDef, "vehicle", vehicles, id);
        $t!(CityBuilding, $crate::data::raw_defs::CityBuildingDef, "city_building", city_buildings, id);
        $t!(MissionDefinition, $crate::data::raw_defs::MissionDefinitionDef, "mission_definition", mission_definitions, id);
        $t!(EventTransformation, $crate::data::raw_defs::EventTransformationDef, "event_transformation", event_transformations, id);
        $t!(MartialArt, $crate::data::raw_defs::MartialArtDef, "martial_art", martial_arts, id);
        $t!(MonsterAttack, $crate::data::raw_defs::MonsterAttackDef, "monster_attack", monster_attacks, id);
        $t!(WeakpointSet, $crate::data::raw_defs::WeakpointSetDef, "weakpoint_set", weakpoint_sets, id);
        $t!(RecipeGroup, $crate::data::raw_defs::RecipeGroupDef, "recipe_group", recipe_groups, id);
        $t!(MonsterFlag, $crate::data::raw_defs::MonsterFlagDef, "monster_flag", monster_flags, id);
        $t!(ActivityType, $crate::data::raw_defs::ActivityTypeDef, "activity_type", activity_types, id);
        $t!(AmmoEffect, $crate::data::raw_defs::AmmoEffectDef, "ammo_effect", ammo_effects, id);
        $t!(ToolQuality, $crate::data::raw_defs::ToolQualityDef, "tool_quality", tool_qualities, id);
        $t!(Fault, $crate::data::raw_defs::FaultDef, "fault", faults, id);
        $t!(MapExtra, $crate::data::raw_defs::MapExtraDef, "map_extra", map_extras, id);
        $t!(FaultFix, $crate::data::raw_defs::FaultFixDef, "fault_fix", fault_fixes, id);
        $t!(TerFurnTransform, $crate::data::raw_defs::TerFurnTransformDef, "ter_furn_transform", ter_furn_transforms, id);
        $t!(ConnectGroup, $crate::data::raw_defs::ConnectGroupDef, "connect_group", connect_groups, id);
        $t!(AttackVector, $crate::data::raw_defs::AttackVectorDef, "attack_vector", attack_vectors, id);
        $t!(RegionTerrainFurniture, $crate::data::raw_defs::RegionTerrainFurnitureDef, "region_terrain_furniture", region_terrain_furnitures, id);
        $t!(ItemCategory, $crate::data::raw_defs::ItemCategoryDef, "ITEM_CATEGORY", item_categories, id);
        $t!(OterVision, $crate::data::raw_defs::OterVisionDef, "oter_vision", oter_visions, id);
        $t!(ProfessionItemSubstitutions, $crate::data::raw_defs::ProfessionItemSubstitutionsDef, "profession_item_substitutions", profession_item_substitutions, item);
        $t!(CharacterMod, $crate::data::raw_defs::CharacterModDef, "character_mod", character_mods, id);
        $t!(WeaponCategory, $crate::data::raw_defs::WeaponCategoryDef, "weapon_category", weapon_categories, id);
        $t!(RotatableSymbol, $crate::data::raw_defs::RotatableSymbolDef, "rotatable_symbol", rotatable_symbols, synthetic);
        $t!(OterIdMigration, $crate::data::raw_defs::OterIdMigrationDef, "oter_id_migration", oter_id_migrations, custom);
        $t!(ClimbingAid, $crate::data::raw_defs::ClimbingAidDef, "climbing_aid", climbing_aids, id);
        $t!(Conduct, $crate::data::raw_defs::ConductDef, "conduct", conducts, id);
        $t!(WeatherType, $crate::data::raw_defs::WeatherTypeDef, "weather_type", weather_types, id);
        $t!(ProficiencyCategory, $crate::data::raw_defs::ProficiencyCategoryDef, "proficiency_category", proficiency_categories, id);
        $t!(FactionMission, $crate::data::raw_defs::FactionMissionDef, "faction_mission", faction_missions, id);
        $t!(FaultGroup, $crate::data::raw_defs::FaultGroupDef, "fault_group", fault_groups, id);
        $t!(JmathFunction, $crate::data::raw_defs::JmathFunctionDef, "jmath_function", jmath_functions, id);
        $t!(BodyGraph, $crate::data::raw_defs::BodyGraphDef, "body_graph", body_graphs, id);
        $t!(LimbScore, $crate::data::raw_defs::LimbScoreDef, "limb_score", limb_scores, id);
        $t!(ConstructionCategory, $crate::data::raw_defs::ConstructionCategoryDef, "construction_category", construction_categories, id);
        $t!(RecipeCategory, $crate::data::raw_defs::RecipeCategoryDef, "recipe_category", recipe_categories, id);
        $t!(AddictionType, $crate::data::raw_defs::AddictionTypeDef, "addiction_type", addiction_types, id);
        $t!(RegionSettings, $crate::data::raw_defs::RegionSettingsDef, "region_settings", region_settings, id);
        $t!(Gate, $crate::data::raw_defs::GateDef, "gate", gates, id);
        $t!(DamageType, $crate::data::raw_defs::DamageTypeDef, "damage_type", damage_types, id);
        $t!(Anatomy, $crate::data::raw_defs::AnatomyDef, "anatomy", anatomies, id);
        $t!(EndScreen, $crate::data::raw_defs::EndScreenDef, "end_screen", end_screens, id);
    };

    // ── list-context mode ──────────────────────────────────────────────
    (list $mapper:ident) => {
        vec![
            $mapper!(Item, $crate::data::raw_defs::ItemDef, "ITEM", items, id),
            $mapper!(Monster, $crate::data::raw_defs::MonsterDef, "MONSTER", monsters, id),
            $mapper!(Terrain, $crate::data::raw_defs::TerrainDef, "terrain", terrain, id),
            $mapper!(Furniture, $crate::data::raw_defs::FurnitureDef, "furniture", furniture, id),
            $mapper!(Recipe, $crate::data::raw_defs::RecipeDef, "recipe", recipes, id),
            $mapper!(ItemGroup, $crate::data::raw_defs::ItemGroupDef, "item_group", item_groups, id),
            $mapper!(MapgenPalette, $crate::data::raw_defs::MapgenPaletteDef, "palette", palettes, id),
            $mapper!(OvermapTerrain, $crate::data::raw_defs::OvermapTerrainDef, "overmap_terrain", overmap_terrains, id),
            $mapper!(OvermapSpecial, $crate::data::raw_defs::OvermapSpecialDef, "overmap_special", overmap_specials, id),
            $mapper!(OvermapConnection, $crate::data::raw_defs::OvermapConnectionDef, "overmap_connection", overmap_connections, id),
            $mapper!(OvermapLocation, $crate::data::raw_defs::OvermapLocationDef, "overmap_location", overmap_locations, id),
            $mapper!(OvermapLandUseCode, $crate::data::raw_defs::OvermapLandUseCodeDef, "overmap_land_use_code", overmap_land_use_codes, id),
            $mapper!(Field, $crate::data::raw_defs::FieldDef, "field_type", fields, id),
            $mapper!(VehiclePart, $crate::data::raw_defs::VehiclePartDef, "vehicle_part", vehicle_parts, id),
            $mapper!(VehiclePartLocation, $crate::data::raw_defs::VehiclePartLocationDef, "vehicle_part_location", vehicle_part_locations, id),
            $mapper!(VehiclePartCategory, $crate::data::raw_defs::VehiclePartCategoryDef, "vehicle_part_category", vehicle_part_categories, id),
            $mapper!(Mutation, $crate::data::raw_defs::MutationDef, "mutation", mutations, id),
            $mapper!(MutationCategory, $crate::data::raw_defs::MutationCategoryDef, "mutation_category", mutation_categories, id),
            $mapper!(TraitGroup, $crate::data::raw_defs::TraitGroupDef, "trait_group", trait_groups, id),
            $mapper!(Bionic, $crate::data::raw_defs::BionicDef, "bionic", bionics, id),
            $mapper!(Effect, $crate::data::raw_defs::EffectDef, "effect_type", effects, id),
            $mapper!(Faction, $crate::data::raw_defs::FactionDef, "faction", factions, id),
            $mapper!(Scenario, $crate::data::raw_defs::ScenarioDef, "scenario", scenarios, id),
            $mapper!(Material, $crate::data::raw_defs::MaterialDef, "material", materials, id),
            $mapper!(Skill, $crate::data::raw_defs::SkillDef, "skill", skills, id),
            $mapper!(Trap, $crate::data::raw_defs::TrapDef, "trap", traps, id),
            $mapper!(StartLocation, $crate::data::raw_defs::StartLocationDef, "start_location", start_locations, id),
            $mapper!(JsonFlag, $crate::data::raw_defs::JsonFlagDef, "json_flag", json_flags, id),
            $mapper!(AsciiArt, $crate::data::raw_defs::AsciiArtDef, "ascii_art", ascii_art, id),
            $mapper!(ConstructionGroup, $crate::data::raw_defs::ConstructionGroupDef, "construction_group", construction_groups, id),
            $mapper!(ItemAction, $crate::data::raw_defs::ItemActionDef, "item_action", item_actions, id),
            $mapper!(Technique, $crate::data::raw_defs::TechniqueDef, "technique", techniques, id),
            $mapper!(AmmunitionType, $crate::data::raw_defs::AmmunitionTypeDef, "ammunition_type", ammunition_types, id),
            $mapper!(MoraleType, $crate::data::raw_defs::MoraleTypeDef, "morale_type", morale_types, id),
            $mapper!(ScentType, $crate::data::raw_defs::ScentTypeDef, "scent_type", scent_types, id),
            $mapper!(MovementMode, $crate::data::raw_defs::MovementModeDef, "movement_mode", movement_modes, id),
            $mapper!(MoodFace, $crate::data::raw_defs::MoodFaceDef, "mood_face", mood_faces, id),
            $mapper!(Achievement, $crate::data::raw_defs::AchievementDef, "achievement", achievements, id),
            $mapper!(BodyPart, $crate::data::raw_defs::BodyPartDef, "body_part", body_parts, id),
            $mapper!(Dream, $crate::data::raw_defs::DreamDef, "dream", dreams, synthetic),
            $mapper!(Emit, $crate::data::raw_defs::EmitDef, "emit", emits, id),
            $mapper!(EventStatistic, $crate::data::raw_defs::EventStatisticDef, "event_statistic", event_statistics, id),
            $mapper!(Harvest, $crate::data::raw_defs::HarvestDef, "harvest", harvests, id),
            $mapper!(ItemMigration, $crate::data::raw_defs::ItemMigrationDef, "MIGRATION", item_migrations, id),
            $mapper!(MonsterGroup, $crate::data::raw_defs::MonsterGroupDef, "monstergroup", monster_groups, id),
            $mapper!(MutationType, $crate::data::raw_defs::MutationTypeDef, "mutation_type", mutation_types, id),
            $mapper!(NestedCategory, $crate::data::raw_defs::NestedCategoryDef, "nested_category", nested_categories, id),
            $mapper!(Practice, $crate::data::raw_defs::PracticeDef, "practice", practices, id),
            $mapper!(Profession, $crate::data::raw_defs::ProfessionDef, "profession", professions, id),
            $mapper!(Proficiency, $crate::data::raw_defs::ProficiencyDef, "proficiency", proficiencies, id),
            $mapper!(Score, $crate::data::raw_defs::ScoreDef, "score", scores, id),
            $mapper!(Species, $crate::data::raw_defs::SpeciesDef, "SPECIES", species, id),
            $mapper!(SubBodyPart, $crate::data::raw_defs::SubBodyPartDef, "sub_body_part", sub_body_parts, id),
            $mapper!(Uncraft, $crate::data::raw_defs::UncraftDef, "uncraft", uncrafts, id),
            $mapper!(Vitamin, $crate::data::raw_defs::VitaminDef, "vitamin", vitamins, id),
            $mapper!(TalkTopic, $crate::data::raw_defs::TalkTopicDef, "talk_topic", talk_topics, id),
            $mapper!(Widget, $crate::data::raw_defs::WidgetDef, "widget", widgets, id),
            $mapper!(EffectOnCondition, $crate::data::raw_defs::EffectOnConditionDef, "effect_on_condition", effects_on_condition, id),
            $mapper!(Construction, $crate::data::raw_defs::ConstructionDef, "construction", constructions, id),
            $mapper!(Snippet, $crate::data::raw_defs::SnippetDef, "snippet", snippets, custom),
            $mapper!(Npc, $crate::data::raw_defs::NpcDef, "npc", npcs, id),
            $mapper!(NpcClass, $crate::data::raw_defs::NpcClassDef, "npc_class", npc_classes, id),
            $mapper!(Requirement, $crate::data::raw_defs::RequirementDef, "requirement", requirements, id),
            $mapper!(Spell, $crate::data::raw_defs::SpellDef, "SPELL", spells, id),
            $mapper!(Vehicle, $crate::data::raw_defs::VehicleDef, "vehicle", vehicles, id),
            $mapper!(CityBuilding, $crate::data::raw_defs::CityBuildingDef, "city_building", city_buildings, id),
            $mapper!(MissionDefinition, $crate::data::raw_defs::MissionDefinitionDef, "mission_definition", mission_definitions, id),
            $mapper!(EventTransformation, $crate::data::raw_defs::EventTransformationDef, "event_transformation", event_transformations, id),
            $mapper!(MartialArt, $crate::data::raw_defs::MartialArtDef, "martial_art", martial_arts, id),
            $mapper!(MonsterAttack, $crate::data::raw_defs::MonsterAttackDef, "monster_attack", monster_attacks, id),
            $mapper!(WeakpointSet, $crate::data::raw_defs::WeakpointSetDef, "weakpoint_set", weakpoint_sets, id),
            $mapper!(RecipeGroup, $crate::data::raw_defs::RecipeGroupDef, "recipe_group", recipe_groups, id),
            $mapper!(MonsterFlag, $crate::data::raw_defs::MonsterFlagDef, "monster_flag", monster_flags, id),
            $mapper!(ActivityType, $crate::data::raw_defs::ActivityTypeDef, "activity_type", activity_types, id),
            $mapper!(AmmoEffect, $crate::data::raw_defs::AmmoEffectDef, "ammo_effect", ammo_effects, id),
            $mapper!(ToolQuality, $crate::data::raw_defs::ToolQualityDef, "tool_quality", tool_qualities, id),
            $mapper!(Fault, $crate::data::raw_defs::FaultDef, "fault", faults, id),
            $mapper!(MapExtra, $crate::data::raw_defs::MapExtraDef, "map_extra", map_extras, id),
            $mapper!(FaultFix, $crate::data::raw_defs::FaultFixDef, "fault_fix", fault_fixes, id),
            $mapper!(TerFurnTransform, $crate::data::raw_defs::TerFurnTransformDef, "ter_furn_transform", ter_furn_transforms, id),
            $mapper!(ConnectGroup, $crate::data::raw_defs::ConnectGroupDef, "connect_group", connect_groups, id),
            $mapper!(AttackVector, $crate::data::raw_defs::AttackVectorDef, "attack_vector", attack_vectors, id),
            $mapper!(RegionTerrainFurniture, $crate::data::raw_defs::RegionTerrainFurnitureDef, "region_terrain_furniture", region_terrain_furnitures, id),
            $mapper!(ItemCategory, $crate::data::raw_defs::ItemCategoryDef, "ITEM_CATEGORY", item_categories, id),
            $mapper!(OterVision, $crate::data::raw_defs::OterVisionDef, "oter_vision", oter_visions, id),
            $mapper!(ProfessionItemSubstitutions, $crate::data::raw_defs::ProfessionItemSubstitutionsDef, "profession_item_substitutions", profession_item_substitutions, item),
            $mapper!(CharacterMod, $crate::data::raw_defs::CharacterModDef, "character_mod", character_mods, id),
            $mapper!(WeaponCategory, $crate::data::raw_defs::WeaponCategoryDef, "weapon_category", weapon_categories, id),
            $mapper!(RotatableSymbol, $crate::data::raw_defs::RotatableSymbolDef, "rotatable_symbol", rotatable_symbols, synthetic),
            $mapper!(OterIdMigration, $crate::data::raw_defs::OterIdMigrationDef, "oter_id_migration", oter_id_migrations, custom),
            $mapper!(ClimbingAid, $crate::data::raw_defs::ClimbingAidDef, "climbing_aid", climbing_aids, id),
            $mapper!(Conduct, $crate::data::raw_defs::ConductDef, "conduct", conducts, id),
            $mapper!(WeatherType, $crate::data::raw_defs::WeatherTypeDef, "weather_type", weather_types, id),
            $mapper!(ProficiencyCategory, $crate::data::raw_defs::ProficiencyCategoryDef, "proficiency_category", proficiency_categories, id),
            $mapper!(FactionMission, $crate::data::raw_defs::FactionMissionDef, "faction_mission", faction_missions, id),
            $mapper!(FaultGroup, $crate::data::raw_defs::FaultGroupDef, "fault_group", fault_groups, id),
            $mapper!(JmathFunction, $crate::data::raw_defs::JmathFunctionDef, "jmath_function", jmath_functions, id),
            $mapper!(BodyGraph, $crate::data::raw_defs::BodyGraphDef, "body_graph", body_graphs, id),
            $mapper!(LimbScore, $crate::data::raw_defs::LimbScoreDef, "limb_score", limb_scores, id),
            $mapper!(ConstructionCategory, $crate::data::raw_defs::ConstructionCategoryDef, "construction_category", construction_categories, id),
            $mapper!(RecipeCategory, $crate::data::raw_defs::RecipeCategoryDef, "recipe_category", recipe_categories, id),
            $mapper!(AddictionType, $crate::data::raw_defs::AddictionTypeDef, "addiction_type", addiction_types, id),
            $mapper!(RegionSettings, $crate::data::raw_defs::RegionSettingsDef, "region_settings", region_settings, id),
            $mapper!(Gate, $crate::data::raw_defs::GateDef, "gate", gates, id),
            $mapper!(DamageType, $crate::data::raw_defs::DamageTypeDef, "damage_type", damage_types, id),
            $mapper!(Anatomy, $crate::data::raw_defs::AnatomyDef, "anatomy", anatomies, id),
            $mapper!(EndScreen, $crate::data::raw_defs::EndScreenDef, "end_screen", end_screens, id),
        ]
    };
}
