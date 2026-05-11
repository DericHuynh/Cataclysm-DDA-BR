//! # cdda_context — Headless context state machine
//!
//! Owns the context state machine (`Ctx`, `ContextStack`, `ctx_def`),
//! input handling (`handle_navigation_input`), and focus.
//! Has **no dependency** on `bevy_render`, `bevy_sprite`, or `bevy_window`.
//!
//! `cdda_render` reads the same `Ctx` state and registers
//! `OnEnter`/`OnExit` systems to spawn/despawn visual UI.

pub mod actions;
pub mod config;
pub mod ctx;
pub mod cursor;
pub mod focus;
pub mod menu;
pub mod nav;
pub mod overlay;
pub mod screen;
pub mod systems;

// ----- Re-exports ---------------------------------------------------------

pub use actions::{ContextAction, ContextActions};
pub use config::{CharacterCreationState, GameSettings, WorldCreationSettings};
pub use ctx::{ContextStack, Ctx};
pub use cursor::ExamineCursor;
pub use focus::{InputFocus, KeyboardFocusable};
pub use menu::{MenuItem, MenuList, SelectedIndex};
pub use nav::{
    ctx_def, handle_navigation_input, handle_panel_openers, pop_ctx, push_ctx, sync_input_context,
    FocusedCommandIndex, GameEvent, ScreenCommand, ScreenDefinition, ScreenListItem,
    TransitionTarget,
};
pub use overlay::{
    cleanup_activity_overlay, is_input_blocked, sync_activity_overlay, Overlay, OverlayStack,
};
pub use screen::{CddaScreen, Screen};
pub use systems::{ctx_and_cursor, menu_navigation};

// ----- Plugin -------------------------------------------------------------

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_state::app::AppExtStates;

/// Registers context navigation resources, events, systems.
pub struct ContextPlugin;

impl Plugin for ContextPlugin {
    fn build(&self, app: &mut App) {
        // Bevy States — drives OnEnter/OnExit scheduling
        app.init_state::<Ctx>();

        // Resources
        app.insert_resource(ContextActions::default());
        app.insert_resource(OverlayStack::default());
        app.insert_resource(ContextStack::default());
        app.insert_resource(FocusedCommandIndex::default());
        app.insert_resource(InputFocus::default());
        app.insert_resource(crate::context::cursor::ExamineCursor::default());
        app.insert_resource(crate::context::config::GameSettings::default());
        app.insert_resource(crate::context::config::CharacterCreationState::default());
        app.insert_resource(crate::context::config::WorldCreationSettings::default());

        // Core navigation — processes InputAction messages and dispatches transitions.
        // Runs in Update so it sees messages written by bridge_actionstate (also Update).
        app.add_systems(
            Update,
            (handle_navigation_input, handle_panel_openers)
                .after(crate::input::systems::bridge_actionstate),
        );

        // Sync input context with current context
        app.add_systems(Update, crate::context::nav::sync_input_context);

        // Menu scroll and cursor movement
        app.add_systems(Update, (menu_navigation, ctx_and_cursor));
    }
}
