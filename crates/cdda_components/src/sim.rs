//! Spatial and projectile components owned by `cdda_sim`.
//!
//! Actor components live in `crate::actor::components`.
//! Item components live in `crate::item::components`.
//! Import directly from those crates rather than through this module.

use bevy_ecs::component::Component;
use bevy_ecs::prelude::Resource;
use bevy_reflect::Reflect;
use cdda_core_types::core::coords::WorldPos;

#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
pub struct WorldPosition(#[reflect(ignore)] pub WorldPos);

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct Solid;

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Velocity {
    pub dx: i32,
    pub dy: i32,
}

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct InFlight;

// ---------------------------------------------------------------------------
// GameTime — in-game clock
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTime {
    pub turn: u64,
}

impl Default for GameTime {
    fn default() -> Self {
        Self { turn: 0 }
    }
}

impl GameTime {
    pub fn advance(&mut self) {
        self.turn += 1;
    }
    pub fn hours_elapsed(&self) -> u64 {
        (self.turn * 6) / 3600
    }
    pub const TURNS_PER_DAY: u64 = 14400;
}

// TurnAdvanced message — defined in the messages module.
pub use crate::messages::TurnAdvanced;
