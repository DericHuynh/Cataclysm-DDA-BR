//! World cursor — tracks which tile the player is examining or targeting.
//!
//! The `ExamineCursor` resource is the *logical* cursor position.
//! `cdda_render` reads this resource to draw a highlight glyph on the
//! appropriate tile.  Nothing in `cdda_screen` or `cdda_render` should store
//! a second copy of the cursor position — this is the single source of truth.

use bevy_ecs::prelude::Resource;

/// The world-tile position currently under the examine/look cursor.
///
/// `None` means no cursor is active (default gameplay mode).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct ExamineCursor {
    pub tile: Option<cdda_core::coords::WorldPos>,
}
