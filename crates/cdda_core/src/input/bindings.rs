//! Keybinding storage and resolution.
//!
//! `ContextBindings` maps key chords to `GameAction`s, organized by input
//! context.  Global bindings take priority and work in any context.

use std::collections::HashMap;

use bevy_ecs::prelude::Resource;
use bevy_input::keyboard::KeyCode;

use crate::input::actions::{Direction, GameAction};
use crate::input::context::InputContextId;

// ---------------------------------------------------------------------------
// KeyChord
// ---------------------------------------------------------------------------

/// A physical key plus optional modifier keys.
///
/// Modifiers are stored as booleans so that `KeyChord` works as a hash-map
/// key without needing to match against specific left/right modifier keycodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub key: KeyCode,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyChord {
    /// Build a chord from a bare key (no modifiers).
    pub fn new(key: KeyCode) -> Self {
        Self {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    /// Builder-style: set the shift modifier.
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Builder-style: set the ctrl modifier.
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// Builder-style: set the alt modifier.
    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

// ---------------------------------------------------------------------------
// ContextBindings
// ---------------------------------------------------------------------------

/// All keybindings for every input context, plus a set of global hotkeys.
///
/// ```ignore
/// let bindings: Res<ContextBindings>;
/// if let Some(action) = bindings.resolve(&context_stack.top(), &key, shift, ctrl, alt) {
///     // fire InputAction { action, source: Keyboard }
/// }
/// ```
#[derive(Resource, Debug, Clone)]
pub struct ContextBindings {
    /// Per-context bindings (only checked when that context is top-of-stack).
    pub contexts: HashMap<InputContextId, HashMap<KeyChord, GameAction>>,
    /// Global hotkeys that work in **any** context.
    pub global: HashMap<KeyChord, GameAction>,
}

impl ContextBindings {
    /// Resolve a key press to a `GameAction`.
    ///
    /// 1. Global bindings are checked first (win over context).
    /// 2. If no global match, the active context's bindings are checked.
    ///
    /// Modifier state (`shift`, `ctrl`, `alt`) is computed from
    /// `ButtonInput<KeyCode>` by the caller (the `handle_raw_input`
    /// system) rather than passed through `KeyboardInput.modifiers`,
    /// because Bevy 0.18.1 does not expose `modifiers` on `KeyboardInput`.
    ///
    /// Returns `None` if neither global nor context bindings contain the chord.
    pub fn resolve(
        &self,
        context: &InputContextId,
        key: KeyCode,
        shift: bool,
        ctrl: bool,
        alt: bool,
    ) -> Option<GameAction> {
        let chord = KeyChord {
            key,
            shift,
            ctrl,
            alt,
        };

        // Global bindings take priority.
        if let Some(action) = self.global.get(&chord) {
            return Some(action.clone());
        }

        // Context-specific bindings.
        if let Some(context_map) = self.contexts.get(context) {
            if let Some(action) = context_map.get(&chord) {
                return Some(action.clone());
            }
        }

        None
    }

    /// Insert a binding into a specific context.
    pub fn bind(&mut self, context: InputContextId, chord: KeyChord, action: GameAction) {
        self.contexts
            .entry(context)
            .or_default()
            .insert(chord, action);
    }

    /// Insert a global binding.
    pub fn bind_global(&mut self, chord: KeyChord, action: GameAction) {
        self.global.insert(chord, action);
    }

    /// Remove **all** bindings that map to `action` within `context`.
    /// Returns the removed chords.
    pub fn remove_binding(
        &mut self,
        context: &InputContextId,
        action: &GameAction,
    ) -> Vec<KeyChord> {
        let mut removed = Vec::new();
        let Some(ctx) = self.contexts.get_mut(context) else {
            return removed;
        };
        ctx.retain(|chord, act| {
            if act == action {
                removed.push(*chord);
                false
            } else {
                true
            }
        });
        removed
    }

    /// Replace the bindings for `action` in `context` with a single new chord.
    /// All old chords for that action are removed first.
    pub fn rebind(&mut self, context: InputContextId, action: GameAction, new_chord: KeyChord) {
        self.remove_binding(&context, &action);
        self.bind(context, new_chord, action);
    }

    /// List all (chord, action) pairs for a given context.
    pub fn list_bindings(&self, context: &InputContextId) -> Vec<(KeyChord, GameAction)> {
        self.contexts
            .get(context)
            .map(|ctx| ctx.iter().map(|(c, a)| (*c, a.clone())).collect())
            .unwrap_or_default()
    }

    /// List all global (chord, action) pairs.
    pub fn list_global(&self) -> Vec<(KeyChord, GameAction)> {
        self.global.iter().map(|(c, a)| (*c, a.clone())).collect()
    }
}

// ---------------------------------------------------------------------------
// Default bindings — sensible CDDA defaults
// ---------------------------------------------------------------------------

fn g(shift: bool, ctrl: bool, alt: bool, key: KeyCode) -> KeyChord {
    KeyChord {
        key,
        shift,
        ctrl,
        alt,
    }
}

fn k(key: KeyCode) -> KeyChord {
    KeyChord::new(key)
}

/// Helper to build a per-context binding map from a list of (key, action) pairs.
fn map_from(
    iter: impl IntoIterator<Item = (KeyChord, GameAction)>,
) -> HashMap<KeyChord, GameAction> {
    iter.into_iter().collect()
}

/// Return a `ContextBindings` populated with Cataclysm-DDA-inspired defaults.
pub fn default_bindings() -> ContextBindings {
    let mut bindings = ContextBindings {
        contexts: HashMap::new(),
        global: HashMap::new(),
    };

    // -- Global hotkeys (work everywhere) -----------------------------------
    bindings.bind_global(k(KeyCode::F5), GameAction::Quicksave);
    bindings.bind_global(k(KeyCode::F9), GameAction::Quickload);
    bindings.bind_global(k(KeyCode::F3), GameAction::ToggleDebug);
    bindings.bind_global(k(KeyCode::F1), GameAction::Help);
    bindings.bind_global(k(KeyCode::Slash), GameAction::Help);

    // -- Gameplay context ---------------------------------------------------
    bindings.contexts.insert(
        InputContextId::Gameplay,
        map_from([
            // Movement – vi-keys + arrows + numpad
            (k(KeyCode::KeyK), GameAction::Move(Direction::North)),
            (k(KeyCode::ArrowUp), GameAction::Move(Direction::North)),
            (k(KeyCode::Numpad8), GameAction::Move(Direction::North)),
            (k(KeyCode::KeyJ), GameAction::Move(Direction::South)),
            (k(KeyCode::ArrowDown), GameAction::Move(Direction::South)),
            (k(KeyCode::Numpad2), GameAction::Move(Direction::South)),
            (k(KeyCode::KeyL), GameAction::Move(Direction::East)),
            (k(KeyCode::ArrowRight), GameAction::Move(Direction::East)),
            (k(KeyCode::Numpad6), GameAction::Move(Direction::East)),
            (k(KeyCode::KeyH), GameAction::Move(Direction::West)),
            (k(KeyCode::ArrowLeft), GameAction::Move(Direction::West)),
            (k(KeyCode::Numpad4), GameAction::Move(Direction::West)),
            // Diagonals
            (k(KeyCode::KeyY), GameAction::Move(Direction::NorthWest)),
            (k(KeyCode::Numpad7), GameAction::Move(Direction::NorthWest)),
            (k(KeyCode::KeyU), GameAction::Move(Direction::NorthEast)),
            (k(KeyCode::Numpad9), GameAction::Move(Direction::NorthEast)),
            (k(KeyCode::KeyB), GameAction::Move(Direction::SouthWest)),
            (k(KeyCode::Numpad1), GameAction::Move(Direction::SouthWest)),
            (k(KeyCode::KeyN), GameAction::Move(Direction::SouthEast)),
            (k(KeyCode::Numpad3), GameAction::Move(Direction::SouthEast)),
            // Wait in place
            (k(KeyCode::Period), GameAction::Wait),
            (k(KeyCode::Numpad5), GameAction::Wait),
            // World interaction
            (k(KeyCode::KeyE), GameAction::Examine),
            (k(KeyCode::KeyG), GameAction::Pickup),
            (k(KeyCode::KeyD), GameAction::Drop),
            (k(KeyCode::KeyR), GameAction::Reload),
            (k(KeyCode::KeyF), GameAction::Fire),
            (k(KeyCode::KeyT), GameAction::Throw),
            // Debug tools
            (k(KeyCode::F2), GameAction::Custom(1)), // open debug spawn panel
            // Panel openers
            (k(KeyCode::KeyI), GameAction::OpenInventory),
            (k(KeyCode::KeyC), GameAction::OpenCrafting),
            (
                g(true, false, false, KeyCode::KeyC),
                GameAction::OpenCrafting,
            ), // shift+C
            (k(KeyCode::KeyP), GameAction::OpenCharacterSheet),
            (k(KeyCode::KeyM), GameAction::OpenMap),
        ]),
    );

    // -- MainMenu / hub screen context ------------------------------------
    bindings.contexts.insert(
        InputContextId::MainMenu,
        map_from([
            // Navigation — arrow keys move focus up/down through menu items.
            // hjkl are NOT bound for navigation here because they conflict
            // with hotkey letters (h=Help, l=Load Game).
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
            (k(KeyCode::ArrowLeft), GameAction::NavigateLeft),
            (k(KeyCode::ArrowRight), GameAction::NavigateRight),
            (k(KeyCode::PageUp), GameAction::NavigatePageUp),
            (k(KeyCode::PageDown), GameAction::NavigatePageDown),
            (k(KeyCode::Home), GameAction::NavigateHome),
            (k(KeyCode::End), GameAction::NavigateEnd),
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
            // Hotkeys — one letter per menu item, matching screen_def(MainMenu).commands
            // These take priority over any navigation binding.
            (k(KeyCode::KeyM), GameAction::HotkeyPress('m')), // [M]OTD
            (k(KeyCode::KeyN), GameAction::HotkeyPress('n')), // [N]ew Game
            (k(KeyCode::KeyL), GameAction::HotkeyPress('l')), // [L]oad Game
            (k(KeyCode::KeyW), GameAction::HotkeyPress('w')), // [W]orld
            (k(KeyCode::KeyS), GameAction::HotkeyPress('s')), // [S]pecial
            (k(KeyCode::KeyT), GameAction::HotkeyPress('t')), // Se[T]tings
            (k(KeyCode::KeyH), GameAction::HotkeyPress('h')), // [H]elp
            (k(KeyCode::KeyC), GameAction::HotkeyPress('c')), // [C]redits
            (k(KeyCode::KeyB), GameAction::HotkeyPress('b')), // [B]ack (sub-screens)
            (k(KeyCode::KeyQ), GameAction::HotkeyPress('q')), // [Q]uit
        ]),
    );

    // -- Inventory context --------------------------------------------------
    bindings.contexts.insert(
        InputContextId::Inventory,
        map_from([
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::KeyK), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
            (k(KeyCode::KeyJ), GameAction::NavigateDown),
            (k(KeyCode::ArrowLeft), GameAction::NavigatePrevTab),
            (k(KeyCode::KeyH), GameAction::NavigatePrevTab),
            (k(KeyCode::ArrowRight), GameAction::NavigateNextTab),
            (k(KeyCode::KeyL), GameAction::NavigateNextTab),
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::KeyE), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyQ), GameAction::Cancel),
            (k(KeyCode::Slash), GameAction::Filter),
            (k(KeyCode::PageUp), GameAction::NavigatePageUp),
            (k(KeyCode::PageDown), GameAction::NavigatePageDown),
            (k(KeyCode::Home), GameAction::NavigateHome),
            (k(KeyCode::End), GameAction::NavigateEnd),
            // [w] — wield / unwield focused item
            (k(KeyCode::KeyW), GameAction::UseItem),
        ]),
    );

    // -- CraftingMenu context ----------------------------------------------
    bindings.contexts.insert(
        InputContextId::CraftingMenu,
        map_from([
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::KeyK), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
            (k(KeyCode::KeyJ), GameAction::NavigateDown),
            (k(KeyCode::ArrowLeft), GameAction::NavigatePrevTab),
            (k(KeyCode::KeyH), GameAction::NavigatePrevTab),
            (k(KeyCode::ArrowRight), GameAction::NavigateNextTab),
            (k(KeyCode::KeyL), GameAction::NavigateNextTab),
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyQ), GameAction::Cancel),
            (k(KeyCode::Slash), GameAction::Filter),
            (k(KeyCode::PageUp), GameAction::NavigatePageUp),
            (k(KeyCode::PageDown), GameAction::NavigatePageDown),
        ]),
    );

    // -- CharacterSheet context --------------------------------------------
    bindings.contexts.insert(
        InputContextId::CharacterSheet,
        map_from([
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::KeyK), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
            (k(KeyCode::KeyJ), GameAction::NavigateDown),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyQ), GameAction::Cancel),
        ]),
    );

    // -- Dialog context ----------------------------------------------------
    bindings.contexts.insert(
        InputContextId::Dialog,
        map_from([
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::KeyY), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyN), GameAction::Cancel),
            (k(KeyCode::KeyQ), GameAction::Cancel),
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
        ]),
    );

    // -- ExamineLook context -----------------------------------------------
    bindings.contexts.insert(
        InputContextId::ExamineLook,
        map_from([
            (k(KeyCode::KeyK), GameAction::Move(Direction::North)),
            (k(KeyCode::ArrowUp), GameAction::Move(Direction::North)),
            (k(KeyCode::Numpad8), GameAction::Move(Direction::North)),
            (k(KeyCode::KeyJ), GameAction::Move(Direction::South)),
            (k(KeyCode::ArrowDown), GameAction::Move(Direction::South)),
            (k(KeyCode::Numpad2), GameAction::Move(Direction::South)),
            (k(KeyCode::KeyL), GameAction::Move(Direction::East)),
            (k(KeyCode::ArrowRight), GameAction::Move(Direction::East)),
            (k(KeyCode::Numpad6), GameAction::Move(Direction::East)),
            (k(KeyCode::KeyH), GameAction::Move(Direction::West)),
            (k(KeyCode::ArrowLeft), GameAction::Move(Direction::West)),
            (k(KeyCode::Numpad4), GameAction::Move(Direction::West)),
            (k(KeyCode::KeyY), GameAction::Move(Direction::NorthWest)),
            (k(KeyCode::Numpad7), GameAction::Move(Direction::NorthWest)),
            (k(KeyCode::KeyU), GameAction::Move(Direction::NorthEast)),
            (k(KeyCode::Numpad9), GameAction::Move(Direction::NorthEast)),
            (k(KeyCode::KeyB), GameAction::Move(Direction::SouthWest)),
            (k(KeyCode::Numpad1), GameAction::Move(Direction::SouthWest)),
            (k(KeyCode::KeyN), GameAction::Move(Direction::SouthEast)),
            (k(KeyCode::Numpad3), GameAction::Move(Direction::SouthEast)),
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyQ), GameAction::Cancel),
        ]),
    );

    // -- DirectionSelect context (same movement keys, consumed by caller) ---
    bindings.contexts.insert(
        InputContextId::DirectionSelect,
        map_from([
            (k(KeyCode::KeyK), GameAction::Move(Direction::North)),
            (k(KeyCode::ArrowUp), GameAction::Move(Direction::North)),
            (k(KeyCode::Numpad8), GameAction::Move(Direction::North)),
            (k(KeyCode::KeyJ), GameAction::Move(Direction::South)),
            (k(KeyCode::ArrowDown), GameAction::Move(Direction::South)),
            (k(KeyCode::Numpad2), GameAction::Move(Direction::South)),
            (k(KeyCode::KeyL), GameAction::Move(Direction::East)),
            (k(KeyCode::ArrowRight), GameAction::Move(Direction::East)),
            (k(KeyCode::Numpad6), GameAction::Move(Direction::East)),
            (k(KeyCode::KeyH), GameAction::Move(Direction::West)),
            (k(KeyCode::ArrowLeft), GameAction::Move(Direction::West)),
            (k(KeyCode::Numpad4), GameAction::Move(Direction::West)),
            (k(KeyCode::KeyY), GameAction::Move(Direction::NorthWest)),
            (k(KeyCode::Numpad7), GameAction::Move(Direction::NorthWest)),
            (k(KeyCode::KeyU), GameAction::Move(Direction::NorthEast)),
            (k(KeyCode::Numpad9), GameAction::Move(Direction::NorthEast)),
            (k(KeyCode::KeyB), GameAction::Move(Direction::SouthWest)),
            (k(KeyCode::Numpad1), GameAction::Move(Direction::SouthWest)),
            (k(KeyCode::KeyN), GameAction::Move(Direction::SouthEast)),
            (k(KeyCode::Numpad3), GameAction::Move(Direction::SouthEast)),
            (k(KeyCode::Escape), GameAction::Cancel),
        ]),
    );

    // -- TextInput context --------------------------------------------------
    // TextChar is handled specially in handle_raw_input (all printable keys).
    bindings.contexts.insert(
        InputContextId::TextInput,
        map_from([
            (k(KeyCode::Enter), GameAction::TextCommit),
            (k(KeyCode::Escape), GameAction::TextCancel),
            (k(KeyCode::Backspace), GameAction::TextBackspace),
            (k(KeyCode::Delete), GameAction::TextDelete),
            (k(KeyCode::ArrowLeft), GameAction::NavigateLeft),
            (k(KeyCode::ArrowRight), GameAction::NavigateRight),
            (k(KeyCode::Home), GameAction::NavigateHome),
            (k(KeyCode::End), GameAction::NavigateEnd),
        ]),
    );

    // -- Settings context ---------------------------------------------------
    bindings.contexts.insert(
        InputContextId::Settings,
        map_from([
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
            (k(KeyCode::ArrowLeft), GameAction::NavigateLeft),
            (k(KeyCode::ArrowRight), GameAction::NavigateRight),
            (k(KeyCode::Tab), GameAction::NavigateNextTab),
            (
                g(true, false, false, KeyCode::Tab),
                GameAction::NavigatePrevTab,
            ),
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyR), GameAction::Custom(0)), // reset to defaults
        ]),
    );

    // -- PauseMenu context -------------------------------------------------
    bindings.contexts.insert(
        InputContextId::PauseMenu,
        map_from([
            (k(KeyCode::ArrowUp), GameAction::NavigateUp),
            (k(KeyCode::ArrowDown), GameAction::NavigateDown),
            (k(KeyCode::Enter), GameAction::Confirm),
            (k(KeyCode::Escape), GameAction::Cancel),
        ]),
    );

    // -- QuantityInput context ---------------------------------------------
    bindings.contexts.insert(
        InputContextId::QuantityInput,
        map_from([
            (k(KeyCode::Enter), GameAction::TextCommit),
            (k(KeyCode::Escape), GameAction::TextCancel),
            (k(KeyCode::Backspace), GameAction::TextBackspace),
        ]),
    );

    // -- VehicleInteraction context ----------------------------------------
    bindings.contexts.insert(
        InputContextId::VehicleInteraction,
        map_from([
            (k(KeyCode::KeyK), GameAction::Move(Direction::North)),
            (k(KeyCode::ArrowUp), GameAction::Move(Direction::North)),
            (k(KeyCode::KeyJ), GameAction::Move(Direction::South)),
            (k(KeyCode::ArrowDown), GameAction::Move(Direction::South)),
            (k(KeyCode::KeyL), GameAction::Move(Direction::East)),
            (k(KeyCode::ArrowRight), GameAction::Move(Direction::East)),
            (k(KeyCode::KeyH), GameAction::Move(Direction::West)),
            (k(KeyCode::ArrowLeft), GameAction::Move(Direction::West)),
            (k(KeyCode::KeyE), GameAction::Examine),
            (k(KeyCode::Escape), GameAction::Cancel),
            (k(KeyCode::KeyQ), GameAction::Cancel),
            (k(KeyCode::Enter), GameAction::Confirm),
        ]),
    );

    bindings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::actions::{Direction, GameAction};
    use crate::input::context::InputContextId;

    /// Convenience: pack modifier bools into a slice for the old-style resolve.
    /// (Kept as a simple helper for test readability.)

    #[test]
    fn gameplay_vi_key_movement() {
        let bindings = default_bindings();
        let ctx = InputContextId::Gameplay;

        assert_eq!(
            bindings.resolve(&ctx, KeyCode::KeyK, false, false, false),
            Some(GameAction::Move(Direction::North)),
        );
        assert_eq!(
            bindings.resolve(&ctx, KeyCode::KeyJ, false, false, false),
            Some(GameAction::Move(Direction::South)),
        );
        assert_eq!(
            bindings.resolve(&ctx, KeyCode::KeyL, false, false, false),
            Some(GameAction::Move(Direction::East)),
        );
        assert_eq!(
            bindings.resolve(&ctx, KeyCode::KeyH, false, false, false),
            Some(GameAction::Move(Direction::West)),
        );
    }

    #[test]
    fn global_hotkeys_work_in_any_context() {
        let bindings = default_bindings();

        // F5 should resolve to Quicksave regardless of context.
        for ctx in &[
            InputContextId::Gameplay,
            InputContextId::Inventory,
            InputContextId::Dialog,
        ] {
            assert_eq!(
                bindings.resolve(ctx, KeyCode::F5, false, false, false),
                Some(GameAction::Quicksave),
                "F5 should resolve in {:?}",
                ctx,
            );
        }
    }

    #[test]
    fn context_specific_bindings() {
        let bindings = default_bindings();

        // 'e' is Examine in Gameplay, but Confirm in Inventory.
        assert_eq!(
            bindings.resolve(
                &InputContextId::Gameplay,
                KeyCode::KeyE,
                false,
                false,
                false
            ),
            Some(GameAction::Examine),
        );
        assert_eq!(
            bindings.resolve(
                &InputContextId::Inventory,
                KeyCode::KeyE,
                false,
                false,
                false
            ),
            Some(GameAction::Confirm),
        );
    }

    #[test]
    fn unbound_key_returns_none() {
        let bindings = default_bindings();
        assert_eq!(
            bindings.resolve(&InputContextId::Gameplay, KeyCode::F12, false, false, false),
            None,
        );
    }

    #[test]
    fn key_chord_modifiers() {
        let chord = KeyChord::new(KeyCode::KeyC);
        assert!(!chord.shift);
        assert!(!chord.ctrl);
        assert!(!chord.alt);

        let chord = KeyChord::new(KeyCode::KeyC).with_shift();
        assert!(chord.shift);
    }
}
