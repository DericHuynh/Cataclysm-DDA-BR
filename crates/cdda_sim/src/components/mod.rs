//! ECS components for the simulation layer.
//!
//! Static item data (name, weight, weapon stats, etc.) comes from
//! definition entities cloned at spawn time — see
//! `cdda_sim::def_components` for those types.
//!
//! This module only contains **runtime mutable** components
//! (health, position, turn state, relationships) and thin wrappers
//! (`CurrentCharges`, `LoadedAmmo`) over mutable instance state.
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
use cdda_core::coords::WorldPos;
use cdda_core::Damage;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Spatial
// ---------------------------------------------------------------------------

/// World position of an entity.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WorldPosition(pub WorldPos);

/// Entity occupies a tile (can't share with other solid entities).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Solid;

/// Velocity vector for movement interpolation.
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub dx: i32,
    pub dy: i32,
}

// ===========================================================================
// Item — mutable runtime state only
// ===========================================================================

/// How many of this item stack are present.
///
/// # Contract
/// Must be >= 1.  Systems that decrement to zero must despawn the entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct StackCount(u32);

impl StackCount {
    pub fn new(n: u32) -> Self {
        assert!(n >= 1, "StackCount must be >= 1");
        Self(n)
    }
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Current charges remaining (tools, batteries, magazines).
/// Cloned from definition's `ToolData.max_charges` or `MagazineData.capacity`
/// at spawn time; mutated at runtime.
#[derive(Component, Debug, Clone, Copy)]
pub struct CurrentCharges(pub i32);

impl Default for CurrentCharges {
    fn default() -> Self {
        Self(0)
    }
}

/// Ammo loaded in a magazine (separate from CurrentCharges).
/// Set at spawn time from `MagazineData.default_ammo` count.
#[derive(Component, Debug, Clone, Copy)]
pub struct LoadedAmmo(pub i32);

impl Default for LoadedAmmo {
    fn default() -> Self {
        Self(0)
    }
}

/// Food item spoils over time.
#[derive(Component, Debug, Clone)]
pub struct Spoilable {
    /// The rotten version of this item template.
    pub rotten: cdda_core::ItemId,
    /// Total spoil time (turns).
    pub total: cdda_core::Time,
    /// Remaining before spoilage.
    pub remaining: cdda_core::Time,
}

/// Item is damaged / degraded.
#[derive(Component, Debug, Clone, Copy)]
pub struct ItemDamage(pub u32);

// ===========================================================================
// Container tags (zero-sized, marker)
// ===========================================================================

/// Container is sealed.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Sealed;

/// Container has rigid walls.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Rigid;

/// Container contents are hidden from view.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Watertight;

/// Container preserves temperature.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PreservesTemp;

/// Container is fireproof.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Fireproof;

/// Container is gas-tight.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct GasTight;

// ===========================================================================
// Creature core
// ===========================================================================

/// Creature identity — present on monsters and NPCs.
#[derive(Component, Debug, Clone)]
pub struct Creature {
    /// String ID of the definition entity this creature was spawned from (e.g. "mon_zombie").
    pub def_id: String,
    pub name: String,
    pub species: cdda_core::SpeciesId,
    pub symbol: char,
}

/// Damage reduction (applied before health loss is calculated).
#[derive(Debug, Clone, Copy)]
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
#[derive(Component, Debug, Clone)]
pub struct CombatStats {
    pub melee_skill: i32,
    pub melee_dice: i32,
    pub melee_dice_sides: i32,
    pub dodge: i32,
    pub armor: DamageReduction,
}

/// Vision range (day/night).
#[derive(Component, Debug, Clone)]
pub struct Vision {
    pub day_range: i32,
    pub night_range: i32,
}

// ===========================================================================
// Creature state
// ===========================================================================

#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Faction {
    pub id: cdda_core::FactionId,
}

/// Character attributes (strength, dexterity, intelligence, perception).
#[derive(Component, Debug, Clone, Copy)]
pub struct Stats(pub cdda_core::Stats);

/// Body temperature in degrees Celsius.
#[derive(Component, Debug, Clone, Copy)]
pub struct BodyTemperature(pub f64);

/// Wetness level (0 = dry, higher = wetter).
#[derive(Component, Debug, Clone, Copy)]
pub struct Wetness(pub u32);

// ===========================================================================
// Creature progression — skills, mutations, proficiencies
// ===========================================================================

/// Skill levels for a creature.  O(1) lookup by SkillId.
#[derive(Component, Debug, Clone)]
pub struct SkillSet {
    pub skills: HashMap<cdda_core::SkillId, SkillLevel>,
}

#[derive(Debug, Clone, Copy)]
pub struct SkillLevel {
    pub level: u32,
    pub experience: u32,
}

/// Active mutations on a creature.
/// Each mutation tracks its own visibility flag, avoiding the
/// subset-of-visible invariant that two separate Vecs would create.
#[derive(Component, Debug, Clone)]
pub struct Mutations {
    pub active: Vec<MutationState>,
}

#[derive(Debug, Clone)]
pub struct MutationState {
    pub id: cdda_core::MutationId,
    pub visible: bool,
}

