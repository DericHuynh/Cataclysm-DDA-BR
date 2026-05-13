//! Keybinding storage using leafwing `InputMap<BindableAction>`.
//!
//! `ContextInputMaps` holds one `InputMap<BindableAction>` per input context
//! plus a global map.  The active context's merged map is written onto the
//! `GlobalInputEntity` each time the screen state changes.

use std::collections::HashMap;

use bevy_ecs::prelude::Resource;
use bevy_input::keyboard::KeyCode;
use leafwing_input_manager::prelude::{ButtonlikeChord, InputMap, ModifierKey, UserInputWrapper};
use leafwing_input_manager::user_input::Buttonlike;

use crate::actions::BindableAction;
use cdda_components::input::InputContextId;

// ---------------------------------------------------------------------------
// ContextInputMaps
// ---------------------------------------------------------------------------

/// All per-context `InputMap<BindableAction>` tables plus a global map.
#[derive(Resource, Debug, Clone)]
pub struct ContextInputMaps {
    pub contexts: HashMap<InputContextId, InputMap<BindableAction>>,
    pub global: InputMap<BindableAction>,
}

impl ContextInputMaps {
    /// Borrow the `InputMap` for a context.
    pub fn get(&self, ctx: &InputContextId) -> Option<&InputMap<BindableAction>> {
        self.contexts.get(ctx)
    }

    /// Return a merged map: global + context-specific bindings.
    pub fn merged_for(&self, ctx: &InputContextId) -> InputMap<BindableAction> {
        let mut merged = self.global.clone();
        if let Some(ctx_map) = self.contexts.get(ctx) {
            for (action, buttons) in ctx_map.iter_buttonlike() {
                for button in buttons {
                    merged.insert_boxed(action.clone(), button.clone());
                }
            }
        }
        merged
    }

    /// Replace the binding for `action` in `ctx` with `input`.
    pub fn rebind(
        &mut self,
        ctx: InputContextId,
        action: BindableAction,
        input: Box<dyn Buttonlike>,
    ) {
        let map = self.contexts.entry(ctx).or_default();
        map.clear_action(&action);
        map.insert_boxed(action, input);
    }

