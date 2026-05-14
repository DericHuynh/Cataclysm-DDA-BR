//! Generic string-based `DefId<T>` and `DefCategory` enum.

use bevy_reflect::Reflect;
use core::fmt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Every kind of definition known to the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefCategory {
    Item,
    Monster,
    Terrain,
    Furniture,
    Recipe,
    ItemGroup,
    Field,
    Mutation,
    Bionic,
    Effect,
    Faction,
    Skill,
    VehiclePart,
    VehiclePartLocation,
    VehiclePartCategory,
    MapgenPalette,
    OvermapTerrain,
    OvermapSpecial,
    OvermapConnection,
    OvermapLocation,
    OvermapLandUseCode,
    AmmoType,
    BodyPart,
    DamageType,
    Material,
    MutationCategory,
    TraitGroup,
    Species,
    Vitamin,
    Technique,
    SpecialAttack,
    Trap,
    StartLocation,
    Scenario,
    Profession,
    Proficiency,
    Quality,
    // Extended categories
    JsonFlag,
    AsciiArt,
    ConstructionGroup,
    ItemAction,
    MoraleType,
    ScentType,
    MovementMode,
    MoodFace,
    Achievement,
    Dream,
    Emit,
    EventStatistic,
    Harvest,
    ItemMigration,
    MonsterGroup,
    MutationType,
    NestedCategory,
    Practice,
    Score,
    SubBodyPart,
    Uncraft,
    TalkTopic,
    Widget,
    EffectOnCondition,
    Construction,
    Snippet,
    Npc,
    NpcClass,
    Requirement,
    Spell,
    Vehicle,
    CityBuilding,
    MissionDefinition,
    EventTransformation,
    MartialArt,
    MonsterAttack,
    WeakpointSet,
    RecipeGroup,
    MonsterFlag,
    ActivityType,
    AmmoEffect,
    Fault,
    MapExtra,
    FaultFix,
    TerFurnTransform,
    ConnectGroup,
    AttackVector,
    RegionTerrainFurniture,
    ItemCategory,
    OterVision,
    ProfessionItemSubstitutions,
    CharacterMod,
    WeaponCategory,
    RotatableSymbol,
    OterIdMigration,
    ClimbingAid,
    Conduct,
    WeatherType,
    ProficiencyCategory,
    FactionMission,
    FaultGroup,
    JmathFunction,
    BodyGraph,
    LimbScore,
    ConstructionCategory,
    RecipeCategory,
    AddictionType,
    RegionSettings,
    Gate,
    Anatomy,
    EndScreen,
}

// ---------------------------------------------------------------------------
// Generic string-based DefId<T>
// ---------------------------------------------------------------------------

/// A type-safe identifier for game definitions.
#[derive(Debug, Clone, Serialize, Reflect)]
pub struct DefId<T> {
    id: String,
    #[serde(skip)]
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

impl<T> JsonSchema for DefId<T> {
    fn schema_name() -> String {
        "DefId".to_string()
    }
    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <String>::json_schema(_gen)
    }
}

impl<T> PartialEq for DefId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for DefId<T> {}
impl<T> std::hash::Hash for DefId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> Default for DefId<T> {
    fn default() -> Self {
        DefId {
            id: String::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> DefId<T> {
    pub fn new(id: impl Into<String>) -> Self {
        DefId {
            id: id.into(),
            _marker: PhantomData,
        }
    }
    pub fn empty() -> Self {
        DefId {
            id: String::new(),
            _marker: PhantomData,
        }
    }
    pub fn as_str(&self) -> &str {
        &self.id
    }
    pub fn into_string(self) -> String {
        self.id
    }
}

impl<T> fmt::Display for DefId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<'de, T> Deserialize<'de> for DefId<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        Ok(DefId {
            id,
            _marker: PhantomData,
        })
    }
}

impl<T> From<String> for DefId<T> {
    fn from(s: String) -> Self {
        DefId::new(s)
    }
}
impl<T> From<&str> for DefId<T> {
    fn from(s: &str) -> Self {
        DefId::new(s)
    }
}
impl<T> std::ops::Deref for DefId<T> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.id
    }
}
impl<T> PartialEq<str> for DefId<T> {
    fn eq(&self, other: &str) -> bool {
        self.id == other
    }
}
