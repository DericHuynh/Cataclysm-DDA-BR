//! ECS components for creatures, player, NPCs, stats, bionics, effects.
//!
//! Extracted from `crate::sim::components` to its own crate so that
//! `cdda_sim` can depend on `cdda_actor` but not vice versa.
//!
//! ## Design rules
//!
//! * **Small, focused components** — one job each, composed together.
//! * **Relationships** use `#[relationship]` / `#[relationship_target]`
//!   with mutations via `commands.insert()`, never `&mut` queries.
//! * **No redundant marker tags** — if a data component implies identity
//!   (e.g. `PlayerData` implies the entity is the player), skip the marker.
//! * **No Vec<T> inside components** when T has its own lifecycle —
//!   use relationships for independent sub-entities.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_reflect::Reflect;

// ===========================================================================
// Creature identity
// ===========================================================================

/// Creature identity — present on monsters and NPCs.
#[derive(Component, Debug, Clone, Reflect)]
pub struct Creature {
    pub def_id: String,
    pub name: String,
    pub species: crate::SpeciesId,
    pub symbol: char,
}

/// Player character data — only present on the player entity.
/// `With<PlayerData>` replaces `With<IsPlayer>`.
#[derive(Component, Debug, Clone, Reflect)]
pub struct PlayerData {
    pub name: String,
    #[reflect(ignore)]
    pub gender: Gender,
    pub age: u32,
    pub height: u32,
    pub blood_type: String,
    #[reflect(ignore)]
    pub profession: Option<crate::ProfessionId>,
    #[reflect(ignore)]
    pub scenario: Option<crate::ScenarioId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Reflect)]
pub enum Gender {
    #[default]
    Male,
    Female,
    NonBinary,
    Custom(String),
}

/// Non-player character data — present on NPCs.
/// `With<NpcData>` replaces `With<IsNPC>`.
#[derive(Component, Debug, Clone, Reflect)]
pub struct NpcData {
    pub name: String,
    pub npc_class: String,
    #[reflect(ignore)]
    pub personality: NpcPersonality,
    pub dialogue_id: Option<String>,
    pub schedule: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Reflect)]
pub struct NpcPersonality {
    pub aggression: i32,
    pub bravery: i32,
    pub altruism: i32,
    pub collector: i32,
}

// ===========================================================================
// Stats
// ===========================================================================

#[derive(Component, Debug, Clone, Reflect)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

/// Character attributes (strength, dexterity, intelligence, perception).
/// Re-exported from `crate::core::stats::Stats`, which derives `Component` directly.
pub use crate::core::stats::Stats;

/// Faction affiliation.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct Faction {
    pub id: crate::FactionId,
}

/// Body temperature in degrees Celsius.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct BodyTemperature(pub f64);

/// Wetness level (0 = dry, higher = wetter).
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Wetness(pub u32);

// ===========================================================================
// Combat
// ===========================================================================

/// Damage reduction (applied before health loss is calculated).
#[derive(Debug, Clone, Copy, Reflect)]
pub struct DamageReduction {
    pub bash: u32,
    pub cut: u32,
    pub pierce: u32,
    pub bullet: u32,
    pub fire: u32,
    pub acid: u32,
    pub electric: u32,
    pub cold: u32,
}

/// Combat statistics.
#[derive(Component, Debug, Clone, Reflect)]
pub struct CombatStats {
    pub melee_skill: i32,
    pub melee_dice: i32,
    pub melee_dice_sides: i32,
    pub dodge: i32,
    pub armor: DamageReduction,
}

/// Vision range (day/night).
#[derive(Component, Debug, Clone, Reflect)]
pub struct Vision {
    pub day_range: i32,
    pub night_range: i32,
}

// ===========================================================================
// Skills — relationship-based (one entity per skill)
// ===========================================================================

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = CreatureSkills)]
pub struct SkillOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = SkillOf, linked_spawn)]
pub struct CreatureSkills(Vec<Entity>);

