//! # cdda_input – Bevy input plugin
//!
//! A decoupled input architecture for Cataclysm-DDA BR:
//!
//! 1. **`handle_raw_input`** (PreUpdate) handles rebind capture and text input,
//!    emitting semantic `InputAction` messages for those special cases.
//! 2. **`bridge_actionstate`** (Update) reads leafwing `ActionState<BindableAction>`
//!    and converts just-pressed actions into `InputAction` messages.
//!
//! Downstream systems consume `InputAction` messages and never see raw keys.

pub mod actions;
pub mod bindings;
pub mod context;
pub mod crafting;
pub mod systems;

// ----- Re-exports ---------------------------------------------------------

pub use actions::{ActionSource, BindableAction, Direction, GameAction, InputAction};
pub use bindings::{default_bindings, ActiveKeybindings, ContextInputMaps};
pub use context::{InputContextId, InputContextStack};
pub use systems::{
    bridge_actionstate, clear_rebind_flag, handle_raw_input, refresh_active_keybindings,
    sync_leafwing_input_map, GlobalInputEntity, RebindCapture, RebindCaptureInner,
};

// ----- Plugin -------------------------------------------------------------

use bevy_app::{App, Plugin, PreUpdate, Startup, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use leafwing_input_manager::prelude::InputManagerPlugin;

/// Registers input resources, events, and systems.
pub struct CddaInputPlugin;

impl Plugin for CddaInputPlugin {
    fn build(&self, app: &mut App) {
        // leafwing: processes ActionState each PreUpdate
        app.add_plugins(InputManagerPlugin::<BindableAction>::default());

        // Resources
        app.insert_resource(InputContextStack::new());
        app.insert_resource(default_bindings());
        app.init_resource::<RebindCapture>();

        // Messages — InputAction is the core decoupling point
        app.add_message::<InputAction>();

        // Startup: spawn the global input entity with the initial InputMap
        app.add_systems(Startup, spawn_global_input_entity);

        // PreUpdate: handle text input and rebind capture before leafwing
        app.add_systems(PreUpdate, handle_raw_input);

        // Update: bridge ActionState → InputAction messages, then clean up
        app.add_systems(Update, (bridge_actionstate, clear_rebind_flag).chain());

        // Update: keep InputMap in sync when context changes
        app.add_systems(Update, sync_leafwing_input_map);

        // Update: rebuild ActiveKeybindings resource for UI hints
        app.init_resource::<ActiveKeybindings>();
        app.add_systems(
            Update,
            refresh_active_keybindings.after(sync_leafwing_input_map),
        );
    }
}

fn spawn_global_input_entity(
    mut commands: bevy_ecs::prelude::Commands,
    context_maps: bevy_ecs::prelude::Res<ContextInputMaps>,
) {
    let initial_map = context_maps.merged_for(&InputContextId::MainMenu);
    commands.spawn((GlobalInputEntity, initial_map));
}
