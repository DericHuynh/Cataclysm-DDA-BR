//! # cdda_input – Bevy input plugin
//!
//! A decoupled input architecture for Cataclysm-DDA BR:
//!
//! 1. **`handle_raw_input`** reads `KeyboardInput` messages, resolves keys
//!    against the active input context, and fires semantic `InputAction` messages.
//!
//! Systems **never** see raw keys — they consume `InputAction` messages.
//!
//! Focus navigation has moved to the `cdda_screen` crate.

pub mod actions;
pub mod bindings;
pub mod context;
pub mod systems;

// ----- Re-exports ---------------------------------------------------------

pub use actions::{ActionSource, Direction, GameAction, InputAction};
pub use bindings::{default_bindings, ContextBindings, KeyChord};
pub use context::{InputContextId, InputContextStack};
pub use systems::{handle_raw_input, RebindCapture, RebindCaptureInner};

// ----- Plugin -------------------------------------------------------------

use bevy_app::{App, Plugin, PreUpdate};

/// Registers input resources, events, and systems.
///
/// Add this plugin to your `App` to enable the input pipeline:
///
/// ```ignore
/// app.add_plugins(CddaInputPlugin);
/// ```
pub struct CddaInputPlugin;

impl Plugin for CddaInputPlugin {
    fn build(&self, app: &mut App) {
        // Resources
        app.insert_resource(crate::input::InputContextStack::new());
        app.insert_resource(crate::input::default_bindings());
        app.init_resource::<crate::input::RebindCapture>();

        // Messages — InputAction is the core decoupling point
        app.add_message::<crate::input::InputAction>();

        // Systems — raw input runs in PreUpdate so InputAction messages are
        // available to handle_navigation_input (also PreUpdate) in the same frame.
        // Bevy processes PreUpdate systems in registration order within the same set,
        // so CddaInputPlugin must be added BEFORE ScreenNavigationPlugin.
        app.add_systems(PreUpdate, crate::input::handle_raw_input);
    }
}
