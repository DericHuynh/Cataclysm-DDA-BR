//! Shared input action types — Direction, GameAction, ActionSource, InputAction.
//!
//! Extracted from `cdda_core::input::actions` so that downstream crates
//! (e.g. `cdda_replay`) can use them without depending on `cdda_core`.

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
///
/// Data-carrying variants (`TextChar`, `HotkeyPress`, `Custom`, `Move`, `Run`)
/// are generated at runtime and cannot appear in an `InputMap`.
/// Use `BindableAction` for binding storage.
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
    /// A typed character from `KeyboardInput::logical_key`.
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

    /// A screen-specific hotkey letter (e.g. 'n' for New Game).
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
// InputAction (Bevy message)
// ---------------------------------------------------------------------------

/// A semantic input event — the core decoupling point between raw input and
/// game logic.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct InputAction {
    pub action: GameAction,
    pub source: ActionSource,
}

impl InputAction {
    pub fn new(action: GameAction, source: ActionSource) -> Self {
        Self { action, source }
    }

    pub fn keyboard(action: GameAction) -> Self {
        Self {
            action,
            source: ActionSource::Keyboard,
        }
    }
}
