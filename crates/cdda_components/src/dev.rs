//! Dev/debug marker components and resources.
//!
//! Moved here from `cdda_core::worldgen::dev` and `cdda_core::worldgen::dev_move`
//! to break circular dependencies when extracting subsystems into separate crates.

use bevy_ecs::component::Component;
use bevy_ecs::prelude::Resource;

/// Marker for the dev-world player entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct DevPlayer;

/// Display name for an item spawned on the ground in the dev world.
#[derive(Component, Debug, Clone)]
pub struct DevGroundItemName(pub String);

/// Camera position in OMT-grid coordinates for the dev-worldgen showcase.
#[derive(Resource, Debug, Clone)]
pub struct DevCamera {
    /// Current X position in OMT units.
    pub x: i32,
    /// Current Y position in OMT units.
    pub y: i32,
    /// Current Z level.
    pub z: i32,
}

impl Default for DevCamera {
    fn default() -> Self {
        Self { x: 0, y: 0, z: 0 }
    }
}

impl DevCamera {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}
