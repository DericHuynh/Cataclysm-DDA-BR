//! Shared context / screen-state types extracted from `cdda_core` so that
//! downstream crates (e.g. `cdda_inventory`) can reference them without a
//! circular dependency on `cdda_core`.

use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

// ---------------------------------------------------------------------------
// Ctx — Bevy States
// ---------------------------------------------------------------------------

/// Identifies which top-level screen or overlay is currently active.
///
/// This is a Bevy `States` enum — use `OnEnter(Ctx::...)` /
/// `OnExit(Ctx::...)` to spawn/despawn UI in render crates.
/// The `#[default]` is `MainMenu`.
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Ctx {
    /// Title screen / main menu.
    #[default]
    MainMenu,
    /// Free-roam gameplay with the world viewport.
    Gameplay,
    /// Hub screen after clicking "New Game".
    NewGameHub,
    /// Scenario selection screen.
    ScenarioSelect,
    /// Profession / class selection screen.
    ProfessionSelect,
    /// Character creation / point-buy screen.
    CharacterCreation,
    /// Final character review before starting the game.
    CharacterConfirm,
    /// World creation / selection screen.
    WorldMenu,
    /// World-specific settings (mods, difficulty, etc.).
    WorldSettings,
    /// Settings / options menu.
    SettingsMenu,
    /// Help / keybindings reference screen.
    HelpScreen,
    /// Credits screen.
    CreditsScreen,
    /// Full-screen inventory panel.
    Inventory,
    /// Item detail / examine overlay.
    ItemExamine,
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
    /// In-game pause menu.
    PauseMenu,
    /// Quantity input (e.g. "how many?").
    QuantityInput,
    /// Vehicle interaction / driving controls.
    VehicleInteraction,
    /// Dev-worldgen building showcase.
    DevWorldgen,
    /// Overmap viewer — navigable terrain map.
    Overmap,
    /// Debug item-spawn panel (press F2 in gameplay).
    DevSpawnPanel,
    /// Debug registry viewer — browse all definition registries.
    RegistryViewer,
    /// Extensible fallback.
    Custom(u32),
}

// ---------------------------------------------------------------------------
// ContextStack — hierarchical navigation
// ---------------------------------------------------------------------------

/// Tracks the parent screen so `pop_ctx` knows where to go back.
///
/// Pushing a child pushes onto this stack; popping restores the previous
/// screen.  The stack is kept in sync with `NextState<Ctx>` by the
/// `push_ctx` / `pop_ctx` helpers below.
#[derive(Resource, Default, Debug, Clone)]
pub struct ContextStack(pub Vec<Ctx>);

// ---------------------------------------------------------------------------
// FocusedCommandIndex
// ---------------------------------------------------------------------------

/// Tracks the focused command index **per screen** so that returning via
/// Escape restores the cursor to where it was left.
#[derive(Resource, Default, Debug, Clone)]
pub struct FocusedCommandIndex {
    history: std::collections::HashMap<Ctx, usize>,
    current: usize,
}

impl FocusedCommandIndex {
    /// The focused index for the current screen.
    pub fn current(&self) -> usize {
        self.current
    }

    /// Move focus within the current screen.
    pub fn set(&mut self, idx: usize) {
        if self.current != idx {
            self.current = idx;
        }
    }

    /// Save focus for `from`, load saved focus for `to` (0 if first visit).
    pub fn on_push(&mut self, from: Ctx, to: Ctx) {
        self.history.insert(from, self.current);
        self.current = *self.history.get(&to).unwrap_or(&0);
    }

    /// Restore saved focus for `to` (the screen we are returning to).
    pub fn on_pop(&mut self, to: Ctx) {
        let saved = *self.history.get(&to).unwrap_or(&0);
        if self.current != saved {
            self.current = saved;
        }
    }
}

// ---------------------------------------------------------------------------
// push_ctx / pop_ctx
// ---------------------------------------------------------------------------

/// Push a child screen. Saves current focus for `current`, restores saved
/// focus for `next_screen`. Always call this instead of touching
/// `ContextStack`, `NextState`, and `FocusedCommandIndex` separately.
pub fn push_ctx(
    current: Ctx,
    next_screen: Ctx,
    stack: &mut ContextStack,
    next: &mut NextState<Ctx>,
    focused: &mut FocusedCommandIndex,
) {
    focused.on_push(current, next_screen);
    stack.0.push(current);
    next.set(next_screen);
}

/// Pop back to the parent screen, restoring saved focus. No-op if empty.
pub fn pop_ctx(
    stack: &mut ContextStack,
    next: &mut NextState<Ctx>,
    focused: &mut FocusedCommandIndex,
) {
    if let Some(parent) = stack.0.pop() {
        focused.on_pop(parent);
        next.set(parent);
    }
}