/// Proficiencies known by a creature.  O(1) membership check.
#[derive(Component, Debug, Clone)]
pub struct ProficiencySet {
    pub known: HashSet<cdda_core::ProficiencyId>,
}

// ===========================================================================
// Bionics — installed cybernetic implants
// ===========================================================================

/// Spawned as a child entity of the creature, related via BionicOf.
#[derive(Component)]
#[relationship(relationship_target = InstalledBionics)]
pub struct BionicOf(pub Entity);

/// On the creature entity — collects all installed bionics.
#[derive(Component)]
#[relationship_target(relationship = BionicOf, linked_spawn)]
pub struct InstalledBionics(Vec<Entity>);

impl InstalledBionics {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// Data for a single installed bionic — lives on the bionic entity.
#[derive(Component, Debug, Clone)]
pub struct Bionic {
    pub bionic_id: cdda_core::BionicId,
    pub active: bool,
    pub power_used: cdda_core::Energy,
}

// ===========================================================================
// Morale — per-bonus entity
// ===========================================================================

/// On the bonus entity — points to the creature it affects.
/// # Mutation
/// Do not query this as `&mut`. To remove a bonus, despawn the entity.
#[derive(Component)]
#[relationship(relationship_target = MoraleBonuses)]
pub struct MoraleBonusOf(pub Entity);

/// On the creature entity — collects all active morale bonuses.
#[derive(Component)]
#[relationship_target(relationship = MoraleBonusOf, linked_spawn)]
pub struct MoraleBonuses(Vec<Entity>);

impl MoraleBonuses {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// A single morale bonus.
#[derive(Component, Debug, Clone)]
pub struct MoraleBonus {
    pub reason: String,
    pub amount: i32,
    pub remaining: cdda_core::Time,
}

/// Current morale total — computed by the MoraleSystem from MoraleBonuses.
/// Systems that only need the total should read this, not MoraleBonuses.
#[derive(Component, Debug, Clone, Copy)]
pub struct Morale(pub i32);

// ===========================================================================
// Status effects — per-effect entity
// ===========================================================================

/// On the effect entity — points to the creature affected.
/// # Mutation
/// Do not query this as `&mut`. To remove an effect, despawn the entity.
#[derive(Component)]
#[relationship(relationship_target = ActiveEffects)]
pub struct EffectOn(pub Entity);

/// On the creature entity — collects all active status effects.
#[derive(Component)]
#[relationship_target(relationship = EffectOn, linked_spawn)]
pub struct ActiveEffects(Vec<Entity>);

impl ActiveEffects {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// A single active status effect.
#[derive(Component, Debug, Clone)]
pub struct StatusEffect {
    pub effect_id: cdda_core::EffectId,
    pub intensity: u32,
    pub remaining: cdda_core::Time,
}

// ===========================================================================
// Player / NPC identity
// ===========================================================================

/// Player character data — only present on the player entity.
/// Markers are redundant; `With<PlayerData>` replaces `With<IsPlayer>`.
#[derive(Component, Debug, Clone)]
pub struct PlayerData {
    pub name: String,
    pub gender: Gender,
    pub age: u32,
    pub height: u32,
    pub blood_type: String,
    pub profession: Option<cdda_core::ProfessionId>,
    pub scenario: Option<cdda_core::ScenarioId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    NonBinary,
    Custom(String),
}

/// Non-player character data — present on NPCs.
/// Markers are redundant; `With<NpcData>` replaces `With<IsNPC>`.
#[derive(Component, Debug, Clone)]
pub struct NpcData {
    pub name: String,
    pub npc_class: String,
    pub personality: NpcPersonality,
    pub dialogue_id: Option<String>,
    pub schedule: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct NpcPersonality {
    pub aggression: i32,
    pub bravery: i32,
    pub altruism: i32,
    pub collector: i32,
}

// ===========================================================================
// Relationships — inventory containment, equipment, attachments
// ===========================================================================

// -- Containment ------------------------------------------------------------

/// On the item entity — points to the container it is inside.
/// # Mutation
/// Do not query this as `&mut`. Reinsert via commands:
/// `commands.entity(item).insert(InsideContainer(new_container));`
#[derive(Component)]
#[relationship(relationship_target = ContainerContents)]
pub struct InsideContainer(pub Entity);

/// On the container entity — collects all items inside it.
#[derive(Component)]
#[relationship_target(relationship = InsideContainer, linked_spawn)]
pub struct ContainerContents(Vec<Entity>);

impl ContainerContents {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Wielding ---------------------------------------------------------------

/// On the item entity — points to the creature wielding it.
/// # Mutation
/// Do not query this as `&mut`. Reinsert via commands:
/// `commands.entity(item).insert(WieldedBy(new_wielder));`
#[derive(Component)]
#[relationship(relationship_target = WieldedItems)]
pub struct WieldedBy(pub Entity);

/// On the wielder entity — collects all wielded items.
#[derive(Component)]
#[relationship_target(relationship = WieldedBy, linked_spawn)]
pub struct WieldedItems(Vec<Entity>);

impl WieldedItems {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// -- Wearing ----------------------------------------------------------------

/// On the item entity — points to the creature wearing it.
/// # Mutation
/// Do not query this as `&mut`. Reinsert via commands:
/// `commands.entity(item).insert(WornOn { wearer, slot: Some(BodyPartSlot::Torso) });`
#[derive(Component)]
#[relationship(relationship_target = WornBy)]
pub struct WornOn {
    #[relationship]
    pub wearer: Entity,
    pub slot: Option<BodyPartSlot>,
}

/// On the wearer entity — collects all worn items.
#[derive(Component)]
#[relationship_target(relationship = WornOn, linked_spawn)]
pub struct WornBy(Vec<Entity>);

impl WornBy {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// The body part where an item is worn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyPartSlot {
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

// -- Pocket attachment ------------------------------------------------------

/// On the pocket entity — points to the attachment slot it's mounted on.
#[derive(Component)]
#[relationship(relationship_target = MountedPockets)]
pub struct MountedOn(pub Entity);

/// On the slot entity — collects all mounted pockets.
#[derive(Component)]
#[relationship_target(relationship = MountedOn, linked_spawn)]
pub struct MountedPockets(Vec<Entity>);

impl MountedPockets {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

// ===========================================================================
// Pocket system — dynamic, composable inventory
// ===========================================================================

/// A single pocket in a container.  Each pocket is its own entity (child of
/// the container), allowing pockets to be added/removed at runtime (e.g.
/// attaching a holster to a belt, or MOLLE pouches to a vest).
///
/// Boolean behaviour is controlled by tag components on the pocket entity:
/// `With<Sealed>`, `With<Rigid>`, `With<Watertight>`, `With<Fireproof>`,
/// `With<GasTight>`, `With<PreservesTemp>`.
#[derive(Component, Debug, Clone)]
pub struct Pocket {
    /// Max volume this pocket holds.
    pub max_volume: cdda_core::Volume,
    /// Max weight this pocket holds.
    pub max_weight: cdda_core::Weight,
    /// Longest item that fits.
    pub max_item_length: cdda_core::Length,
    /// Minimum item volume (sieve — keep tiny things out if desired).
    pub min_item_volume: cdda_core::Volume,
    /// Pocket category.
    pub pocket_type: PocketType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PocketType {
    /// General-purpose (backpack, box).
    Container,
    /// Internal magazine (integrated in a firearm).
    Magazine,
    /// Magazine well (accepts detachable magazines).
    MagazineWell,
    /// Shaped holster for a specific weapon or tool.
    Holster,
    /// Special-purpose pocket with custom rules.
    Special,
}

/// Restricts what a pocket can accept.
#[derive(Component, Debug, Clone)]
pub struct PocketRestriction {
    /// Only accept items with these flag strings.
    pub allowed_flags: Vec<String>,
    /// Only accept specific item IDs.
    pub allowed_items: Vec<cdda_core::ItemId>,
    /// Only accept items of this ammo type (magazines).
    pub ammo_type: Option<String>,
    /// Only accept items of this category.
    pub item_category: Option<String>,
    /// Minimum / maximum item volume.
    pub max_item_volume: cdda_core::Volume,
}

/// An attachment point on a piece of gear where another pocket can be
/// mounted (MOLLE webbing, belt loop, clip, etc.).
#[derive(Component, Debug, Clone)]
pub struct AttachmentSlot {
    pub slot_type: AttachmentType,
    /// Maximum volume of the pocket that can be attached here.
    pub max_volume: cdda_core::Volume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentType {
    Molle,
    Belt,
    Clip,
    Velcro,
    Universal,
}

// ===========================================================================
// Status markers (zero-sized)
// ===========================================================================

/// Entity is alive (has Health, can die).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct IsAlive;

/// Entity is currently stunned.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Stunned;

/// Entity is bleeding (ticking damage over time).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Bleeding;

/// Entity is on fire.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OnFire;

/// Entity is a projectile in flight.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InFlight;

/// The container has its own entity and is not stored in a map grid.
/// Items inside it are children connected via InsideContainer/ContainerContents.
#[derive(Component, Debug, Clone)]
pub struct Container {
    pub capacity: cdda_core::Volume,
}

// ===========================================================================
// Turn scheduling — action point system
// ===========================================================================

/// Current action points (move points) for this entity.
///
/// Gained each turn based on `Speed`. Spent to perform actions.
/// Carryover from previous turn (can be negative = debt).
/// Clamped: minimum = -Speed*2 (can't accrue more than 2 turns of debt).
///
/// Reference: Section 8 — The Turn & Action Point System
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovePoints(pub i32);

impl Default for MovePoints {
    fn default() -> Self {
        Self(0)
    }
}

/// How many action points this entity gains per turn (base 100).
///
/// Speed 100 = average human. Speed 200 = twice as fast (acts twice as often).
/// Speed 50 = half speed (acts every other turn).
///
/// Reference: Section 8 — The Turn & Action Point System
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Speed(pub i32);

impl Default for Speed {
    fn default() -> Self {
        Self(100)
    }
}
