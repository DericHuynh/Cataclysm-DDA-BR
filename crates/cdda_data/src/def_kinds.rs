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
/// Type paths use `cdda_defs_raw::raw_defs::`.
#[macro_export]
macro_rules! for_each_raw_def_kind {
    // ── item-context mode ──────────────────────────────────────────────
    (call $t:ident) => {
        $t!(Item, cdda_defs_raw::raw_defs::ItemDef, "ITEM", items, id);
                $t!(Monster, cdda_defs_raw::raw_defs::MonsterDef, "MONSTER", monsters, id);
                $t!(Terrain, cdda_defs_raw::raw_defs::TerrainDef, "terrain", terrain, id);
                $t!(Furniture, cdda_defs_raw::raw_defs::FurnitureDef, "furniture", furniture, id);
                $t!(Recipe, cdda_defs_raw::raw_defs::RecipeDef, "recipe", recipes, id);
                $t!(ItemGroup, cdda_defs_raw::raw_defs::ItemGroupDef, "item_group", item_groups, id);
                $t!(MapgenPalette, cdda_defs_raw::raw_defs::MapgenPaletteDef, "palette", palettes, id);
                $t!(OvermapTerrain, cdda_defs_raw::raw_defs::OvermapTerrainDef, "overmap_terrain", overmap_terrains, id);
                $t!(OvermapSpecial, cdda_defs_raw::raw_defs::OvermapSpecialDef, "overmap_special", overmap_specials, id);
                $t!(OvermapConnection, cdda_defs_raw::raw_defs::OvermapConnectionDef, "overmap_connection", overmap_connections, id);
                $t!(OvermapLocation, cdda_defs_raw::raw_defs::OvermapLocationDef, "overmap_location", overmap_locations, id);
                $t!(OvermapLandUseCode, cdda_defs_raw::raw_defs::OvermapLandUseCodeDef, "overmap_land_use_code", overmap_land_use_codes, id);
                $t!(Field, cdda_defs_raw::raw_defs::FieldDef, "field_type", fields, id);
                $t!(VehiclePart, cdda_defs_raw::raw_defs::VehiclePartDef, "vehicle_part", vehicle_parts, id);
                $t!(VehiclePartLocation, cdda_defs_raw::raw_defs::VehiclePartLocationDef, "vehicle_part_location", vehicle_part_locations, id);
                $t!(VehiclePartCategory, cdda_defs_raw::raw_defs::VehiclePartCategoryDef, "vehicle_part_category", vehicle_part_categories, id);
                $t!(Mutation, cdda_defs_raw::raw_defs::MutationDef, "mutation", mutations, id);
                $t!(MutationCategory, cdda_defs_raw::raw_defs::MutationCategoryDef, "mutation_category", mutation_categories, id);
                $t!(TraitGroup, cdda_defs_raw::raw_defs::TraitGroupDef, "trait_group", trait_groups, id);
                $t!(Bionic, cdda_defs_raw::raw_defs::BionicDef, "bionic", bionics, id);
                $t!(Effect, cdda_defs_raw::raw_defs::EffectDef, "effect_type", effects, id);
                $t!(Faction, cdda_defs_raw::raw_defs::FactionDef, "faction", factions, id);
                $t!(Scenario, cdda_defs_raw::raw_defs::ScenarioDef, "scenario", scenarios, id);
                $t!(Material, cdda_defs_raw::raw_defs::MaterialDef, "material", materials, id);
                $t!(Skill, cdda_defs_raw::raw_defs::SkillDef, "skill", skills, id);
                $t!(Trap, cdda_defs_raw::raw_defs::TrapDef, "trap", traps, id);
                $t!(StartLocation, cdda_defs_raw::raw_defs::StartLocationDef, "start_location", start_locations, id);
                $t!(JsonFlag, cdda_defs_raw::raw_defs::JsonFlagDef, "json_flag", json_flags, id);
                $t!(AsciiArt, cdda_defs_raw::raw_defs::AsciiArtDef, "ascii_art", ascii_art, id);
                $t!(ConstructionGroup, cdda_defs_raw::raw_defs::ConstructionGroupDef, "construction_group", construction_groups, id);
                $t!(ItemAction, cdda_defs_raw::raw_defs::ItemActionDef, "item_action", item_actions, id);
                $t!(Technique, cdda_defs_raw::raw_defs::TechniqueDef, "technique", techniques, id);
                $t!(AmmunitionType, cdda_defs_raw::raw_defs::AmmunitionTypeDef, "ammunition_type", ammunition_types, id);
                $t!(MoraleType, cdda_defs_raw::raw_defs::MoraleTypeDef, "morale_type", morale_types, id);
                $t!(ScentType, cdda_defs_raw::raw_defs::ScentTypeDef, "scent_type", scent_types, id);
                $t!(MovementMode, cdda_defs_raw::raw_defs::MovementModeDef, "movement_mode", movement_modes, id);
                $t!(MoodFace, cdda_defs_raw::raw_defs::MoodFaceDef, "mood_face", mood_faces, id);
                $t!(Achievement, cdda_defs_raw::raw_defs::AchievementDef, "achievement", achievements, id);
                $t!(BodyPart, cdda_defs_raw::raw_defs::BodyPartDef, "body_part", body_parts, id);
                $t!(Dream, cdda_defs_raw::raw_defs::DreamDef, "dream", dreams, synthetic);
                $t!(Emit, cdda_defs_raw::raw_defs::EmitDef, "emit", emits, id);
                $t!(EventStatistic, cdda_defs_raw::raw_defs::EventStatisticDef, "event_statistic", event_statistics, id);
                $t!(Harvest, cdda_defs_raw::raw_defs::HarvestDef, "harvest", harvests, id);
                $t!(ItemMigration, cdda_defs_raw::raw_defs::ItemMigrationDef, "MIGRATION", item_migrations, id);
                $t!(MonsterGroup, cdda_defs_raw::raw_defs::MonsterGroupDef, "monstergroup", monster_groups, id);
                $t!(MutationType, cdda_defs_raw::raw_defs::MutationTypeDef, "mutation_type", mutation_types, id);
                $t!(NestedCategory, cdda_defs_raw::raw_defs::NestedCategoryDef, "nested_category", nested_categories, id);
                $t!(Practice, cdda_defs_raw::raw_defs::PracticeDef, "practice", practices, id);
                $t!(Profession, cdda_defs_raw::raw_defs::ProfessionDef, "profession", professions, id);
                $t!(Proficiency, cdda_defs_raw::raw_defs::ProficiencyDef, "proficiency", proficiencies, id);
                $t!(Score, cdda_defs_raw::raw_defs::ScoreDef, "score", scores, id);
                $t!(Species, cdda_defs_raw::raw_defs::SpeciesDef, "SPECIES", species, id);
                $t!(SubBodyPart, cdda_defs_raw::raw_defs::SubBodyPartDef, "sub_body_part", sub_body_parts, id);
                $t!(Uncraft, cdda_defs_raw::raw_defs::UncraftDef, "uncraft", uncrafts, id);
                $t!(Vitamin, cdda_defs_raw::raw_defs::VitaminDef, "vitamin", vitamins, id);
                $t!(TalkTopic, cdda_defs_raw::raw_defs::TalkTopicDef, "talk_topic", talk_topics, id);
                $t!(Widget, cdda_defs_raw::raw_defs::WidgetDef, "widget", widgets, id);
                $t!(EffectOnCondition, cdda_defs_raw::raw_defs::EffectOnConditionDef, "effect_on_condition", effects_on_condition, id);
                $t!(Construction, cdda_defs_raw::raw_defs::ConstructionDef, "construction", constructions, id);
                $t!(Snippet, cdda_defs_raw::raw_defs::SnippetDef, "snippet", snippets, custom);
                $t!(Npc, cdda_defs_raw::raw_defs::NpcDef, "npc", npcs, id);
                $t!(NpcClass, cdda_defs_raw::raw_defs::NpcClassDef, "npc_class", npc_classes, id);
                $t!(Requirement, cdda_defs_raw::raw_defs::RequirementDef, "requirement", requirements, id);
                $t!(Spell, cdda_defs_raw::raw_defs::SpellDef, "SPELL", spells, id);
                $t!(Vehicle, cdda_defs_raw::raw_defs::VehicleDef, "vehicle", vehicles, id);
                $t!(CityBuilding, cdda_defs_raw::raw_defs::CityBuildingDef, "city_building", city_buildings, id);
                $t!(MissionDefinition, cdda_defs_raw::raw_defs::MissionDefinitionDef, "mission_definition", mission_definitions, id);
                $t!(EventTransformation, cdda_defs_raw::raw_defs::EventTransformationDef, "event_transformation", event_transformations, id);
                $t!(MartialArt, cdda_defs_raw::raw_defs::MartialArtDef, "martial_art", martial_arts, id);
                $t!(MonsterAttack, cdda_defs_raw::raw_defs::MonsterAttackDef, "monster_attack", monster_attacks, id);
                $t!(WeakpointSet, cdda_defs_raw::raw_defs::WeakpointSetDef, "weakpoint_set", weakpoint_sets, id);
                $t!(RecipeGroup, cdda_defs_raw::raw_defs::RecipeGroupDef, "recipe_group", recipe_groups, id);
                $t!(MonsterFlag, cdda_defs_raw::raw_defs::MonsterFlagDef, "monster_flag", monster_flags, id);
                $t!(ActivityType, cdda_defs_raw::raw_defs::ActivityTypeDef, "activity_type", activity_types, id);
                $t!(AmmoEffect, cdda_defs_raw::raw_defs::AmmoEffectDef, "ammo_effect", ammo_effects, id);
                $t!(ToolQuality, cdda_defs_raw::raw_defs::ToolQualityDef, "tool_quality", tool_qualities, id);
                $t!(Fault, cdda_defs_raw::raw_defs::FaultDef, "fault", faults, id);
                $t!(MapExtra, cdda_defs_raw::raw_defs::MapExtraDef, "map_extra", map_extras, id);
                $t!(FaultFix, cdda_defs_raw::raw_defs::FaultFixDef, "fault_fix", fault_fixes, id);
                $t!(TerFurnTransform, cdda_defs_raw::raw_defs::TerFurnTransformDef, "ter_furn_transform", ter_furn_transforms, id);
                $t!(ConnectGroup, cdda_defs_raw::raw_defs::ConnectGroupDef, "connect_group", connect_groups, id);
                $t!(AttackVector, cdda_defs_raw::raw_defs::AttackVectorDef, "attack_vector", attack_vectors, id);
                $t!(RegionTerrainFurniture, cdda_defs_raw::raw_defs::RegionTerrainFurnitureDef, "region_terrain_furniture", region_terrain_furnitures, id);
                $t!(ItemCategory, cdda_defs_raw::raw_defs::ItemCategoryDef, "ITEM_CATEGORY", item_categories, id);
                $t!(OterVision, cdda_defs_raw::raw_defs::OterVisionDef, "oter_vision", oter_visions, id);
                $t!(ProfessionItemSubstitutions, cdda_defs_raw::raw_defs::ProfessionItemSubstitutionsDef, "profession_item_substitutions", profession_item_substitutions, item);
                $t!(CharacterMod, cdda_defs_raw::raw_defs::CharacterModDef, "character_mod", character_mods, id);
                $t!(WeaponCategory, cdda_defs_raw::raw_defs::WeaponCategoryDef, "weapon_category", weapon_categories, id);
                $t!(RotatableSymbol, cdda_defs_raw::raw_defs::RotatableSymbolDef, "rotatable_symbol", rotatable_symbols, synthetic);
                $t!(OterIdMigration, cdda_defs_raw::raw_defs::OterIdMigrationDef, "oter_id_migration", oter_id_migrations, custom);
                $t!(ClimbingAid, cdda_defs_raw::raw_defs::ClimbingAidDef, "climbing_aid", climbing_aids, id);
                $t!(Conduct, cdda_defs_raw::raw_defs::ConductDef, "conduct", conducts, id);
                $t!(WeatherType, cdda_defs_raw::raw_defs::WeatherTypeDef, "weather_type", weather_types, id);
                $t!(ProficiencyCategory, cdda_defs_raw::raw_defs::ProficiencyCategoryDef, "proficiency_category", proficiency_categories, id);
                $t!(FactionMission, cdda_defs_raw::raw_defs::FactionMissionDef, "faction_mission", faction_missions, id);
                $t!(FaultGroup, cdda_defs_raw::raw_defs::FaultGroupDef, "fault_group", fault_groups, id);
                $t!(JmathFunction, cdda_defs_raw::raw_defs::JmathFunctionDef, "jmath_function", jmath_functions, id);
                $t!(BodyGraph, cdda_defs_raw::raw_defs::BodyGraphDef, "body_graph", body_graphs, id);
                $t!(LimbScore, cdda_defs_raw::raw_defs::LimbScoreDef, "limb_score", limb_scores, id);
                $t!(ConstructionCategory, cdda_defs_raw::raw_defs::ConstructionCategoryDef, "construction_category", construction_categories, id);
                $t!(RecipeCategory, cdda_defs_raw::raw_defs::RecipeCategoryDef, "recipe_category", recipe_categories, id);
                $t!(AddictionType, cdda_defs_raw::raw_defs::AddictionTypeDef, "addiction_type", addiction_types, id);
                $t!(RegionSettings, cdda_defs_raw::raw_defs::RegionSettingsDef, "region_settings", region_settings, id);
                $t!(Gate, cdda_defs_raw::raw_defs::GateDef, "gate", gates, id);
                $t!(DamageType, cdda_defs_raw::raw_defs::DamageTypeDef, "damage_type", damage_types, id);
                $t!(Anatomy, cdda_defs_raw::raw_defs::AnatomyDef, "anatomy", anatomies, id);
                $t!(EndScreen, cdda_defs_raw::raw_defs::EndScreenDef, "end_screen", end_screens, id);
                $t!(HtnCompound, cdda_defs_raw::raw_defs::HtnCompoundDef, "htn_compound", htn_compounds, id);
            };

            // ── list-context mode ──────────────────────────────────────────────
            (list $mapper:ident) => {
                vec![
                    $mapper!(Item, cdda_defs_raw::raw_defs::ItemDef, "ITEM", items, id),
                    $mapper!(Monster, cdda_defs_raw::raw_defs::MonsterDef, "MONSTER", monsters, id),
                    $mapper!(Terrain, cdda_defs_raw::raw_defs::TerrainDef, "terrain", terrain, id),
                    $mapper!(Furniture, cdda_defs_raw::raw_defs::FurnitureDef, "furniture", furniture, id),
                    $mapper!(Recipe, cdda_defs_raw::raw_defs::RecipeDef, "recipe", recipes, id),
                    $mapper!(ItemGroup, cdda_defs_raw::raw_defs::ItemGroupDef, "item_group", item_groups, id),
                    $mapper!(MapgenPalette, cdda_defs_raw::raw_defs::MapgenPaletteDef, "palette", palettes, id),
                    $mapper!(OvermapTerrain, cdda_defs_raw::raw_defs::OvermapTerrainDef, "overmap_terrain", overmap_terrains, id),
                    $mapper!(OvermapSpecial, cdda_defs_raw::raw_defs::OvermapSpecialDef, "overmap_special", overmap_specials, id),
                    $mapper!(OvermapConnection, cdda_defs_raw::raw_defs::OvermapConnectionDef, "overmap_connection", overmap_connections, id),
                    $mapper!(OvermapLocation, cdda_defs_raw::raw_defs::OvermapLocationDef, "overmap_location", overmap_locations, id),
                    $mapper!(OvermapLandUseCode, cdda_defs_raw::raw_defs::OvermapLandUseCodeDef, "overmap_land_use_code", overmap_land_use_codes, id),
                    $mapper!(Field, cdda_defs_raw::raw_defs::FieldDef, "field_type", fields, id),
                    $mapper!(VehiclePart, cdda_defs_raw::raw_defs::VehiclePartDef, "vehicle_part", vehicle_parts, id),
                    $mapper!(VehiclePartLocation, cdda_defs_raw::raw_defs::VehiclePartLocationDef, "vehicle_part_location", vehicle_part_locations, id),
                    $mapper!(VehiclePartCategory, cdda_defs_raw::raw_defs::VehiclePartCategoryDef, "vehicle_part_category", vehicle_part_categories, id),
                    $mapper!(Mutation, cdda_defs_raw::raw_defs::MutationDef, "mutation", mutations, id),
                    $mapper!(MutationCategory, cdda_defs_raw::raw_defs::MutationCategoryDef, "mutation_category", mutation_categories, id),
                    $mapper!(TraitGroup, cdda_defs_raw::raw_defs::TraitGroupDef, "trait_group", trait_groups, id),
                    $mapper!(Bionic, cdda_defs_raw::raw_defs::BionicDef, "bionic", bionics, id),
                    $mapper!(Effect, cdda_defs_raw::raw_defs::EffectDef, "effect_type", effects, id),
                    $mapper!(Faction, cdda_defs_raw::raw_defs::FactionDef, "faction", factions, id),
                    $mapper!(Scenario, cdda_defs_raw::raw_defs::ScenarioDef, "scenario", scenarios, id),
                    $mapper!(Material, cdda_defs_raw::raw_defs::MaterialDef, "material", materials, id),
                    $mapper!(Skill, cdda_defs_raw::raw_defs::SkillDef, "skill", skills, id),
                    $mapper!(Trap, cdda_defs_raw::raw_defs::TrapDef, "trap", traps, id),
                    $mapper!(StartLocation, cdda_defs_raw::raw_defs::StartLocationDef, "start_location", start_locations, id),
                    $mapper!(JsonFlag, cdda_defs_raw::raw_defs::JsonFlagDef, "json_flag", json_flags, id),
                    $mapper!(AsciiArt, cdda_defs_raw::raw_defs::AsciiArtDef, "ascii_art", ascii_art, id),
                    $mapper!(ConstructionGroup, cdda_defs_raw::raw_defs::ConstructionGroupDef, "construction_group", construction_groups, id),
                    $mapper!(ItemAction, cdda_defs_raw::raw_defs::ItemActionDef, "item_action", item_actions, id),
                    $mapper!(Technique, cdda_defs_raw::raw_defs::TechniqueDef, "technique", techniques, id),
                    $mapper!(AmmunitionType, cdda_defs_raw::raw_defs::AmmunitionTypeDef, "ammunition_type", ammunition_types, id),
                    $mapper!(MoraleType, cdda_defs_raw::raw_defs::MoraleTypeDef, "morale_type", morale_types, id),
                    $mapper!(ScentType, cdda_defs_raw::raw_defs::ScentTypeDef, "scent_type", scent_types, id),
                    $mapper!(MovementMode, cdda_defs_raw::raw_defs::MovementModeDef, "movement_mode", movement_modes, id),
                    $mapper!(MoodFace, cdda_defs_raw::raw_defs::MoodFaceDef, "mood_face", mood_faces, id),
                    $mapper!(Achievement, cdda_defs_raw::raw_defs::AchievementDef, "achievement", achievements, id),
                    $mapper!(BodyPart, cdda_defs_raw::raw_defs::BodyPartDef, "body_part", body_parts, id),
                    $mapper!(Dream, cdda_defs_raw::raw_defs::DreamDef, "dream", dreams, synthetic),
                    $mapper!(Emit, cdda_defs_raw::raw_defs::EmitDef, "emit", emits, id),
                    $mapper!(EventStatistic, cdda_defs_raw::raw_defs::EventStatisticDef, "event_statistic", event_statistics, id),
                    $mapper!(Harvest, cdda_defs_raw::raw_defs::HarvestDef, "harvest", harvests, id),
                    $mapper!(ItemMigration, cdda_defs_raw::raw_defs::ItemMigrationDef, "MIGRATION", item_migrations, id),
                    $mapper!(MonsterGroup, cdda_defs_raw::raw_defs::MonsterGroupDef, "monstergroup", monster_groups, id),
                    $mapper!(MutationType, cdda_defs_raw::raw_defs::MutationTypeDef, "mutation_type", mutation_types, id),
                    $mapper!(NestedCategory, cdda_defs_raw::raw_defs::NestedCategoryDef, "nested_category", nested_categories, id),
                    $mapper!(Practice, cdda_defs_raw::raw_defs::PracticeDef, "practice", practices, id),
                    $mapper!(Profession, cdda_defs_raw::raw_defs::ProfessionDef, "profession", professions, id),
                    $mapper!(Proficiency, cdda_defs_raw::raw_defs::ProficiencyDef, "proficiency", proficiencies, id),
                    $mapper!(Score, cdda_defs_raw::raw_defs::ScoreDef, "score", scores, id),
                    $mapper!(Species, cdda_defs_raw::raw_defs::SpeciesDef, "SPECIES", species, id),
                    $mapper!(SubBodyPart, cdda_defs_raw::raw_defs::SubBodyPartDef, "sub_body_part", sub_body_parts, id),
                    $mapper!(Uncraft, cdda_defs_raw::raw_defs::UncraftDef, "uncraft", uncrafts, id),
                    $mapper!(Vitamin, cdda_defs_raw::raw_defs::VitaminDef, "vitamin", vitamins, id),
                    $mapper!(TalkTopic, cdda_defs_raw::raw_defs::TalkTopicDef, "talk_topic", talk_topics, id),
                    $mapper!(Widget, cdda_defs_raw::raw_defs::WidgetDef, "widget", widgets, id),
                    $mapper!(EffectOnCondition, cdda_defs_raw::raw_defs::EffectOnConditionDef, "effect_on_condition", effects_on_condition, id),
                    $mapper!(Construction, cdda_defs_raw::raw_defs::ConstructionDef, "construction", constructions, id),
                    $mapper!(Snippet, cdda_defs_raw::raw_defs::SnippetDef, "snippet", snippets, custom),
                    $mapper!(Npc, cdda_defs_raw::raw_defs::NpcDef, "npc", npcs, id),
                    $mapper!(NpcClass, cdda_defs_raw::raw_defs::NpcClassDef, "npc_class", npc_classes, id),
                    $mapper!(Requirement, cdda_defs_raw::raw_defs::RequirementDef, "requirement", requirements, id),
                    $mapper!(Spell, cdda_defs_raw::raw_defs::SpellDef, "SPELL", spells, id),
                    $mapper!(Vehicle, cdda_defs_raw::raw_defs::VehicleDef, "vehicle", vehicles, id),
                    $mapper!(CityBuilding, cdda_defs_raw::raw_defs::CityBuildingDef, "city_building", city_buildings, id),
                    $mapper!(MissionDefinition, cdda_defs_raw::raw_defs::MissionDefinitionDef, "mission_definition", mission_definitions, id),
                    $mapper!(EventTransformation, cdda_defs_raw::raw_defs::EventTransformationDef, "event_transformation", event_transformations, id),
                    $mapper!(MartialArt, cdda_defs_raw::raw_defs::MartialArtDef, "martial_art", martial_arts, id),
                    $mapper!(MonsterAttack, cdda_defs_raw::raw_defs::MonsterAttackDef, "monster_attack", monster_attacks, id),
                    $mapper!(WeakpointSet, cdda_defs_raw::raw_defs::WeakpointSetDef, "weakpoint_set", weakpoint_sets, id),
                    $mapper!(RecipeGroup, cdda_defs_raw::raw_defs::RecipeGroupDef, "recipe_group", recipe_groups, id),
                    $mapper!(MonsterFlag, cdda_defs_raw::raw_defs::MonsterFlagDef, "monster_flag", monster_flags, id),
                    $mapper!(ActivityType, cdda_defs_raw::raw_defs::ActivityTypeDef, "activity_type", activity_types, id),
                    $mapper!(AmmoEffect, cdda_defs_raw::raw_defs::AmmoEffectDef, "ammo_effect", ammo_effects, id),
                    $mapper!(ToolQuality, cdda_defs_raw::raw_defs::ToolQualityDef, "tool_quality", tool_qualities, id),
                    $mapper!(Fault, cdda_defs_raw::raw_defs::FaultDef, "fault", faults, id),
                    $mapper!(MapExtra, cdda_defs_raw::raw_defs::MapExtraDef, "map_extra", map_extras, id),
                    $mapper!(FaultFix, cdda_defs_raw::raw_defs::FaultFixDef, "fault_fix", fault_fixes, id),
                    $mapper!(TerFurnTransform, cdda_defs_raw::raw_defs::TerFurnTransformDef, "ter_furn_transform", ter_furn_transforms, id),
                    $mapper!(ConnectGroup, cdda_defs_raw::raw_defs::ConnectGroupDef, "connect_group", connect_groups, id),
                    $mapper!(AttackVector, cdda_defs_raw::raw_defs::AttackVectorDef, "attack_vector", attack_vectors, id),
                    $mapper!(RegionTerrainFurniture, cdda_defs_raw::raw_defs::RegionTerrainFurnitureDef, "region_terrain_furniture", region_terrain_furnitures, id),
                    $mapper!(ItemCategory, cdda_defs_raw::raw_defs::ItemCategoryDef, "ITEM_CATEGORY", item_categories, id),
                    $mapper!(OterVision, cdda_defs_raw::raw_defs::OterVisionDef, "oter_vision", oter_visions, id),
                    $mapper!(ProfessionItemSubstitutions, cdda_defs_raw::raw_defs::ProfessionItemSubstitutionsDef, "profession_item_substitutions", profession_item_substitutions, item),
                    $mapper!(CharacterMod, cdda_defs_raw::raw_defs::CharacterModDef, "character_mod", character_mods, id),
                    $mapper!(WeaponCategory, cdda_defs_raw::raw_defs::WeaponCategoryDef, "weapon_category", weapon_categories, id),
                    $mapper!(RotatableSymbol, cdda_defs_raw::raw_defs::RotatableSymbolDef, "rotatable_symbol", rotatable_symbols, synthetic),
                    $mapper!(OterIdMigration, cdda_defs_raw::raw_defs::OterIdMigrationDef, "oter_id_migration", oter_id_migrations, custom),
                    $mapper!(ClimbingAid, cdda_defs_raw::raw_defs::ClimbingAidDef, "climbing_aid", climbing_aids, id),
                    $mapper!(Conduct, cdda_defs_raw::raw_defs::ConductDef, "conduct", conducts, id),
                    $mapper!(WeatherType, cdda_defs_raw::raw_defs::WeatherTypeDef, "weather_type", weather_types, id),
                    $mapper!(ProficiencyCategory, cdda_defs_raw::raw_defs::ProficiencyCategoryDef, "proficiency_category", proficiency_categories, id),
                    $mapper!(FactionMission, cdda_defs_raw::raw_defs::FactionMissionDef, "faction_mission", faction_missions, id),
                    $mapper!(FaultGroup, cdda_defs_raw::raw_defs::FaultGroupDef, "fault_group", fault_groups, id),
                    $mapper!(JmathFunction, cdda_defs_raw::raw_defs::JmathFunctionDef, "jmath_function", jmath_functions, id),
                    $mapper!(BodyGraph, cdda_defs_raw::raw_defs::BodyGraphDef, "body_graph", body_graphs, id),
                    $mapper!(LimbScore, cdda_defs_raw::raw_defs::LimbScoreDef, "limb_score", limb_scores, id),
                    $mapper!(ConstructionCategory, cdda_defs_raw::raw_defs::ConstructionCategoryDef, "construction_category", construction_categories, id),
                    $mapper!(RecipeCategory, cdda_defs_raw::raw_defs::RecipeCategoryDef, "recipe_category", recipe_categories, id),
                    $mapper!(AddictionType, cdda_defs_raw::raw_defs::AddictionTypeDef, "addiction_type", addiction_types, id),
                    $mapper!(RegionSettings, cdda_defs_raw::raw_defs::RegionSettingsDef, "region_settings", region_settings, id),
                    $mapper!(Gate, cdda_defs_raw::raw_defs::GateDef, "gate", gates, id),
                    $mapper!(DamageType, cdda_defs_raw::raw_defs::DamageTypeDef, "damage_type", damage_types, id),
                    $mapper!(Anatomy, cdda_defs_raw::raw_defs::AnatomyDef, "anatomy", anatomies, id),
                    $mapper!(EndScreen, cdda_defs_raw::raw_defs::EndScreenDef, "end_screen", end_screens, id),
                    $mapper!(HtnCompound, cdda_defs_raw::raw_defs::HtnCompoundDef, "htn_compound", htn_compounds, id),
                ]
    };
}
