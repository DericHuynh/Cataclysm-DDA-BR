//! Input context stack — determines which keybindings are active.
//!
//! The current context decides how raw keys are resolved into `GameAction`
//! events.  Only the topmost context is used for resolution, while global
//! bindings always apply.

use bevy_ecs::prelude::Resource;

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
