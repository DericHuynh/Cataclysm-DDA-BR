//! Dev camera movement — navigate the building showcase grid.
//!
//! Simple 2D camera panning over the OMT grid. The camera position
//! is stored as a resource and read by the ASCII renderer.

use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use crate::input::{Direction, GameAction, InputAction};

// ---------------------------------------------------------------------------
// DevCamera
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// dev_camera_move
// ---------------------------------------------------------------------------

/// Reads movement `InputAction` messages and pans the `DevCamera`.
///
/// Arrow keys / vi-keys move the camera by 1 OMT per press.
/// `<` / `>` change Z level.
pub fn dev_camera_move(mut reader: MessageReader<InputAction>, mut camera: ResMut<DevCamera>) {
    for event in reader.read() {
        match &event.action {
            GameAction::Move(dir) => match dir {
                Direction::North => camera.y = camera.y.saturating_sub(1),
                Direction::South => camera.y = camera.y.saturating_add(1),
                Direction::West => camera.x = camera.x.saturating_sub(1),
                Direction::East => camera.x = camera.x.saturating_add(1),
                Direction::NorthWest => {
                    camera.y = camera.y.saturating_sub(1);
                    camera.x = camera.x.saturating_sub(1);
                }
                Direction::NorthEast => {
                    camera.y = camera.y.saturating_sub(1);
                    camera.x = camera.x.saturating_add(1);
                }
                Direction::SouthWest => {
                    camera.y = camera.y.saturating_add(1);
                    camera.x = camera.x.saturating_sub(1);
                }
                Direction::SouthEast => {
                    camera.y = camera.y.saturating_add(1);
                    camera.x = camera.x.saturating_add(1);
                }
                Direction::Up => camera.z = camera.z.saturating_add(1),
                Direction::Down => camera.z = camera.z.saturating_sub(1),
                Direction::Here => {} // wait — no movement
            },
            GameAction::NavigateUp => camera.y = camera.y.saturating_sub(1),
            GameAction::NavigateDown => camera.y = camera.y.saturating_add(1),
            GameAction::NavigateLeft => camera.x = camera.x.saturating_sub(1),
            GameAction::NavigateRight => camera.x = camera.x.saturating_add(1),
            _ => {}
        }
    }
}
