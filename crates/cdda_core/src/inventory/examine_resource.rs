//! Resource storing the item entity being examined in the detail overlay.
//!
//! Set by `inventory_screen_input` when the player presses the Examine key
//! on a focused inventory item. Read by `spawn_examine_overlay` in the
//! render crate to populate the detail UI.

use bevy_ecs::prelude::*;

/// The entity of the item currently being examined.
///
/// `None` when no examine overlay is active.  Set to `Some(entity)` just
/// before pushing `Screen::ItemExamine`; reset on pop.
#[derive(Resource, Debug, Clone, Default)]
pub struct ExaminedItem(pub Option<Entity>);
