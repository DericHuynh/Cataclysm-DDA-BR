//! Semantic game actions and input events.
//!
//! Systems never see raw keys — they consume `InputAction` events
//! that carry a resolved `GameAction` and an `ActionSource`.

use bevy_ecs::message::Message;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Direction
// ---------------------------------------------------------------------------

/// The eight compass directions plus Up, Down, and Here (wait in place).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    Up,
    Down,
    Here,
}

// ---------------------------------------------------------------------------
// GameAction
// ---------------------------------------------------------------------------

/// Every action a player can trigger through any input device.
///
/// This is the **only** way systems should learn about player intent.
/// No system should ever read `ButtonInput<KeyCode>` directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameAction {
    // -- Movement -----------------------------------------------------------
    Move(Direction),
    Run(Direction),
    Crouch,

    // -- World interaction --------------------------------------------------
    Examine,
    Pickup,
    Open,
    Close,
    UseItem,
    Fire,
    Reload,
    Throw,
    Wait,
    Drop,

    // -- UI panel navigation ------------------------------------------------
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    NavigateNextTab,
    NavigatePrevTab,
    NavigatePageUp,
    NavigatePageDown,
    NavigateHome,
    NavigateEnd,

    // -- Selection ----------------------------------------------------------
    Confirm,
    Cancel,
    Toggle,
    Filter,

    // -- Text input ---------------------------------------------------------
    /// A typed character (or multi-character sequence on some platforms).
    /// The value comes from `KeyboardInput::logical_key`'s `Key::Character(ch)`
    /// (layout-correct, cross-platform).
    /// The old `key_to_printable_char` QWERTY table has been deleted.
    TextChar(String),
    TextBackspace,
    TextDelete,
    TextCommit,
    TextCancel,

    // -- Meta actions -------------------------------------------------------
    Pause,
    Help,
    ToggleDebug,
    Quicksave,
    Quickload,

    /// A screen-specific hotkey letter was pressed (e.g. 'n' for New Game).
    /// `handle_navigation_input` matches this against `screen_def().commands[i].hotkey`.
    HotkeyPress(char),

    // -- Main menu actions -------------------------------------------------
    StartNewGame,
    LoadGame,
    OpenSettings,
    ShowMotd,
    OpenSpecial,
    Quit,
    StartGame,

    // -- Panel openers ------------------------------------------------------
    OpenInventory,
    OpenCrafting,
    OpenCharacterSheet,
    OpenMap,
    OpenHelp,
    OpenCredits,
    OpenWorldMenu,

    // -- Extensible ---------------------------------------------------------
    /// Fallback for mods or scripts that need a raw action id.
    Custom(u32),
}

// ---------------------------------------------------------------------------
// ActionSource
// ---------------------------------------------------------------------------

/// Which device produced the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionSource {
    Keyboard,
    Gamepad,
    Mouse,
    Script,
}

// ---------------------------------------------------------------------------
// InputAction (Bevy event)
// ---------------------------------------------------------------------------

/// A semantic input event — the core decoupling point between raw input and
/// game logic.
///
/// Systems subscribe to this event and match on `action` to react.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct InputAction {
    pub action: GameAction,
    pub source: ActionSource,
}

impl InputAction {
    pub fn new(action: GameAction, source: ActionSource) -> Self {
        Self { action, source }
    }

    /// Convenience constructor for keyboard-originated actions.
    pub fn keyboard(action: GameAction) -> Self {
        Self {
            action,
            source: ActionSource::Keyboard,
        }
    }
}
