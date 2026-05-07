//! Spatial and projectile components owned by `cdda_sim`.
//!
//! Actor components live in `cdda_actor::components`.
//! Item components live in `cdda_item::components`.
//! Import directly from those crates rather than through this module.

use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use cdda_core::coords::WorldPos;

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
