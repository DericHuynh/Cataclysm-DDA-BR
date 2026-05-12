//! Shared input action types — Direction, GameAction, ActionSource, InputAction.
//!
//! Extracted from `cdda_core::input::actions` so that downstream crates
//! (e.g. `cdda_replay`) can use them without depending on `cdda_core`.

use bevy_ecs::message::Message;
use bevy_ecs::prelude::Resource;
use bevy_reflect::Reflect;
use leafwing_input_manager::Actionlike;
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

// ---------------------------------------------------------------------------
// BindableAction — leafwing Actionlike
// ---------------------------------------------------------------------------

/// Every action that can be bound to a physical input.
///
/// This is the type used with `InputMap<BindableAction>` and
/// `ActionState<BindableAction>`.  Variants are all unit-only so that the
/// `Actionlike` derive works cleanly.
///
/// Data-carrying actions (`TextChar(String)`, `HotkeyPress(char)`) cannot be
/// bound here — they are generated at runtime in `handle_raw_input`.
#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum BindableAction {
    // -- Movement -----------------------------------------------------------
    MoveNorth,
    MoveSouth,
    MoveEast,
    MoveWest,
    MoveNorthEast,
    MoveNorthWest,
    MoveSouthEast,
    MoveSouthWest,
    Wait,
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
    Drop,

    // -- UI navigation ------------------------------------------------------
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
    TextCommit,
    TextCancel,
    TextBackspace,
    TextDelete,
    TextNavLeft,
    TextNavRight,
    TextNavHome,
    TextNavEnd,

    // -- Meta ---------------------------------------------------------------
    Pause,
    Help,
    ToggleDebug,
    Quicksave,
    Quickload,
    Quit,

    // -- Panel openers / screen transitions ---------------------------------
    OpenInventory,
    OpenCrafting,
    OpenCharacterSheet,
    OpenMap,
    OpenHelp,
    OpenCredits,
    OpenWorldMenu,
    OpenSettings,
    StartNewGame,
    LoadGame,
    StartGame,

    // -- Hotkey letters (a-z) used by menu screens --------------------------
    HotkeyA,
    HotkeyB,
    HotkeyC,
    HotkeyD,
    HotkeyE,
    HotkeyF,
    HotkeyG,
    HotkeyH,
    HotkeyI,
    HotkeyJ,
    HotkeyK,
    HotkeyL,
    HotkeyM,
    HotkeyN,
    HotkeyO,
    HotkeyP,
    HotkeyQ,
    HotkeyR,
    HotkeyS,
    HotkeyT,
    HotkeyU,
    HotkeyV,
    HotkeyW,
    HotkeyX,
    HotkeyY,
    HotkeyZ,

    // -- Extensible ---------------------------------------------------------
    Custom0,
    Custom1,
    Custom2,
    Custom3,
    Custom4,
}

