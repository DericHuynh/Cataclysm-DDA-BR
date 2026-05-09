//! Screen state machine — which screen is currently active.
//!
//! `Screen` is a Bevy `States` enum (not a flat `Resource`). This means
//! render crates can register `OnEnter(Ctx::Foo)` / `OnExit(Ctx::Foo)`
//! systems and Bevy schedules them automatically.
//!
//! `ContextStack` tracks the hierarchy: push a child screen, pop back to
//! parent. The stack is kept in sync with `NextState<Ctx>` by the
//! `push_ctx` / `pop_ctx` helpers — callers never touch both manually.

use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

// ---------------------------------------------------------------------------
// Screen — Bevy States
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
    /// Debug item-spawn panel (press F2 in gameplay).
    DevSpawnPanel,
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
