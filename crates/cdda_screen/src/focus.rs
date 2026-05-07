//! Keyboard focus for UI panel navigation.
//!
//! Entities with `KeyboardFocusable` can receive keyboard-driven navigation
//! via `InputAction` messages (NavigateUp/Down, Confirm, Cancel, etc.).
//!
//! The `InputFocus` resource tracks which entity currently has focus.
//!
//! ## Design note
//!
//! We use a hand-rolled `InputFocus` resource (not `bevy_input_focus::InputFocus`)
//! to keep the `cdda_screen` crate fully headless — `bevy_input_focus` may pull in
//! `bevy_window` or `bevy_render` as transitive dependencies, which would break
//! headless tests.

use bevy_ecs::prelude::{Component, Entity, Resource};

use cdda_input::GameAction;

// ---------------------------------------------------------------------------
// KeyboardFocusable
// ---------------------------------------------------------------------------

/// Marker + ordering component for entities that participate in keyboard focus
/// navigation.
///
/// The `focus_order` field determines the traversal order when the player
/// presses NavigateUp / NavigateDown.  Entities are sorted by this value.
///
/// When the entity receives a Confirm action, `confirm_action` (if set) is
/// sent as a new `InputAction` message so that game logic can react.
#[derive(Component, Debug, Clone)]
pub struct KeyboardFocusable {
    /// Order in the focus traversal sequence (lower = earlier).
    pub focus_order: u32,
    /// Optional action to emit when this entity is confirmed.
    pub confirm_action: Option<GameAction>,
}

// ---------------------------------------------------------------------------
// InputFocus
// ---------------------------------------------------------------------------

/// Tracks which entity (if any) currently has keyboard focus.
#[derive(Resource, Debug, Clone)]
pub struct InputFocus {
    pub entity: Option<Entity>,
}

impl Default for InputFocus {
    fn default() -> Self {
        Self { entity: None }
    }
}