    /// List (action, key-display-string) pairs for a context, one per action.
    /// Used by the settings keybinding tab.
    pub fn list_bindings(&self, ctx: &InputContextId) -> Vec<(BindableAction, String)> {
        let Some(map) = self.contexts.get(ctx) else {
            return vec![];
        };
        let mut rows = Vec::new();
        for action in BindableAction::all() {
            if let Some(inputs) = map.get(&action) {
                if !inputs.is_empty() {
                    rows.push((action, format_wrapper(&inputs[0])));
                }
            }
        }
        rows.sort_by(|a, b| a.0.label().cmp(b.0.label()));
        rows
    }
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

/// Format a `UserInputWrapper` as a human-readable key label.
pub fn format_wrapper(wrapper: &UserInputWrapper) -> String {
    // Use Debug and clean up the output to something short and readable.
    let raw = format!("{wrapper:?}");
    let s = raw
        .replace("Button(ButtonlikeChord([KeyCode(", "")
        .replace("Button(KeyCode(", "")
        .replace("Button(", "");
    // Strip trailing parens, brackets, and "Key" prefix.
    let key = s
        .replace("])", "")
        .replace("))", "")
        .replace(')', "")
        .replace("Key", "")
        .trim()
        .to_string();
    // Map common key names to compact display forms.
    match key.as_str() {
        "ArrowUp" => "\u{2191}".to_string(),
        "ArrowDown" => "\u{2193}".to_string(),
        "ArrowLeft" => "\u{2190}".to_string(),
        "ArrowRight" => "\u{2192}".to_string(),
        "Escape" => "Esc".to_string(),
        "Backspace" => "Bksp".to_string(),
        "PageUp" => "PgUp".to_string(),
        "PageDown" => "PgDn".to_string(),
        "Slash" => "/".to_string(),
        "Space" => "Spc".to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// ActiveKeybindings — live key display strings for the current context
// ---------------------------------------------------------------------------

/// Maps each `BindableAction` to its currently-bound key display string
/// for the active input context. Updated whenever the context or bindings
/// change. UI systems query this to render dynamic key hints instead of
/// hardcoding `[w]` / `[d]` etc.
#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveKeybindings {
    /// `BindableAction` → human-readable key string (e.g. `"W"`, `"D"`, `"Esc"`).
    pub keys: HashMap<BindableAction, String>,
}

impl ActiveKeybindings {
    /// Get the display key for an action, or `"?"` if unbound.
    pub fn key_for(&self, action: BindableAction) -> &str {
        self.keys.get(&action).map(|s| s.as_str()).unwrap_or("?")
    }
}

// ---------------------------------------------------------------------------
// Default bindings
// ---------------------------------------------------------------------------

pub fn default_bindings() -> ContextInputMaps {
    let mut maps = ContextInputMaps {
        contexts: HashMap::new(),
        global: InputMap::default(),
    };

    // -- Global hotkeys -----------------------------------------------------
    maps.global.insert(BindableAction::Quicksave, KeyCode::F5);
    maps.global.insert(BindableAction::Quickload, KeyCode::F9);
    maps.global.insert(BindableAction::ToggleDebug, KeyCode::F3);
    maps.global.insert(BindableAction::Help, KeyCode::F1);
    maps.global.insert(BindableAction::Help, KeyCode::Slash);

    // -- Gameplay -----------------------------------------------------------
    let mut gameplay = InputMap::new([
        (BindableAction::MoveNorth, KeyCode::KeyK),
        (BindableAction::MoveNorth, KeyCode::ArrowUp),
        (BindableAction::MoveNorth, KeyCode::Numpad8),
        (BindableAction::MoveSouth, KeyCode::KeyJ),
        (BindableAction::MoveSouth, KeyCode::ArrowDown),
        (BindableAction::MoveSouth, KeyCode::Numpad2),
        (BindableAction::MoveEast, KeyCode::KeyL),
        (BindableAction::MoveEast, KeyCode::ArrowRight),
        (BindableAction::MoveEast, KeyCode::Numpad6),
        (BindableAction::MoveWest, KeyCode::KeyH),
        (BindableAction::MoveWest, KeyCode::ArrowLeft),
        (BindableAction::MoveWest, KeyCode::Numpad4),
        (BindableAction::MoveNorthWest, KeyCode::KeyY),
        (BindableAction::MoveNorthWest, KeyCode::Numpad7),
        (BindableAction::MoveNorthEast, KeyCode::KeyU),
        (BindableAction::MoveNorthEast, KeyCode::Numpad9),
        (BindableAction::MoveSouthWest, KeyCode::KeyB),
        (BindableAction::MoveSouthWest, KeyCode::Numpad1),
        (BindableAction::MoveSouthEast, KeyCode::KeyN),
        (BindableAction::MoveSouthEast, KeyCode::Numpad3),
        (BindableAction::Wait, KeyCode::Period),
        (BindableAction::Wait, KeyCode::Numpad5),
        (BindableAction::Examine, KeyCode::KeyE),
        (BindableAction::Pickup, KeyCode::KeyG),
        (BindableAction::Drop, KeyCode::KeyD),
        (BindableAction::Reload, KeyCode::KeyR),
        (BindableAction::Fire, KeyCode::KeyF),
        (BindableAction::Throw, KeyCode::KeyT),
        (BindableAction::Custom1, KeyCode::F2), // debug spawn panel
        (BindableAction::OpenInventory, KeyCode::KeyI),
        (BindableAction::OpenCrafting, KeyCode::KeyC),
        (BindableAction::OpenCharacterSheet, KeyCode::KeyP),
        (BindableAction::OpenMap, KeyCode::KeyM),
    ]);
    // Shift+C also opens crafting
    gameplay.insert(
        BindableAction::OpenCrafting,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyC),
    );
    maps.contexts.insert(InputContextId::Gameplay, gameplay);

// -- Overmap ------------------------------------------------------------
    // Arrow keys pan the camera.  Shift pans 5 tiles.
    // < > change z-level.  Escape returns to gameplay.
    let mut overmap = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateUp, KeyCode::KeyK),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::NavigateDown, KeyCode::KeyJ),
        (BindableAction::NavigateLeft, KeyCode::ArrowLeft),
        (BindableAction::NavigateLeft, KeyCode::KeyH),
        (BindableAction::NavigateRight, KeyCode::ArrowRight),
        (BindableAction::NavigateRight, KeyCode::KeyL),
        (BindableAction::Custom1, KeyCode::Comma),
        (BindableAction::Custom2, KeyCode::Period),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyQ),
    ]);
    overmap.insert(
        BindableAction::NavigateRight,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyL),
    );
    overmap.insert(
        BindableAction::NavigateLeft,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyH),
    );
    overmap.insert(
        BindableAction::NavigateDown,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyJ),
    );
    overmap.insert(
        BindableAction::NavigateUp,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyK),
    );
    maps.contexts.insert(InputContextId::Overmap, overmap);

    // -- MainMenu -----------------------------------------------------------
    let main_menu = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::NavigateLeft, KeyCode::ArrowLeft),
        (BindableAction::NavigateRight, KeyCode::ArrowRight),
        (BindableAction::NavigatePageUp, KeyCode::PageUp),
        (BindableAction::NavigatePageDown, KeyCode::PageDown),
        (BindableAction::NavigateHome, KeyCode::Home),
        (BindableAction::NavigateEnd, KeyCode::End),
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::HotkeyM, KeyCode::KeyM),
        (BindableAction::HotkeyN, KeyCode::KeyN),
        (BindableAction::HotkeyL, KeyCode::KeyL),
        (BindableAction::HotkeyW, KeyCode::KeyW),
        (BindableAction::HotkeyS, KeyCode::KeyS),
        (BindableAction::HotkeyT, KeyCode::KeyT),
        (BindableAction::HotkeyH, KeyCode::KeyH),
        (BindableAction::HotkeyC, KeyCode::KeyC),
        (BindableAction::HotkeyB, KeyCode::KeyB),
        (BindableAction::HotkeyQ, KeyCode::KeyQ),
    ]);
    maps.contexts.insert(InputContextId::MainMenu, main_menu);

    // -- Inventory ----------------------------------------------------------
    let inventory = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateUp, KeyCode::KeyK),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::NavigateDown, KeyCode::KeyJ),
        (BindableAction::NavigatePrevTab, KeyCode::ArrowLeft),
        (BindableAction::NavigatePrevTab, KeyCode::KeyH),
        (BindableAction::NavigateNextTab, KeyCode::ArrowRight),
        (BindableAction::NavigateNextTab, KeyCode::KeyL),
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Confirm, KeyCode::KeyE),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyQ),
        (BindableAction::Filter, KeyCode::Slash),
        (BindableAction::NavigatePageUp, KeyCode::PageUp),
        (BindableAction::NavigatePageDown, KeyCode::PageDown),
        (BindableAction::NavigateHome, KeyCode::Home),
        (BindableAction::NavigateEnd, KeyCode::End),
        (BindableAction::UseItem, KeyCode::KeyW),
        (BindableAction::Examine, KeyCode::KeyX),
        (BindableAction::Drop, KeyCode::KeyD),
        (BindableAction::HotkeyR, KeyCode::KeyR),
    ]);
    maps.contexts.insert(InputContextId::Inventory, inventory);

    // -- CraftingMenu -------------------------------------------------------
    let mut crafting = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateUp, KeyCode::KeyK),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::NavigateDown, KeyCode::KeyJ),
        (BindableAction::NavigateLeft, KeyCode::ArrowLeft),
        (BindableAction::NavigateLeft, KeyCode::KeyH),
        (BindableAction::NavigateRight, KeyCode::ArrowRight),
        (BindableAction::NavigateRight, KeyCode::KeyL),
        (BindableAction::NavigateNextTab, KeyCode::Tab),
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyQ),
        (BindableAction::Filter, KeyCode::Slash),
        (BindableAction::HotkeyA, KeyCode::KeyA),
        (BindableAction::NavigatePageUp, KeyCode::PageUp),
        (BindableAction::NavigatePageDown, KeyCode::PageDown),
    ]);
    crafting.insert(
        BindableAction::NavigatePrevTab,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::Tab),
    );
    maps.contexts.insert(InputContextId::CraftingMenu, crafting);

    // -- CharacterSheet -----------------------------------------------------
    let mut char_sheet = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateUp, KeyCode::KeyK),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::NavigateDown, KeyCode::KeyJ),
        (BindableAction::NavigateNextTab, KeyCode::Tab),
        (BindableAction::NavigatePageUp, KeyCode::PageUp),
        (BindableAction::NavigatePageDown, KeyCode::PageDown),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyQ),
    ]);
    char_sheet.insert(
        BindableAction::NavigatePrevTab,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::Tab),
    );
    maps.contexts
        .insert(InputContextId::CharacterSheet, char_sheet);

    // -- Dialog -------------------------------------------------------------
    let dialog = InputMap::new([
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Confirm, KeyCode::KeyY),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyN),
        (BindableAction::Cancel, KeyCode::KeyQ),
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
    ]);
    maps.contexts.insert(InputContextId::Dialog, dialog);

    // -- ExamineLook --------------------------------------------------------
    let examine = InputMap::new([
        (BindableAction::MoveNorth, KeyCode::KeyK),
        (BindableAction::MoveNorth, KeyCode::ArrowUp),
        (BindableAction::MoveSouth, KeyCode::KeyJ),
        (BindableAction::MoveSouth, KeyCode::ArrowDown),
        (BindableAction::MoveEast, KeyCode::KeyL),
        (BindableAction::MoveEast, KeyCode::ArrowRight),
        (BindableAction::MoveWest, KeyCode::KeyH),
        (BindableAction::MoveWest, KeyCode::ArrowLeft),
        (BindableAction::MoveNorthWest, KeyCode::KeyY),
        (BindableAction::MoveNorthEast, KeyCode::KeyU),
        (BindableAction::MoveSouthWest, KeyCode::KeyB),
        (BindableAction::MoveSouthEast, KeyCode::KeyN),
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyQ),
    ]);
    maps.contexts.insert(InputContextId::ExamineLook, examine);

    // -- DirectionSelect ----------------------------------------------------
    let dir_select = InputMap::new([
        (BindableAction::MoveNorth, KeyCode::KeyK),
        (BindableAction::MoveNorth, KeyCode::ArrowUp),
        (BindableAction::MoveSouth, KeyCode::KeyJ),
        (BindableAction::MoveSouth, KeyCode::ArrowDown),
        (BindableAction::MoveEast, KeyCode::KeyL),
        (BindableAction::MoveEast, KeyCode::ArrowRight),
        (BindableAction::MoveWest, KeyCode::KeyH),
        (BindableAction::MoveWest, KeyCode::ArrowLeft),
        (BindableAction::MoveNorthWest, KeyCode::KeyY),
        (BindableAction::MoveNorthEast, KeyCode::KeyU),
        (BindableAction::MoveSouthWest, KeyCode::KeyB),
        (BindableAction::MoveSouthEast, KeyCode::KeyN),
        (BindableAction::Cancel, KeyCode::Escape),
    ]);
    maps.contexts
        .insert(InputContextId::DirectionSelect, dir_select);

    // -- TextInput ----------------------------------------------------------
    let text_input = InputMap::new([
        (BindableAction::TextCommit, KeyCode::Enter),
        (BindableAction::TextCancel, KeyCode::Escape),
        (BindableAction::TextBackspace, KeyCode::Backspace),
        (BindableAction::TextDelete, KeyCode::Delete),
        (BindableAction::TextNavLeft, KeyCode::ArrowLeft),
        (BindableAction::TextNavRight, KeyCode::ArrowRight),
        (BindableAction::TextNavHome, KeyCode::Home),
        (BindableAction::TextNavEnd, KeyCode::End),
    ]);
    maps.contexts.insert(InputContextId::TextInput, text_input);

    // -- Settings -----------------------------------------------------------
    let mut settings = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::NavigateLeft, KeyCode::ArrowLeft),
        (BindableAction::NavigateRight, KeyCode::ArrowRight),
        (BindableAction::NavigateNextTab, KeyCode::Tab),
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Custom0, KeyCode::KeyR), // reset to defaults
    ]);
    settings.insert(
        BindableAction::NavigatePrevTab,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::Tab),
    );
    maps.contexts.insert(InputContextId::Settings, settings);

    // -- PauseMenu ----------------------------------------------------------
    let pause = InputMap::new([
        (BindableAction::NavigateUp, KeyCode::ArrowUp),
        (BindableAction::NavigateDown, KeyCode::ArrowDown),
        (BindableAction::Confirm, KeyCode::Enter),
        (BindableAction::Cancel, KeyCode::Escape),
    ]);
    maps.contexts.insert(InputContextId::PauseMenu, pause);

    // -- QuantityInput ------------------------------------------------------
    let qty = InputMap::new([
        (BindableAction::TextCommit, KeyCode::Enter),
        (BindableAction::TextCancel, KeyCode::Escape),
        (BindableAction::TextBackspace, KeyCode::Backspace),
    ]);
    maps.contexts.insert(InputContextId::QuantityInput, qty);

    // -- VehicleInteraction -------------------------------------------------
    let vehicle = InputMap::new([
        (BindableAction::MoveNorth, KeyCode::KeyK),
        (BindableAction::MoveNorth, KeyCode::ArrowUp),
        (BindableAction::MoveSouth, KeyCode::KeyJ),
        (BindableAction::MoveSouth, KeyCode::ArrowDown),
        (BindableAction::MoveEast, KeyCode::KeyL),
        (BindableAction::MoveEast, KeyCode::ArrowRight),
        (BindableAction::MoveWest, KeyCode::KeyH),
        (BindableAction::MoveWest, KeyCode::ArrowLeft),
        (BindableAction::Examine, KeyCode::KeyE),
        (BindableAction::Cancel, KeyCode::Escape),
        (BindableAction::Cancel, KeyCode::KeyQ),
        (BindableAction::Confirm, KeyCode::Enter),
    ]);
    maps.contexts
        .insert(InputContextId::VehicleInteraction, vehicle);

    maps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_bindings_exist() {
        let maps = default_bindings();
        let gameplay = maps
            .get(&InputContextId::Gameplay)
            .expect("gameplay bindings");
        assert!(
            gameplay
                .get(&BindableAction::MoveNorth)
                .is_some_and(|v| !v.is_empty()),
            "MoveNorth should be bound in gameplay"
        );
    }

    #[test]
    fn global_hotkeys_present() {
        let maps = default_bindings();
        assert!(
            maps.global
                .get(&BindableAction::Quicksave)
                .is_some_and(|v| !v.is_empty()),
            "Quicksave should be globally bound"
        );
    }

    #[test]
    fn context_maps_have_distinct_entries() {
        let maps = default_bindings();
        assert!(maps.get(&InputContextId::Gameplay).is_some());
        assert!(maps.get(&InputContextId::Inventory).is_some());
    }

    #[test]
    fn merged_includes_global() {
        let maps = default_bindings();
        let merged = maps.merged_for(&InputContextId::Gameplay);
        assert!(
            merged
                .get(&BindableAction::Quicksave)
                .is_some_and(|v| !v.is_empty()),
            "merged map should include global quicksave"
        );
    }
}