impl BindableAction {
    /// Convert to the corresponding `GameAction`, expanding data variants.
    pub fn to_game_action(self) -> GameAction {
        use Direction::*;
        match self {
            Self::MoveNorth => GameAction::Move(North),
            Self::MoveSouth => GameAction::Move(South),
            Self::MoveEast => GameAction::Move(East),
            Self::MoveWest => GameAction::Move(West),
            Self::MoveNorthEast => GameAction::Move(NorthEast),
            Self::MoveNorthWest => GameAction::Move(NorthWest),
            Self::MoveSouthEast => GameAction::Move(SouthEast),
            Self::MoveSouthWest => GameAction::Move(SouthWest),
            Self::Wait => GameAction::Wait,
            Self::Crouch => GameAction::Crouch,
            Self::Examine => GameAction::Examine,
            Self::Pickup => GameAction::Pickup,
            Self::Open => GameAction::Open,
            Self::Close => GameAction::Close,
            Self::UseItem => GameAction::UseItem,
            Self::Fire => GameAction::Fire,
            Self::Reload => GameAction::Reload,
            Self::Throw => GameAction::Throw,
            Self::Drop => GameAction::Drop,
            Self::NavigateUp => GameAction::NavigateUp,
            Self::NavigateDown => GameAction::NavigateDown,
            Self::NavigateLeft => GameAction::NavigateLeft,
            Self::NavigateRight => GameAction::NavigateRight,
            Self::NavigateNextTab => GameAction::NavigateNextTab,
            Self::NavigatePrevTab => GameAction::NavigatePrevTab,
            Self::NavigatePageUp => GameAction::NavigatePageUp,
            Self::NavigatePageDown => GameAction::NavigatePageDown,
            Self::NavigateHome => GameAction::NavigateHome,
            Self::NavigateEnd => GameAction::NavigateEnd,
            Self::Confirm => GameAction::Confirm,
            Self::Cancel => GameAction::Cancel,
            Self::Toggle => GameAction::Toggle,
            Self::Filter => GameAction::Filter,
            Self::TextCommit => GameAction::TextCommit,
            Self::TextCancel => GameAction::TextCancel,
            Self::TextBackspace => GameAction::TextBackspace,
            Self::TextDelete => GameAction::TextDelete,
            Self::TextNavLeft => GameAction::NavigateLeft,
            Self::TextNavRight => GameAction::NavigateRight,
            Self::TextNavHome => GameAction::NavigateHome,
            Self::TextNavEnd => GameAction::NavigateEnd,
            Self::Pause => GameAction::Pause,
            Self::Help => GameAction::Help,
            Self::ToggleDebug => GameAction::ToggleDebug,
            Self::Quicksave => GameAction::Quicksave,
            Self::Quickload => GameAction::Quickload,
            Self::Quit => GameAction::Quit,
            Self::OpenInventory => GameAction::OpenInventory,
            Self::OpenCrafting => GameAction::OpenCrafting,
            Self::OpenCharacterSheet => GameAction::OpenCharacterSheet,
            Self::OpenMap => GameAction::OpenMap,
            Self::OpenHelp => GameAction::OpenHelp,
            Self::OpenCredits => GameAction::OpenCredits,
            Self::OpenWorldMenu => GameAction::OpenWorldMenu,
            Self::OpenSettings => GameAction::OpenSettings,
            Self::StartNewGame => GameAction::StartNewGame,
            Self::LoadGame => GameAction::LoadGame,
            Self::StartGame => GameAction::StartGame,
            Self::HotkeyA => GameAction::HotkeyPress('a'),
            Self::HotkeyB => GameAction::HotkeyPress('b'),
            Self::HotkeyC => GameAction::HotkeyPress('c'),
            Self::HotkeyD => GameAction::HotkeyPress('d'),
            Self::HotkeyE => GameAction::HotkeyPress('e'),
            Self::HotkeyF => GameAction::HotkeyPress('f'),
            Self::HotkeyG => GameAction::HotkeyPress('g'),
            Self::HotkeyH => GameAction::HotkeyPress('h'),
            Self::HotkeyI => GameAction::HotkeyPress('i'),
            Self::HotkeyJ => GameAction::HotkeyPress('j'),
            Self::HotkeyK => GameAction::HotkeyPress('k'),
            Self::HotkeyL => GameAction::HotkeyPress('l'),
            Self::HotkeyM => GameAction::HotkeyPress('m'),
            Self::HotkeyN => GameAction::HotkeyPress('n'),
            Self::HotkeyO => GameAction::HotkeyPress('o'),
            Self::HotkeyP => GameAction::HotkeyPress('p'),
            Self::HotkeyQ => GameAction::HotkeyPress('q'),
            Self::HotkeyR => GameAction::HotkeyPress('r'),
            Self::HotkeyS => GameAction::HotkeyPress('s'),
            Self::HotkeyT => GameAction::HotkeyPress('t'),
            Self::HotkeyU => GameAction::HotkeyPress('u'),
            Self::HotkeyV => GameAction::HotkeyPress('v'),
            Self::HotkeyW => GameAction::HotkeyPress('w'),
            Self::HotkeyX => GameAction::HotkeyPress('x'),
            Self::HotkeyY => GameAction::HotkeyPress('y'),
            Self::HotkeyZ => GameAction::HotkeyPress('z'),
            Self::Custom0 => GameAction::Custom(0),
            Self::Custom1 => GameAction::Custom(1),
            Self::Custom2 => GameAction::Custom(2),
            Self::Custom3 => GameAction::Custom(3),
            Self::Custom4 => GameAction::Custom(4),
        }
    }