impl CreatureSkills {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// Data on a skill entity: which skill, its current level and XP.
#[derive(Component, Debug, Clone, Reflect)]
pub struct SkillEntry {
    pub skill_id: crate::SkillId,
    pub level: u32,
    pub experience: u32,
}

// ===========================================================================
// Mutations — relationship-based (one entity per active mutation)
// ===========================================================================

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = CreatureMutations)]
pub struct MutationOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = MutationOf, linked_spawn)]
pub struct CreatureMutations(Vec<Entity>);

impl CreatureMutations {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// Data on a mutation entity: which mutation and whether it is visually apparent.
#[derive(Component, Debug, Clone, Reflect)]
pub struct MutationEntry {
    pub id: crate::MutationId,
    pub visible: bool,
}

// ===========================================================================
// Proficiencies — relationship-based (one entity per known proficiency)
// ===========================================================================

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = CreatureProficiencies)]
pub struct ProficiencyOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = ProficiencyOf, linked_spawn)]
pub struct CreatureProficiencies(Vec<Entity>);

impl CreatureProficiencies {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// Data on a proficiency entity: which proficiency is known.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ProficiencyEntry {
    pub id: crate::ProficiencyId,
}

// ===========================================================================
// Bionics
// ===========================================================================

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = InstalledBionics)]
pub struct BionicOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = BionicOf, linked_spawn)]
pub struct InstalledBionics(Vec<Entity>);

impl InstalledBionics {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct Bionic {
    pub bionic_id: crate::BionicId,
    pub active: bool,
    pub power_used: crate::Energy,
}

// ===========================================================================
// Morale
// ===========================================================================

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = MoraleBonuses)]
pub struct MoraleBonusOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = MoraleBonusOf, linked_spawn)]
pub struct MoraleBonuses(Vec<Entity>);

impl MoraleBonuses {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct MoraleBonus {
    pub reason: String,
    pub amount: i32,
    pub remaining: crate::Time,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Morale(pub i32);

// ===========================================================================
// Status effects
// ===========================================================================

#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = ActiveEffects)]
pub struct EffectOn(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = EffectOn, linked_spawn)]
pub struct ActiveEffects(Vec<Entity>);

impl ActiveEffects {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct StatusEffect {
    pub effect_id: crate::EffectId,
    pub intensity: u32,
    pub remaining: crate::Time,
}

// ===========================================================================
// Body parts
// ===========================================================================

#[derive(Component, Debug, Clone, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = CreatureBodyParts)]
pub struct BodyPartOf(pub Entity);

#[derive(Component, Debug, Clone, Reflect)]
#[relationship_target(relationship = BodyPartOf, linked_spawn)]
pub struct CreatureBodyParts(Vec<Entity>);

impl CreatureBodyParts {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct BodyPartDef(pub Entity);

#[derive(Component, Debug, Clone, Reflect)]
pub struct BodyPartSlot(pub String);

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct BodyPartHp {
    pub max: f32,
    pub current: f32,
    pub damage_multiplier: f32,
}

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[component(storage = "SparseSet")]
pub struct BodyPartBroken;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[component(storage = "SparseSet")]
pub struct BodyPartSevered;

// ===========================================================================
// Status markers
// ===========================================================================

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct IsAlive;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Stunned;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Bleeding;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct OnFire;

// ===========================================================================
// Turn scheduling
// ===========================================================================

/// Current action points (move points) for this entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct ActionPoints(pub i32);

impl Default for ActionPoints {
    fn default() -> Self {
        Self(0)
    }
}

/// How many action points this entity gains per turn (base 100).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct Speed(pub i32);

impl Default for Speed {
    fn default() -> Self {
        Self(100)
    }
}

/// Number of grasping hands this creature has.
///
/// Limits how many items can be wielded simultaneously:
/// normal humans have 2, four-armed mutations have 4, etc.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct HandCount(pub u8);

impl Default for HandCount {
    fn default() -> Self {
        Self(2)
    }
}
