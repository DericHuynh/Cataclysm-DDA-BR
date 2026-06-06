//! Overmap camera — viewport center and z-level for the overmap viewer.
//!
//! Tracks the center OMT tile of the current viewport. All coordinates are
//! world-absolute OMT units. To convert to map tiles or submap coordinates:
//!
//! ```text
//! submap_x = center_x * 2           (each OMT = 2 submaps wide)
//! tile_x   = center_x * 24          (each OMT = 24 map tiles wide)
//! ```

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::{OmPos, ZLevel};

/// Camera for the overmap viewer, tracking the center OMT tile.
///
/// The viewport shows a window of OMT tiles centered on `(center_x, center_y)`.
/// Arrow keys pan; `<`/`>` change z-level.
#[derive(Resource, Debug, Clone)]
pub struct OvermapCamera {
    /// World-absolute OMT x coordinate of the viewport center.
    pub center_x: i32,
    /// World-absolute OMT y coordinate of the viewport center.
    pub center_y: i32,
    /// Z-level being viewed (clamped to -10..=10).
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
    #[inline]
    pub fn top_left(&self) -> (i32, i32) {
        (self.center_x - self.half_width, self.center_y - self.half_height)
    }

    /// Bottom-right corner of the viewport (inclusive).
    #[inline]
    pub fn bottom_right(&self) -> (i32, i32) {
        (self.center_x + self.half_width, self.center_y + self.half_height)
    }

    /// Which overmap contains the center OMT tile.
    #[inline]
    pub fn center_overmap(&self) -> OmPos {
        OmPos::new(
            self.center_x.div_euclid(180),
            self.center_y.div_euclid(180),
            ZLevel::new(self.z as i8),
        )
    }

    /// Pan the camera by `(dx, dy)` OMT units.
    #[inline]
    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.center_x = self.center_x.saturating_add(dx);
        self.center_y = self.center_y.saturating_add(dy);
    }

    /// Set z-level, clamping to the valid range (-10..=10).
    #[inline]
    pub fn set_z(&mut self, z: i32) {
        self.z = z.clamp(-10, 10);
    }

    /// Move the viewport center to a specific OMT position.
    #[inline]
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.center_x = x;
        self.center_y = y;
    }

    /// Center world-submap coordinate of the viewport.
    #[inline]
    pub fn center_submap(&self) -> (i32, i32) {
        (self.center_x * 2, self.center_y * 2)
    }
}