    /// Human-readable display name for use in the settings / keybinding UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::MoveNorth => "Move North",
            Self::MoveSouth => "Move South",
            Self::MoveEast => "Move East",
            Self::MoveWest => "Move West",
            Self::MoveNorthEast => "Move NE",
            Self::MoveNorthWest => "Move NW",
            Self::MoveSouthEast => "Move SE",
            Self::MoveSouthWest => "Move SW",
            Self::Wait => "Wait",
            Self::Crouch => "Crouch",
            Self::Examine => "Examine",
            Self::Pickup => "Pick Up",
            Self::Open => "Open",
            Self::Close => "Close",
            Self::UseItem => "Use / Wield",
            Self::Fire => "Fire",
            Self::Reload => "Reload",
            Self::Throw => "Throw",
            Self::Drop => "Drop",
            Self::NavigateUp => "Navigate Up",
            Self::NavigateDown => "Navigate Down",
            Self::NavigateLeft => "Navigate Left",
            Self::NavigateRight => "Navigate Right",
            Self::NavigateNextTab => "Next Tab",
            Self::NavigatePrevTab => "Prev Tab",
            Self::NavigatePageUp => "Page Up",
            Self::NavigatePageDown => "Page Down",
            Self::NavigateHome => "Home",
            Self::NavigateEnd => "End",
            Self::Confirm => "Confirm",
            Self::Cancel => "Cancel / Back",
            Self::Toggle => "Toggle",
            Self::Filter => "Filter / Search",
            Self::TextCommit => "Text: Commit",
            Self::TextCancel => "Text: Cancel",
            Self::TextBackspace => "Text: Backspace",
            Self::TextDelete => "Text: Delete",
            Self::TextNavLeft => "Text: Cursor Left",
            Self::TextNavRight => "Text: Cursor Right",
            Self::TextNavHome => "Text: Home",
            Self::TextNavEnd => "Text: End",
            Self::Pause => "Pause",
            Self::Help => "Help",
            Self::ToggleDebug => "Toggle Debug",
            Self::Quicksave => "Quick Save",
            Self::Quickload => "Quick Load",
            Self::Quit => "Quit",
            Self::OpenInventory => "Open Inventory",
            Self::OpenCrafting => "Open Crafting",
            Self::OpenCharacterSheet => "Open Character",
            Self::OpenMap => "Open Map",
            Self::OpenHelp => "Open Help",
            Self::OpenCredits => "Credits",
            Self::OpenWorldMenu => "World Menu",
            Self::OpenSettings => "Settings",
            Self::StartNewGame => "Start New Game",
            Self::LoadGame => "Load Game",
            Self::StartGame => "Start Game",
            Self::HotkeyA => "Hotkey: a",
            Self::HotkeyB => "Hotkey: b",
            Self::HotkeyC => "Hotkey: c",
            Self::HotkeyD => "Hotkey: d",
            Self::HotkeyE => "Hotkey: e",
            Self::HotkeyF => "Hotkey: f",
            Self::HotkeyG => "Hotkey: g",
            Self::HotkeyH => "Hotkey: h",
            Self::HotkeyI => "Hotkey: i",
            Self::HotkeyJ => "Hotkey: j",
            Self::HotkeyK => "Hotkey: k",
            Self::HotkeyL => "Hotkey: l",
            Self::HotkeyM => "Hotkey: m",
            Self::HotkeyN => "Hotkey: n",
            Self::HotkeyO => "Hotkey: o",
            Self::HotkeyP => "Hotkey: p",
            Self::HotkeyQ => "Hotkey: q",
            Self::HotkeyR => "Hotkey: r",
            Self::HotkeyS => "Hotkey: s",
            Self::HotkeyT => "Hotkey: t",
            Self::HotkeyU => "Hotkey: u",
            Self::HotkeyV => "Hotkey: v",
            Self::HotkeyW => "Hotkey: w",
            Self::HotkeyX => "Hotkey: x",
            Self::HotkeyY => "Hotkey: y",
            Self::HotkeyZ => "Hotkey: z",
            Self::Custom0 => "Custom 0",
            Self::Custom1 => "Custom 1",
            Self::Custom2 => "Custom 2",
            Self::Custom3 => "Custom 3",
            Self::Custom4 => "Custom 4",
        }
    }

    /// Returns every variant — used for iterating all possible actions when
    /// querying `ActionState` or building the settings keybinding list.
    pub fn all() -> impl Iterator<Item = Self> {
        [
            Self::MoveNorth,
            Self::MoveSouth,
            Self::MoveEast,
            Self::MoveWest,
            Self::MoveNorthEast,
            Self::MoveNorthWest,
            Self::MoveSouthEast,
            Self::MoveSouthWest,
            Self::Wait,
            Self::Crouch,
            Self::Examine,
            Self::Pickup,
            Self::Open,
            Self::Close,
            Self::UseItem,
            Self::Fire,
            Self::Reload,
            Self::Throw,
            Self::Drop,
            Self::NavigateUp,
            Self::NavigateDown,
            Self::NavigateLeft,
            Self::NavigateRight,
            Self::NavigateNextTab,
            Self::NavigatePrevTab,
            Self::NavigatePageUp,
            Self::NavigatePageDown,
            Self::NavigateHome,
            Self::NavigateEnd,
            Self::Confirm,
            Self::Cancel,
            Self::Toggle,
            Self::Filter,
            Self::TextCommit,
            Self::TextCancel,
            Self::TextBackspace,
            Self::TextDelete,
            Self::TextNavLeft,
            Self::TextNavRight,
            Self::TextNavHome,
            Self::TextNavEnd,
            Self::Pause,
            Self::Help,
            Self::ToggleDebug,
            Self::Quicksave,
            Self::Quickload,
            Self::Quit,
            Self::OpenInventory,
            Self::OpenCrafting,
            Self::OpenCharacterSheet,
            Self::OpenMap,
            Self::OpenHelp,
            Self::OpenCredits,
            Self::OpenWorldMenu,
            Self::OpenSettings,
            Self::StartNewGame,
            Self::LoadGame,
            Self::StartGame,
            Self::HotkeyA,
            Self::HotkeyB,
            Self::HotkeyC,
            Self::HotkeyD,
            Self::HotkeyE,
            Self::HotkeyF,
            Self::HotkeyG,
            Self::HotkeyH,
            Self::HotkeyI,
            Self::HotkeyJ,
            Self::HotkeyK,
            Self::HotkeyL,
            Self::HotkeyM,
            Self::HotkeyN,
            Self::HotkeyO,
            Self::HotkeyP,
            Self::HotkeyQ,
            Self::HotkeyR,
            Self::HotkeyS,
            Self::HotkeyT,
            Self::HotkeyU,
            Self::HotkeyV,
            Self::HotkeyW,
            Self::HotkeyX,
            Self::HotkeyY,
            Self::HotkeyZ,
            Self::Custom0,
            Self::Custom1,
            Self::Custom2,
            Self::Custom3,
            Self::Custom4,
        ]
        .into_iter()
    }
}

