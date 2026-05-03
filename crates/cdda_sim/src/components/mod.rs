//! ECS components for the simulation layer.
//!
//! Templates in `cdda_core::templates` are blueprints; these are the
//! runtime ECS components stamped onto entities at spawn time.
//!
//! ## Design: tag components over bool fields
//!
//! Boolean properties use zero-sized Bevy tag components instead of
//! struct fields. This enables:
//! - Efficient archetype queries: `Query<&Container, With<Sealed>>`
//! - No branching on bools in hot systems
//! - New tags can be added without modifying existing component structs

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use cdda_core::coords::WorldPos;
use cdda_core::Damage;

// ---------------------------------------------------------------------------
// Spatial
// ---------------------------------------------------------------------------

/// World position of an entity.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WorldPosition(pub WorldPos);

// ---------------------------------------------------------------------------
// Item components
// ---------------------------------------------------------------------------

/// Core item data present on every item entity.
#[derive(Component, Debug, Clone)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub volume: cdda_core::Volume,
    pub weight: cdda_core::Weight,
    pub symbol: char,
}

/// How many of this item stack are present.
#[derive(Component, Debug, Clone, Copy)]
pub struct StackCount(pub u32);

/// Weapon behaviour. Only entities with this can melee/ranged attack.
#[derive(Component, Debug, Clone)]
pub struct Weapon {
    pub damage: Damage,
    pub to_hit: i32,
    pub reach: u32,
    pub skill: cdda_core::SkillId,
}

/// Armor behaviour.
#[derive(Component, Debug, Clone)]
pub struct Armor {
    pub bash_protection: u32,
    pub cut_protection: u32,
    pub encumbrance: u32,
    pub warmth: u32,
}

/// Container behaviour. Entities with this can hold other items.
///
/// Tag components (zero-sized) control container properties:
/// - `With<Sealed>` — contents are sealed inside
/// - `With<Rigid>` — container has rigid walls
/// - `With<Watertight>` — contents hidden from view
/// - `With<PreservesTemp>` — preserves temperature
#[derive(Component, Debug, Clone)]
pub struct Container {
    pub capacity: cdda_core::Volume,
}

/// Food behaviour.
#[derive(Component, Debug, Clone)]
pub struct Food {
    pub calories: u32,
    pub quench: u32,
    pub fun: i32,
    pub spoils_in: cdda_core::Time,
}

/// Tool behaviour.
///
/// Tag: `With<UsesCharges>` — tool consumes charges per use
#[derive(Component, Debug, Clone)]
pub struct Tool {
    pub max_charges: u32,
    pub charges_per_use: u32,
}

// ---------------------------------------------------------------------------
// Item tag components (zero-sized, marker only)
// ---------------------------------------------------------------------------

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

/// Tool consumes charges per use.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct UsesCharges;

// ---------------------------------------------------------------------------
// Monster / Creature components
// ---------------------------------------------------------------------------

/// Active monster or NPC.
#[derive(Component, Debug, Clone)]
pub struct Creature {
    pub name: String,
    pub species: cdda_core::SpeciesId,
    pub symbol: char,
}

/// Combat statistics.
#[derive(Component, Debug, Clone)]
pub struct CombatStats {
    pub melee_skill: i32,
    pub melee_dice: i32,
    pub melee_dice_sides: i32,
    pub dodge: i32,
    pub armor: Damage,
}

/// Vision range (day/night).
#[derive(Component, Debug, Clone)]
pub struct Vision {
    pub day_range: i32,
    pub night_range: i32,
}

// ---------------------------------------------------------------------------
// Health / status
// ---------------------------------------------------------------------------

#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Faction {
    pub id: cdda_core::FactionId,
}

// ---------------------------------------------------------------------------
// Inventory relationships
// ---------------------------------------------------------------------------

/// Entity is inside a container.
#[derive(Component, Debug, Clone)]
pub struct InsideContainer {
    pub parent: Entity,
}

/// Entity is wielded by a creature.
#[derive(Component, Debug, Clone)]
pub struct WieldedBy {
    pub wielder: Entity,
}

/// Entity is worn by a creature.
#[derive(Component, Debug, Clone)]
pub struct WornBy {
    pub wearer: Entity,
}
