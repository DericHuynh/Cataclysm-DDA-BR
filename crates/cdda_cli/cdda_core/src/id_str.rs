//! # Per-type string identifier types
//!
//! Each definition category gets its own newtype wrapper around `String`.
//! Unlike `DefId<T>` in `cdda_data` (which is a single generic), these are
//! explicit named types — catch cross-category mixups at compile time
//! without relying on a phantom type parameter.
//!
//! These are used during JSON deserialization and copy-from resolution where
//! we only have string IDs available. After loading, they are resolved to
//! dense numeric IDs (the `DefIdx`-based types in `id.rs`).

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! str_id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                $name(id.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                $name(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                $name(s.to_string())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

str_id_type!(ItemStrId, "String identifier for an item definition.");
str_id_type!(MonsterStrId, "String identifier for a monster definition.");
str_id_type!(TerrainStrId, "String identifier for a terrain definition.");
str_id_type!(
    FurnitureStrId,
    "String identifier for a furniture definition."
);
str_id_type!(RecipeStrId, "String identifier for a recipe definition.");
str_id_type!(ItemGroupStrId, "String identifier for an item group.");
str_id_type!(FieldStrId, "String identifier for a field type.");
str_id_type!(MutationStrId, "String identifier for a mutation.");
str_id_type!(BionicStrId, "String identifier for a bionic.");
str_id_type!(EffectStrId, "String identifier for an effect type.");
str_id_type!(FactionStrId, "String identifier for a faction.");
str_id_type!(SkillStrId, "String identifier for a skill.");
str_id_type!(MaterialStrId, "String identifier for a material.");
str_id_type!(VehiclePartStrId, "String identifier for a vehicle part.");
str_id_type!(
    MapgenPaletteStrId,
    "String identifier for a mapgen palette."
);
str_id_type!(
    OvermapTerrainStrId,
    "String identifier for an overmap terrain."
);
str_id_type!(
    OvermapSpecialStrId,
    "String identifier for an overmap special."
);
str_id_type!(
    OvermapConnectionStrId,
    "String identifier for an overmap connection."
);
str_id_type!(
    OvermapLocationStrId,
    "String identifier for an overmap location."
);
str_id_type!(
    OvermapLandUseCodeStrId,
    "String identifier for an overmap land use code."
);
str_id_type!(ScenarioStrId, "String identifier for a scenario.");
str_id_type!(ProfessionStrId, "String identifier for a profession.");
str_id_type!(ProficiencyStrId, "String identifier for a proficiency.");
str_id_type!(
    StartLocationStrId,
    "String identifier for a start location."
);
str_id_type!(TrapStrId, "String identifier for a trap.");
str_id_type!(
    AmmunitionTypeStrId,
    "String identifier for an ammunition type."
);
str_id_type!(BodyPartStrId, "String identifier for a body part.");
str_id_type!(DamageTypeStrId, "String identifier for a damage type.");
str_id_type!(
    MutationCategoryStrId,
    "String identifier for a mutation category."
);
str_id_type!(SpeciesStrId, "String identifier for a species.");
str_id_type!(VitaminStrId, "String identifier for a vitamin.");
str_id_type!(TechniqueStrId, "String identifier for a combat technique.");
str_id_type!(
    SpecialAttackStrId,
    "String identifier for a monster special attack."
);
str_id_type!(
    ConstructionStrId,
    "String identifier for a construction recipe."
);
str_id_type!(MapgenStrId, "String identifier for a mapgen definition.");
str_id_type!(AchievementStrId, "String identifier for an achievement.");
str_id_type!(ActivityTypeStrId, "String identifier for an activity type.");
str_id_type!(
    AddictionTypeStrId,
    "String identifier for an addiction type."
);
str_id_type!(AmmoEffectStrId, "String identifier for an ammo effect.");
str_id_type!(AnatomyStrId, "String identifier for an anatomy definition.");
str_id_type!(AsciiArtStrId, "String identifier for an ASCII art.");
str_id_type!(AttackVectorStrId, "String identifier for an attack vector.");
str_id_type!(
    BashDamageProfileStrId,
    "String identifier for a bash damage profile."
);
str_id_type!(BodyGraphStrId, "String identifier for a body graph.");
str_id_type!(
    ButcheryRequirementStrId,
    "String identifier for a butchery requirement."
);
str_id_type!(
    CampMigrationStrId,
    "String identifier for a camp migration."
);
str_id_type!(
    CharacterModStrId,
    "String identifier for a character modifier."
);
str_id_type!(
    ChargeRemovalBlacklistStrId,
    "String identifier for a charge removal blacklist."
);
str_id_type!(CityBuildingStrId, "String identifier for a city building.");
str_id_type!(ClothingModStrId, "String identifier for a clothing mod.");
str_id_type!(ConductStrId, "String identifier for a conduct.");
str_id_type!(ConnectGroupStrId, "String identifier for a connect group.");
str_id_type!(
    ConstructionCategoryStrId,
    "String identifier for a construction category."
);
str_id_type!(
    ConstructionGroupStrId,
    "String identifier for a construction group."
);
str_id_type!(
    DamageInfoOrderStrId,
    "String identifier for damage info order."
);
str_id_type!(DiseaseTypeStrId, "String identifier for a disease type.");
str_id_type!(DreamStrId, "String identifier for a dream.");
str_id_type!(
    EffectOnConditionStrId,
    "String identifier for an effect on condition."
);
str_id_type!(EmitStrId, "String identifier for an emit.");
str_id_type!(EndScreenStrId, "String identifier for an end screen.");
str_id_type!(
    EventStatisticStrId,
    "String identifier for an event statistic."
);
str_id_type!(
    EventTransformationStrId,
    "String identifier for an event transformation."
);
str_id_type!(
    FactionMissionStrId,
    "String identifier for a faction mission."
);
str_id_type!(FaultStrId, "String identifier for a fault.");
str_id_type!(FaultFixStrId, "String identifier for a fault fix.");
str_id_type!(FaultGroupStrId, "String identifier for a fault group.");
str_id_type!(
    ForestBiomeComponentStrId,
    "String identifier for a forest biome component."
);
str_id_type!(GateStrId, "String identifier for a gate.");
str_id_type!(HarvestStrId, "String identifier for a harvest definition.");
str_id_type!(
    HarvestDropTypeStrId,
    "String identifier for a harvest drop type."
);
str_id_type!(HitRangeStrId, "String identifier for a hit range.");
str_id_type!(ItemActionStrId, "String identifier for an item action.");
str_id_type!(ItemCategoryStrId, "String identifier for an item category.");
str_id_type!(
    ItemMigrationStrId,
    "String identifier for an item migration."
);
str_id_type!(JsonFlagStrId, "String identifier for a JSON flag.");
str_id_type!(LimbScoreStrId, "String identifier for a limb score.");
str_id_type!(MapExtraStrId, "String identifier for a map extra.");
str_id_type!(MartialArtStrId, "String identifier for a martial art.");
str_id_type!(
    MissionDefinitionStrId,
    "String identifier for a mission definition."
);
str_id_type!(
    MonsterAttackStrId,
    "String identifier for a monster attack."
);
str_id_type!(
    MonsterBlacklistStrId,
    "String identifier for a monster blacklist."
);
str_id_type!(
    MonsterFactionStrId,
    "String identifier for a monster faction."
);
str_id_type!(MonsterFlagStrId, "String identifier for a monster flag.");
str_id_type!(MonsterGroupStrId, "String identifier for a monster group.");
str_id_type!(MoodFaceStrId, "String identifier for a mood face.");
str_id_type!(MoraleTypeStrId, "String identifier for a morale type.");
str_id_type!(MovementModeStrId, "String identifier for a movement mode.");
str_id_type!(MutationTypeStrId, "String identifier for a mutation type.");
str_id_type!(
    NestedCategoryStrId,
    "String identifier for a nested category."
);
str_id_type!(NpcClassStrId, "String identifier for an NPC class.");
str_id_type!(NpcStrId, "String identifier for an NPC definition.");
str_id_type!(
    OmtPlaceholderStrId,
    "String identifier for an OMT placeholder."
);
str_id_type!(
    OterIdMigrationStrId,
    "String identifier for an OMT ID migration."
);
str_id_type!(
    OterVisionStrId,
    "String identifier for an overmap terrain vision."
);
str_id_type!(OverlayOrderStrId, "String identifier for an overlay order.");
str_id_type!(
    PracticeStrId,
    "String identifier for a practice definition."
);
str_id_type!(
    ProfessionGroupStrId,
    "String identifier for a profession group."
);
str_id_type!(
    ProfessionItemSubstitutionsStrId,
    "String identifier for profession item substitutions."
);
str_id_type!(
    ProficiencyCategoryStrId,
    "String identifier for a proficiency category."
);
str_id_type!(
    ProficiencyMigrationStrId,
    "String identifier for a proficiency migration."
);
str_id_type!(QualityStrId, "String identifier for a tool quality.");
str_id_type!(
    RecipeCategoryStrId,
    "String identifier for a recipe category."
);
str_id_type!(RecipeGroupStrId, "String identifier for a recipe group.");
str_id_type!(
    RegionSettingStrId,
    "String identifier for a region setting."
);
str_id_type!(
    RegionTerrainFurnitureStrId,
    "String identifier for region terrain/furniture."
);
str_id_type!(
    RelicProcgenDataStrId,
    "String identifier for relic procedural generation data."
);
str_id_type!(RequirementStrId, "String identifier for a requirement.");
str_id_type!(
    RotatableSymbolStrId,
    "String identifier for a rotatable symbol."
);
str_id_type!(
    ScenarioBlacklistStrId,
    "String identifier for a scenario blacklist."
);
str_id_type!(ScentTypeStrId, "String identifier for a scent type.");
str_id_type!(ScoreStrId, "String identifier for a score.");
str_id_type!(
    ShopkeeperBlacklistStrId,
    "String identifier for a shopkeeper blacklist."
);
str_id_type!(
    ShopkeeperConsumptionRatesStrId,
    "String identifier for shopkeeper consumption rates."
);
str_id_type!(
    SkillDisplayTypeStrId,
    "String identifier for a skill display type."
);
str_id_type!(SnippetStrId, "String identifier for a snippet.");
str_id_type!(SpeechStrId, "String identifier for a speech entry.");
str_id_type!(
    SpeedDescriptionStrId,
    "String identifier for a speed description."
);
str_id_type!(SpellStrId, "String identifier for a spell.");
str_id_type!(SubBodyPartStrId, "String identifier for a sub-body part.");
str_id_type!(TalkTopicStrId, "String identifier for a talk topic.");
str_id_type!(
    TemperatureRemovalBlacklistStrId,
    "String identifier for a temperature removal blacklist."
);
str_id_type!(
    TerFurnMigrationStrId,
    "String identifier for a terrain/furniture migration."
);
str_id_type!(
    TerFurnTransformStrId,
    "String identifier for a terrain/furniture transform."
);
str_id_type!(TraitGroupStrId, "String identifier for a trait group.");
str_id_type!(
    TraitMigrationStrId,
    "String identifier for a trait migration."
);
str_id_type!(
    TrapMigrationStrId,
    "String identifier for a trap migration."
);
str_id_type!(UncraftStrId, "String identifier for an uncraft recipe.");
str_id_type!(
    VarMigrationStrId,
    "String identifier for a variable migration."
);
str_id_type!(
    VehicleDefStrId,
    "String identifier for a vehicle definition."
);
str_id_type!(
    VehiclePartCategoryStrId,
    "String identifier for a vehicle part category."
);
str_id_type!(
    VehiclePartLocationStrId,
    "String identifier for a vehicle part location."
);
str_id_type!(
    VehiclePartMigrationStrId,
    "String identifier for a vehicle part migration."
);
str_id_type!(
    VehiclePlacementStrId,
    "String identifier for a vehicle placement."
);
str_id_type!(VehicleSpawnStrId, "String identifier for a vehicle spawn.");
str_id_type!(WeakpointSetStrId, "String identifier for a weakpoint set.");
str_id_type!(
    WeaponCategoryStrId,
    "String identifier for a weapon category."
);
str_id_type!(WeatherTypeStrId, "String identifier for a weather type.");
str_id_type!(
    WeatherGeneratorStrId,
    "String identifier for a weather generator."
);
str_id_type!(WidgetStrId, "String identifier for a widget.");
str_id_type!(
    ZoneFieldTypeStrId,
    "String identifier for a zone field type."
);
