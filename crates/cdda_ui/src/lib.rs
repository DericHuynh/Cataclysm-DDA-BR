//! # cdda_ui — Headless UI state machine
//!
//! Owns the screen state machine (`Screen`, `ScreenStack`, `screen_def`),
//! input handling (`handle_navigation_input`), and focus.
//! Has **no dependency** on `bevy_render`, `bevy_sprite`, or `bevy_window`.
//!
//! Render crates (`cdda_render`) read the same `Screen` state and
//! register `OnEnter`/`OnExit` systems to spawn/despawn visual UI.

pub mod config;
pub mod cursor;
pub mod focus;
pub mod menu;
pub mod screen;
pub mod screen_nav;
pub mod systems;

// ----- Re-exports ---------------------------------------------------------

pub use config::{CharacterCreationState, GameSettings, WorldCreationSettings};
pub use cursor::ExamineCursor;
pub use focus::{InputFocus, KeyboardFocusable};
pub use menu::{MenuItem, MenuList, SelectedIndex};
pub use screen::{Screen, ScreenStack};
pub use screen_nav::{
    handle_navigation_input, pop_screen, push_screen, screen_def, sync_input_context,
    FocusedCommandIndex, GameEvent, ScreenCommand, ScreenDefinition, ScreenListItem,
    TransitionTarget,
};
pub use systems::{menu_navigation, screen_and_cursor};

// ----- Plugin -------------------------------------------------------------

use bevy_app::{App, Plugin, PreUpdate, Update};
use bevy_state::app::AppExtStates;

/// Registers screen navigation resources, events, systems.
pub struct ScreenNavigationPlugin;

impl Plugin for ScreenNavigationPlugin {
    fn build(&self, app: &mut App) {
        // Bevy States — drives OnEnter/OnExit scheduling
        app.init_state::<Screen>();

        // Resources
        app.insert_resource(ScreenStack::default());
        app.insert_resource(FocusedCommandIndex::default());
        app.insert_resource(crate::InputFocus::default());
        app.insert_resource(crate::ExamineCursor::default());
        app.insert_resource(crate::config::GameSettings::default());
        app.insert_resource(crate::config::CharacterCreationState::default());
        app.insert_resource(crate::config::WorldCreationSettings::default());

        // Events (messages)
        app.add_message::<GameEvent>();

        // Core navigation — processes InputAction messages and dispatches transitions
        app.add_systems(PreUpdate, handle_navigation_input);

        // Sync input context with current screen
        app.add_systems(Update, crate::screen_nav::sync_input_context);

        // Menu scroll and cursor movement
        app.add_systems(Update, (menu_navigation, screen_and_cursor));
    }
}
