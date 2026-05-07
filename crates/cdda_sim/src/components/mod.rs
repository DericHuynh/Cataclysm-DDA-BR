//! ECS components for the simulation layer.
//!
//! Most components live in `cdda_actor` or `cdda_item` and are re-exported
//! here so existing code (`crate::components::*`) continues to compile.
//!
//! This module retains only spatial (`WorldPosition`, `Solid`, `Velocity`)
//! and projectile (`InFlight`) components locally.

// ── Re-export from domain crates ──────────────────────────────────────────

pub use cdda_actor::components::{
    ActiveEffects, Bionic, BionicOf, Bleeding, BodyPartBroken, BodyPartDef, BodyPartHp, BodyPartOf,
    BodyPartSevered, BodyPartSlot, BodyTemperature, CombatStats, Creature, CreatureBodyParts,
    DamageReduction, EffectOn, Faction, Gender, Health, InstalledBionics, IsAlive, Morale,
    MoraleBonus, MoraleBonusOf, MoraleBonuses, MovePoints, MutationState, Mutations, NpcData,
    NpcPersonality, OnFire, PlayerData, ProficiencySet, SkillLevel, SkillSet, Speed, Stats,
    StatusEffect, Stunned, Vision, Wetness,
};

pub use cdda_item::components::{
    AttachmentSlot, AttachmentType, Container, ContainerContents, CurrentCharges, DefOrigin,
    Fireproof, GasTight, InsideContainer, ItemDamage, LoadedAmmo, MountedOn, MountedPockets,
    Pocket, PocketRestriction, PocketType, PreservesTemp, Rigid, Sealed, Spoilable, StackCount,
    Watertight, WieldedBy, WieldedItems, WornBy, WornOn,
};

// ── Locally owned (spatial + projectile) ───────────────────────────────────

use bevy_ecs::component::Component;
use cdda_core::coords::WorldPos;

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WorldPosition(pub WorldPos);

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Solid;

#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub dx: i32,
    pub dy: i32,
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InFlight;
