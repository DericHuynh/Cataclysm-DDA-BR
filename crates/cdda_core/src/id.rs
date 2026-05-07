//! Numeric identifier types for the game registry and world entities.
//!
//! * [`DefIdx`] – a dense definition index (never freed), used for registry lookups.
//! * [`GenId`]   – a generation-counted handle for world entities (e.g. items on the map).
//! * Per-category ID types wrap [`DefIdx`] to give each registry slot a concrete type.

use bevy_reflect::Reflect;
use core::fmt;
use core::fmt::{Debug, Formatter};

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
