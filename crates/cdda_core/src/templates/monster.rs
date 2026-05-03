//! # Monster templates
//!
//! Blueprint types for creature / monster definitions.  `MonsterTemplate`
//! bundles baseline properties, combat stats, vision, armour, species,
//! special attacks, death drops, and upgrade paths.

use crate::flags::FlagSet;
use crate::id::*;
use crate::units::*;

// ---------------------------------------------------------------------------
// MonsterBase — fields common to every monster
// ---------------------------------------------------------------------------

/// Baseline properties present on every monster definition.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterBase {
    /// Display name.
    pub name: String,
    /// Flavour text / examine description.
    pub description: String,
    /// Physical size (affects packing, hit chance, etc.).
    pub volume: Volume,
    /// Mass.
    pub weight: Weight,
    /// Map-display character.
    pub symbol: char,
    /// Boolean tags that control AI, behaviour, and interaction rules.
    pub flags: FlagSet,
    /// Materials the monster is composed of (flesh, bone, steel, etc.).
    pub material: Vec<MaterialId>,
}

// ---------------------------------------------------------------------------
// MonsterStats — combat and movement stats
// ---------------------------------------------------------------------------

/// Core combat attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterStats {
    /// Hit points (how much damage it can take).
    pub hp: i32,
    /// Movement speed (higher = faster).
    pub speed: i32,
    /// Aggression (higher = more likely to attack).
    pub aggression: i32,
    /// Morale (higher = less likely to flee).
    pub morale: i32,
    /// Melee combat skill.
    pub melee_skill: i32,
    /// Number of melee damage dice.
    pub melee_dice: i32,
    /// Sides per melee damage die.
    pub melee_dice_sides: i32,
    /// Dodge skill.
    pub dodge: i32,
}

// ---------------------------------------------------------------------------
// Vision
// ---------------------------------------------------------------------------

/// Day and night vision ranges in tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct Vision {
    /// Vision range during daytime.
    pub day: i32,
    /// Vision range at night.
    pub night: i32,
}

// ---------------------------------------------------------------------------
// ArmorSet
// ---------------------------------------------------------------------------

/// Monster armour values, one `Damage` vector covering all damage types.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorSet(pub crate::damage::Damage);

// ---------------------------------------------------------------------------
// MonsterTemplate — the complete creature blueprint
// ---------------------------------------------------------------------------

/// The complete blueprint for a monster / NPC / creature.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterTemplate {
    pub base: MonsterBase,
    pub stats: MonsterStats,
    pub vision: Vision,
    pub armor: ArmorSet,
    /// Biological species / family (human, mammal, insect, …).
    pub species: SpeciesId,
    /// Special attacks this monster can perform.
    pub special_attacks: Vec<SpecialAttackId>,
    /// Item group rolled on death.
    pub death_drops: ItemGroupId,
    /// Optional upgrade path: what the monster turns into after some time
    /// (e.g. zombie → necromancer).
    pub upgrade_path: Option<(MonsterId, Time)>,
    /// Boolean tags (often overlaps with `base.flags`; kept separate for
    /// monster-specific querying).
    pub flags: FlagSet,
    /// Body type identifier (used for hit-location generation).
    pub body_type: String,
}
