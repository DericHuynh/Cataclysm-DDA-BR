//! Overmap camera — viewport center and zoom config.
//!
//! Replaces `DevCamera` for the overmap viewer. Stored as a Bevy
//! `Resource`. Tracks the center OMT position of the viewport.

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::{OmPos, ZLevel};

/// Camera for the overmap viewer, tracking the center OMT tile.
///
/// The viewport shows a window of OMT tiles centered on this position.
/// Arrow keys pan; `<`/`>` change z-level.
#[derive(Resource, Debug, Clone)]
pub struct OvermapCamera {
    /// World-absolute OMT x coordinate of viewport center.
    pub center_x: i32,
    /// World-absolute OMT y coordinate of viewport center.
    pub center_y: i32,
    /// Z-level being viewed.
    pub z: i32,
    /// Half-width of the viewport in OMT tiles.
    pub half_width: i32,
    /// Half-height of the viewport in OMT tiles.
    pub half_height: i32,
}

impl Default for OvermapCamera {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_y: 0,
            z: 0,
            half_width: 30,
            half_height: 20,
        }
    }
}

impl OvermapCamera {
    /// Top-left corner of the viewport in world-absolute OMT coords.
    pub fn top_left(&self) -> (i32, i32) {
        (
            self.center_x - self.half_width,
            self.center_y - self.half_height,
        )
    }

    /// Bottom-right corner of the viewport (inclusive).
    pub fn bottom_right(&self) -> (i32, i32) {
        (
            self.center_x + self.half_width,
            self.center_y + self.half_height,
        )
    }

    /// Which overmap contains the center point.
    pub fn center_overmap(&self) -> OmPos {
        OmPos::new(
            self.center_x.div_euclid(180),
            self.center_y.div_euclid(180),
            ZLevel::new(self.z as i8),
        )
    }

    /// Pan the camera by a delta in OMT units.
    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.center_x = self.center_x.saturating_add(dx);
        self.center_y = self.center_y.saturating_add(dy);
    }

    /// Change z-level, clamping to the valid range.
    pub fn set_z(&mut self, z: i32) {
        self.z = z.clamp(-10, 10);
    }

    /// Move cursor to a specific OMT position.
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.center_x = x;
        self.center_y = y;
    }
}
