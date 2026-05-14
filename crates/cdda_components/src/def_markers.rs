//! Phantom marker types for `DefId<T>` type safety.
//! Each marker corresponds to a CDDA definition category.

use crate::core::DefId;
use bevy_reflect::Reflect;

macro_rules! def_marker {
    ($n:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $n;
        impl bevy_reflect::TypePath for $n {
            fn type_path() -> &'static str {
                stringify!($n)
            }
            fn short_type_path() -> &'static str {
                stringify!($n)
            }
            fn module_path() -> Option<&'static str> {
                Some("cdda_components::def_markers")
            }
        }
    };
}

def_marker!(ItemDef);
def_marker!(MonsterDef);
def_marker!(TerrainDef);
def_marker!(FurnitureDef);
def_marker!(RecipeDef);
def_marker!(SkillDef);
def_marker!(SpeciesDef);
def_marker!(ProfessionDef);
def_marker!(ScenarioDef);
def_marker!(FactionDef);
def_marker!(MutationDef);
def_marker!(ProficiencyDef);
def_marker!(BionicDef);
def_marker!(EffectDef);
pub struct BodyPartDefM;
pub struct DamageTypeDefM;
pub struct MaterialDefM;
pub struct AmmoTypeDefM;
pub struct QualityDefM;

/// Backward-compatible type aliases.
pub type SpeciesId = DefId<SpeciesDef>;
pub type ProfessionId = DefId<ProfessionDef>;
pub type ScenarioId = DefId<ScenarioDef>;
pub type FactionId = DefId<FactionDef>;
pub type MutationId = DefId<MutationDef>;
pub type ProficiencyId = DefId<ProficiencyDef>;
pub type BionicId = DefId<BionicDef>;
pub type EffectId = DefId<EffectDef>;
pub type ItemId = DefId<ItemDef>;
