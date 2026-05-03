//! # Item templates
//!
//! Core item definition types.  `ItemTemplate` is the top-level blueprint for
//! every game item, with optional sub-behaviours for weapons, armour,
//! containers, food, tools, ammo, magazines, and books.

use crate::damage::Damage;
use crate::flags::FlagSet;
use crate::id::*;
use crate::units::*;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Phase
// ---------------------------------------------------------------------------

/// The physical phase of matter an item is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    Plasma,
}

// ---------------------------------------------------------------------------
// Pocket type
// ---------------------------------------------------------------------------

/// The type of pocket a container offers.
///
/// Dictates what kinds of items can be inserted and how the pocket behaves
/// during reload / withdraw actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PocketType {
    /// General-purpose container pocket (backpack, box, etc.).
    Container,
    /// Internal magazine (integrated in a firearm).
    Magazine,
    /// Magazine well (accepts detachable magazines).
    MagazineWell,
    /// Holster (shaped for a specific weapon or tool).
    Holster,
    /// Special-purpose pocket with custom rules.
    Special,
}

// ---------------------------------------------------------------------------
// Count mode
// ---------------------------------------------------------------------------

/// How an item is counted — single item, by discrete count, or by charges
/// (e.g. liquid volume, fuel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CountMode {
    /// A single discrete item (the default).
    Single,
    /// Items that stack by count (nails, bolts, …).
    ByCount { default: u32, max: Option<u32> },
    /// Items that stack by charges (gasoline, water, …).
    Charges { default: u32, max: Option<u32> },
}

// ---------------------------------------------------------------------------
// Container tag
// ---------------------------------------------------------------------------

/// Behavioural tags for containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ContainerTag {
    /// Contents are sealed inside.
    Sealed,
    /// The container has rigid walls.
    Rigid,
    /// Contents are hidden from view (e.g. a safe).
    Watertight,
    /// The container preserves temperature.
    Preserves,
}

// ---------------------------------------------------------------------------
// Container behaviour
// ---------------------------------------------------------------------------

/// Describes a container's capacity and what it can hold.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerBehavior {
    /// How much volume the container can hold.
    pub capacity: Volume,
    /// Maximum volume of a single item that can fit (target spec).
    pub max_volume: Volume,
    /// Maximum weight the container can hold (target spec).
    pub max_weight: Weight,
    /// Maximum length of a single item that can fit (target spec).
    pub max_item_length: Length,
    /// The pocket type (target spec).
    pub pocket_type: PocketType,
    /// Behavioural tags.
    pub tags: BTreeSet<ContainerTag>,
}

// ---------------------------------------------------------------------------
// Weapon (melee / ranged) behaviour
// ---------------------------------------------------------------------------

/// Melee and ranged weapon stats.
#[derive(Debug, Clone, PartialEq)]
pub struct WeaponBehavior {
    /// Base damage per damage category.
    pub damage: Damage,
    /// To-hit bonus (target spec).
    pub to_hit: i32,
    /// Martial-arts techniques this weapon enables (target spec).
    pub techniques: Vec<TechniqueId>,
    /// Reach in tiles (target spec).
    pub reach: u32,
    /// Number of damage dice.
    pub dice: u32,
    /// Sides per die.
    pub dice_sides: u32,
    /// Range in tiles (0 = melee-only).
    pub range: u32,
    /// Base ranged damage.
    pub ranged_damage: u32,
    /// Dispersion penalty.
    pub dispersion: u32,
    /// Recoil generated per shot.
    pub recoil: u32,
    /// Reload time (turns).
    pub reload_time: u32,
    /// Skill used with this weapon.
    pub skill: SkillId,
}

// ---------------------------------------------------------------------------
// Armour behaviour
// ---------------------------------------------------------------------------

/// Protection and coverage data for worn items.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorBehavior {
    /// Layering: base, belt, outer, etc.
    pub covers: Vec<BodyPart>,
    /// Encumbrance penalty.
    pub encumbrance: u32,
    /// Coverage fraction (0–100).
    pub coverage: u32,
    /// Bash protection.
    pub bash_protection: u32,
    /// Cut protection.
    pub cut_protection: u32,
    /// Warmth value.
    pub warmth: u32,
    /// Whether this can be worn over another item.
    pub layering: u32,
}

// ---------------------------------------------------------------------------
// Food behaviour
// ---------------------------------------------------------------------------

/// Food / drink / comestible properties.
#[derive(Debug, Clone, PartialEq)]
pub struct FoodBehavior {
    /// Nutrition in kcal.
    pub calories: u32,
    /// Quench value (thirst reduction).
    pub quench: u32,
    /// Fun value (morale effect).
    pub fun: i32,
    /// Spoilage time.
    pub spoils_in: Time,
    /// Parasite risk.
    pub parasites: u32,
    /// How healthy this food is (negative = junk, positive = healthy).
    pub healthy: i32,
}

// ---------------------------------------------------------------------------
// Tool tag
// ---------------------------------------------------------------------------

/// Behavioural tags for tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolTag {
    /// The tool needs charges to work.
    UsesCharges,
}

// ---------------------------------------------------------------------------
// Tool behaviour
// ---------------------------------------------------------------------------

