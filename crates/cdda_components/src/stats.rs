use crate::actor::IsAlive;
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

/// Default value for each stat (matching CDDA's base character attributes).
pub const STAT_DEFAULT: u32 = 8;
/// Minimum value for base stats (clamped in `Stats::new`).
pub const STAT_MIN: u32 = 1;
/// Maximum value for base stats (clamped in `Stats::new` and `Stats::effective`).
pub const STAT_MAX: u32 = 20;

/// Core creature statistics.
///
/// These are the base attributes that every creature has.
///
/// # Requirements
/// Every entity with `Stats` is implicitly alive.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[require(IsAlive)]
pub struct Stats {
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub perception: u32,
}

impl Stats {
    /// Create stats clamped to `[STAT_MIN, STAT_MAX]`.
    pub fn new(strength: u32, dexterity: u32, intelligence: u32, perception: u32) -> Self {
        Stats {
            strength: strength.clamp(STAT_MIN, STAT_MAX),
            dexterity: dexterity.clamp(STAT_MIN, STAT_MAX),
            intelligence: intelligence.clamp(STAT_MIN, STAT_MAX),
            perception: perception.clamp(STAT_MIN, STAT_MAX),
        }
    }

    /// Return effective stats after applying bonuses.
    ///
    /// Each stat is computed as `base + bonus`, then clamped to `[0, STAT_MAX]`.
    /// Unlike `Stats::new`, effective stats can drop to 0 (incapacitated).
    pub fn effective(&self, bonuses: &StatBonuses) -> Stats {
        let clamp_eff = |base: u32, bonus: i32| -> u32 {
            let v = (base as i32).saturating_add(bonus);
            v.clamp(0, STAT_MAX as i32) as u32
        };
        Stats {
            strength: clamp_eff(self.strength, bonuses.strength),
            dexterity: clamp_eff(self.dexterity, bonuses.dexterity),
            intelligence: clamp_eff(self.intelligence, bonuses.intelligence),
            perception: clamp_eff(self.perception, bonuses.perception),
        }
    }
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            strength: STAT_DEFAULT,
            dexterity: STAT_DEFAULT,
            intelligence: STAT_DEFAULT,
            perception: STAT_DEFAULT,
        }
    }
}

/// Flat bonuses applied on top of base stats (e.g. from enchantments, mutations).
///
/// Positive values increase the stat; negative values decrease it.
/// All fields default to 0.
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect,
)]
pub struct StatBonuses {
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
    pub perception: i32,
    /// Movement speed modifier (not part of the 4 core stats but stored here
    /// for convenience, matching CDDA's enchantment system).
    pub speed: i32,
}
