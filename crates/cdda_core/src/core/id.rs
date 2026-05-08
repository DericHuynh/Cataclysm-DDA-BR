//! Unified identifier types — numeric DefIdx/GenId and generic string DefId<T>.

use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use core::fmt;
use core::fmt::{Debug, Formatter};
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Definition index – dense, never freed
// ---------------------------------------------------------------------------

/// A dense index into a definition registry.
///
/// These are never recycled and are valid for the entire lifetime of the
/// registry.  Comparison is by numeric value only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct DefIdx(pub u32);

impl Debug for DefIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "DefIdx({})", self.0)
    }
}

impl From<u32> for DefIdx {
    #[inline]
    fn from(v: u32) -> Self {
        DefIdx(v)
    }
}

// ---------------------------------------------------------------------------
// Generation-counted handle – for world entities
// ---------------------------------------------------------------------------

/// A generation-counted handle that can detect stale references.
///
/// Useful for entities that may be removed or recycled at runtime (e.g. items
/// lying on the ground).  The generation counter is compared together with the
/// index, so two handles with the same index but different generations are
/// considered different.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenId {
    pub index: u32,
    pub generation: u32,
}

impl Debug for GenId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "GenId({}, gen={})", self.index, self.generation)
    }
}

// ---------------------------------------------------------------------------
// Category enum – for events / ACL / dispatch
// ---------------------------------------------------------------------------

/// Every kind of definition known to the game.
///
/// This is used in event systems, permission checks, and anywhere dispatch
/// over definition categories is needed without a generic type parameter.
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
// Per-category concrete ID types
// ---------------------------------------------------------------------------
// Each wraps a DefIdx so the type system catches cross-category mix-ups.
// All implement From<DefIdx> and From<u32> for ergonomic construction.

macro_rules! def_id_type {
    ($name:ident, $variant:ident) => {
        #[doc = concat!("Concrete ID for a [`", stringify!($name), "`](crate::defs::", stringify!($name), ") definition.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
        pub struct $name(pub DefIdx);

        impl From<DefIdx> for $name {
            #[inline]
            fn from(idx: DefIdx) -> Self {
                $name(idx)
            }
        }

        impl From<u32> for $name {
            #[inline]
            fn from(v: u32) -> Self {
                $name(DefIdx(v))
            }
        }

        impl From<$name> for DefCategory {
            #[inline]
            fn from(_: $name) -> Self {
                DefCategory::$variant
            }
        }
    };
}

def_id_type!(ItemId, Item);
def_id_type!(MonsterId, Monster);
def_id_type!(TerrainId, Terrain);
def_id_type!(FurnitureId, Furniture);
def_id_type!(RecipeId, Recipe);
def_id_type!(ItemGroupId, ItemGroup);
def_id_type!(FieldId, Field);
def_id_type!(MutationId, Mutation);
def_id_type!(MutationCategoryId, MutationCategory);
def_id_type!(TraitGroupId, TraitGroup);
def_id_type!(BionicId, Bionic);
def_id_type!(EffectId, Effect);
def_id_type!(FactionId, Faction);
def_id_type!(SkillId, Skill);
def_id_type!(VehiclePartId, VehiclePart);
def_id_type!(VehiclePartLocationId, VehiclePartLocation);
def_id_type!(VehiclePartCategoryId, VehiclePartCategory);
def_id_type!(MapgenPaletteId, MapgenPalette);
def_id_type!(OvermapTerrainId, OvermapTerrain);
def_id_type!(OvermapSpecialId, OvermapSpecial);
def_id_type!(OvermapConnectionId, OvermapConnection);
def_id_type!(OvermapLocationId, OvermapLocation);
def_id_type!(OvermapLandUseCodeId, OvermapLandUseCode);
def_id_type!(AmmoTypeId, AmmoType);
def_id_type!(BodyPartId, BodyPart);
def_id_type!(DamageTypeId, DamageType);
def_id_type!(MaterialId, Material);
def_id_type!(SpeciesId, Species);
def_id_type!(VitaminId, Vitamin);
def_id_type!(TechniqueId, Technique);
def_id_type!(SpecialAttackId, SpecialAttack);
def_id_type!(TrapId, Trap);
def_id_type!(StartLocationId, StartLocation);
def_id_type!(ScenarioId, Scenario);
def_id_type!(ProfessionId, Profession);
def_id_type!(ProficiencyId, Proficiency);
def_id_type!(QualityId, Quality);

// ---------------------------------------------------------------------------
// Generic string-based DefId<T>
// ---------------------------------------------------------------------------

/// A type-safe identifier for game definitions.
///
/// `DefId<ItemDef>` and `DefId<MonsterDef>` are incompatible at compile time,
/// preventing bugs where the wrong ID type is passed to a lookup.
///
/// Internally stored as a plain String, but the type parameter ensures
/// compile-time safety.
#[derive(Debug, Clone, Serialize)]
pub struct DefId<T> {
    id: String,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

// Manual JsonSchema: DefId serializes/deserializes as a plain string.
impl<T> JsonSchema for DefId<T> {
    fn schema_name() -> String {
        "DefId".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <String>::json_schema(_gen)
    }
}

// Manual PartialEq, Eq, Hash to avoid unnecessary bounds on T
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

impl<T> DefId<T> {
    /// Create a new `DefId` from a string.
    pub fn new(id: impl Into<String>) -> Self {
        DefId {
            id: id.into(),
            _marker: PhantomData,
        }
    }

    /// Get the underlying string ID.
    pub fn as_str(&self) -> &str {
        &self.id
    }

    /// Convert into the underlying string.
    pub fn into_string(self) -> String {
        self.id
    }
}

impl<T> fmt::Display for DefId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

// Custom deserialize because PhantomData doesn't implement Deserialize
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn def_idx_from_u32() {
        let d: DefIdx = 42u32.into();
        assert_eq!(d.0, 42);
    }

    #[test]
    fn id_from_def_idx() {
        let idx = DefIdx(7);
        let item = ItemId::from(idx);
        assert_eq!((item.0).0, 7);
    }

    #[test]
    fn id_from_u32() {
        let monster: MonsterId = 99u32.into();
        assert_eq!((monster.0).0, 99);
    }

    #[test]
    fn gen_id_equality() {
        let a = GenId {
            index: 1,
            generation: 0,
        };
        let b = GenId {
            index: 1,
            generation: 0,
        };
        let c = GenId {
            index: 1,
            generation: 1,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn category_from_id() {
        let id: ItemId = DefIdx(0).into();
        let cat: DefCategory = id.into();
        assert_eq!(cat, DefCategory::Item);
    }

    #[test]
    fn id_types_are_distinct() {
        // Confirm the macro generated distinct types; this should compile.
        let _item = ItemId::from(DefIdx(1));
        let _monster = MonsterId::from(DefIdx(2));
        // Uncommenting the next line would fail to compile because the types
        // are different:
        // let _fail: ItemId = MonsterId(DefIdx(3)).into();
    }
}