/// Properties for items that act as tools.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolBehavior {
    /// Behavioural tags.
    pub tags: BTreeSet<ToolTag>,
    /// Turns needed per use.
    pub turns_per_use: u32,
    /// Ammo type consumed (if any).
    pub ammo_id: Option<ItemId>,
    /// Max charges the tool can hold.
    pub max_charges: u32,
    /// Charges consumed per use.
    pub charges_per_use: u32,
}

// ---------------------------------------------------------------------------
// Ammo behaviour
// ---------------------------------------------------------------------------

/// Properties for ammunition.
#[derive(Debug, Clone, PartialEq)]
pub struct AmmoBehavior {
    /// The ammo type identifier (e.g. "223", "9mm").
    pub ammo_type: String,
    /// Damage dealt by this ammo.
    pub damage: u32,
    /// Armour penetration.
    pub pierce: u32,
    /// Range multiplier.
    pub range: u32,
    /// Dispersion penalty.
    pub dispersion: u32,
    /// Recoil generated.
    pub recoil: u32,
    /// Number of rounds per count.
    pub count: u32,
}

// ---------------------------------------------------------------------------
// Magazine behaviour
// ---------------------------------------------------------------------------

/// Properties for magazines (ammo holders).
#[derive(Debug, Clone, PartialEq)]
pub struct MagazineBehavior {
    /// Ammo type this magazine accepts.
    pub ammo_type: String,
    /// Max capacity in rounds.
    pub capacity: u32,
    /// Default ammo count when spawned.
    pub default_ammo: u32,
    /// Reload time per round.
    pub reload_time: u32,
    /// Linkage item (belt links, speed loader, etc.).
    pub linkage: Option<ItemId>,
}

// ---------------------------------------------------------------------------
// Book behaviour
// ---------------------------------------------------------------------------

/// Properties for books and manuals.
#[derive(Debug, Clone, PartialEq)]
pub struct BookBehavior {
    /// Turns needed to read one chapter.
    pub time: Time,
    /// Skill this book trains.
    pub skill: Option<SkillId>,
    /// Required skill level to understand.
    pub required_level: u32,
    /// Max skill level this book can train to.
    pub max_level: u32,
    /// Fun value while reading.
    pub fun: i32,
    /// Intelligence requirement.
    pub int_requirement: u32,
    /// Chapters in the book.
    pub chapters: u32,
}

// ---------------------------------------------------------------------------
// Body part (used by armour)
// ---------------------------------------------------------------------------

/// Body-part category — used to describe what an armour piece covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyPart {
    Head,
    Eyes,
    Mouth,
    Torso,
    ArmLeft,
    ArmRight,
    HandLeft,
    HandRight,
    LegLeft,
    LegRight,
    FootLeft,
    FootRight,
}

// ---------------------------------------------------------------------------
// ItemBase — fields common to every item
// ---------------------------------------------------------------------------

/// Fields present on every item template, regardless of its sub-behaviours.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemBase {
    /// Display name.
    pub name: String,
    /// Flavour / examine text.
    pub description: String,
    /// How much space the item occupies.
    pub volume: Volume,
    /// How heavy the item is.
    pub weight: Weight,
    /// Materials the item is made from.
    pub material: Vec<MaterialId>,
    /// Map-display character.
    pub symbol: char,
    /// Map-display colour (target spec).
    pub color: String,
    /// Boolean tags.
    pub flags: FlagSet,
    /// Physical phase (solid / liquid / gas / plasma).
    pub phase: Phase,
    /// Item category (target spec).
    pub category: String,
}

// ---------------------------------------------------------------------------
// Gun-mod behaviour
// ---------------------------------------------------------------------------

/// Properties for gun modifications (scopes, suppressors, grips, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct GunModBehavior {
    /// Turns needed to install this mod.
    pub install_time: Time,
    /// Slot names this mod fits into (e.g. "barrel", "stock").
    pub modifies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Drug behaviour
// ---------------------------------------------------------------------------

/// Properties for consumable drugs / chemicals.
#[derive(Debug, Clone, PartialEq)]
pub struct DrugBehavior {
    /// Effect descriptions applied when consumed.
    pub effects: Vec<String>,
    /// How long the effects last.
    pub duration: Time,
    /// Addiction potential (higher = more addictive).
    pub addiction_potential: u32,
}

// ---------------------------------------------------------------------------
// ItemTemplate — the top-level item blueprint
// ---------------------------------------------------------------------------

/// The complete blueprint for any game item.
///
/// Optional sub-structs encode *behaviour* — a crowbar has
/// `weapon: Some(…)`, a helmet has `armor: Some(…)`, canned soup has
/// `food: Some(…)`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemTemplate {
    pub base: ItemBase,
    pub weapon: Option<WeaponBehavior>,
    pub armor: Option<ArmorBehavior>,
    pub container: Option<ContainerBehavior>,
    pub food: Option<FoodBehavior>,
    pub tool: Option<ToolBehavior>,
    pub ammo: Option<AmmoBehavior>,
    pub magazine: Option<MagazineBehavior>,
    pub book: Option<BookBehavior>,
    pub gun_mod: Option<GunModBehavior>,
    pub drug: Option<DrugBehavior>,
}