// ---------------------------------------------------------------------------
// InputContextId
// ---------------------------------------------------------------------------

/// Identifies an input context — a named mode with its own keybindings.
///
/// The `InputContextStack` determines which context is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputContextId {
    /// Default gameplay / world view.
    Gameplay,
    /// Full-screen inventory panel.
    Inventory,
    /// Crafting recipe browser.
    CraftingMenu,
    /// Character sheet / stats panel.
    CharacterSheet,
    /// Examine-look mode (browsing what's at a tile).
    ExamineLook,
    /// Generic dialog / yes-no prompt.
    Dialog,
    /// Selecting a direction (e.g. for throwing, shooting, placing).
    DirectionSelect,
    /// Free-form text input (rename, search, etc.).
    TextInput,
    MainMenu,
    /// Settings menu (tab navigation, keybind editor).
    Settings,
    PauseMenu,
    /// Numeric quantity input (e.g. "how many?").
    QuantityInput,
    /// Vehicle interaction / driving controls.
    VehicleInteraction,
    /// Extensible fallback for mod-specific contexts.
    Custom(u32),
}

// ---------------------------------------------------------------------------
// InputContextStack
// ---------------------------------------------------------------------------

/// A stack of active input contexts.
///
/// Only the **top** context is used to resolve key presses into actions,
/// but global bindings (hotkeys) work regardless of the current context.
///
/// # Example
/// ```ignore
/// let stack = InputContextStack::new();
/// assert_eq!(stack.top(), InputContextId::Gameplay);
///
/// stack.push(InputContextId::Inventory);
/// assert_eq!(stack.top(), InputContextId::Inventory);
///
/// stack.pop();
/// assert_eq!(stack.top(), InputContextId::Gameplay);
/// ```
#[derive(Resource, Debug, Clone)]
pub struct InputContextStack {
    contexts: Vec<InputContextId>,
}

impl InputContextStack {
    /// Create a new stack with `MainMenu` at the top (app starts on main menu).
    pub fn new() -> Self {
        Self {
            contexts: vec![InputContextId::MainMenu],
        }
    }

    /// Push a new context onto the stack.
    pub fn push(&mut self, ctx: InputContextId) {
        self.contexts.push(ctx);
    }

    /// Pop the top context and return it.
    ///
    /// Returns `None` if only `Gameplay` remains (Gameplay is never popped).
    pub fn pop(&mut self) -> Option<InputContextId> {
        if self.contexts.len() <= 1 {
            None
        } else {
            self.contexts.pop()
        }
    }

    /// Peek at the top context without removing it.
    pub fn top(&self) -> InputContextId {
        *self.contexts.last().unwrap_or(&InputContextId::Gameplay)
    }

    /// Replace the top context with a different one.
    ///
    /// This is equivalent to popping and pushing, but is a single operation.
    pub fn replace_top(&mut self, ctx: InputContextId) {
        if !self.contexts.is_empty() {
            let last = self.contexts.len() - 1;
            self.contexts[last] = ctx;
        } else {
            self.contexts.push(ctx);
        }
    }

    /// How many contexts are on the stack.
    pub fn depth(&self) -> usize {
        self.contexts.len()
    }

    /// Reset the stack to just `Gameplay`.
    pub fn clear(&mut self) {
        self.contexts.truncate(1);
        self.contexts[0] = InputContextId::Gameplay;
    }

    /// Iterate over all contexts from bottom (Gameplay) to top.
    pub fn iter(&self) -> impl Iterator<Item = &InputContextId> {
        self.contexts.iter()
    }
}

impl Default for InputContextStack {
    fn default() -> Self {
        Self::new()
    }
}
