//! Ctx navigation — static definitions, transitions, and input handling.
//!
//! Each screen declares its commands via `screen_def()`. The navigation
//! system reads `GameAction` events, checks the focused command index,
//! and dispatches transitions (Push, Replace, Pop, Quit, Event).

use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

use crate::input::{GameAction, InputAction};

use crate::context::ctx::{ContextStack, Ctx};
use crate::context::overlay::OverlayStack;

// ---------------------------------------------------------------------------
// TransitionTarget
// ---------------------------------------------------------------------------

/// Where a screen command leads.
#[derive(Debug, Clone)]
pub enum TransitionTarget {
    /// Push a child screen onto the stack.
    Push(Ctx),
    /// Replace the current screen (no back-navigation parent change).
    Replace(Ctx),
    /// Pop back to the parent screen.
    Pop,
    /// Quit the application.
    Quit,
    /// Emit a `GameEvent` for cross-cutting concerns (e.g. start game).
    Event(GameEvent),
}

// ---------------------------------------------------------------------------
// ScreenCommand / ScreenDefinition
// ---------------------------------------------------------------------------

/// A single command the user can trigger on this screen.
#[derive(Debug, Clone)]
pub struct ScreenCommand {
    /// Display text.
    pub label: &'static str,
    /// Optional hotkey character.
    pub hotkey: Option<char>,
    /// What happens when the command is activated.
    pub target: TransitionTarget,
}

/// Static navigation data for a screen.
#[derive(Debug, Clone)]
pub struct ScreenDefinition {
    /// Title shown at the top of the screen.
    pub title: &'static str,
    /// Available commands for this screen.
    pub commands: Vec<ScreenCommand>,
}

// ---------------------------------------------------------------------------
// GameEvent
// ---------------------------------------------------------------------------

/// Cross-cutting game events that transcend screen navigation.
///
/// For example, when the player clicks "Start Game" on the character
/// confirm screen, the command emits `GameEvent::StartNewGame` rather
/// than navigating to a new screen. An application-level system listens
/// for this event and transitions `AppState`.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    /// Player confirmed character creation — begin loading + worldgen.
    StartNewGame,
    /// Save and exit to main menu.
    SaveAndQuit,
}

// ---------------------------------------------------------------------------
// FocusedCommandIndex
// ---------------------------------------------------------------------------

/// Tracks the focused command index **per screen** so that returning via
/// Escape restores the cursor to where it was left.
///
/// Renderers call `current()` to know what to highlight.
/// Only `handle_navigation_input` writes to this resource.
#[derive(Resource, Default, Debug, Clone)]
pub struct FocusedCommandIndex {
    history: std::collections::HashMap<crate::context::ctx::Ctx, usize>,
    current: usize,
}

impl FocusedCommandIndex {
    /// The focused index for the current screen.
    pub fn current(&self) -> usize {
        self.current
    }

    /// Move focus within the current screen.
    pub fn set(&mut self, idx: usize) {
        self.current = idx;
    }

    /// Save focus for `from`, load saved focus for `to` (0 if first visit).
    pub fn on_push(&mut self, from: crate::context::ctx::Ctx, to: crate::context::ctx::Ctx) {
        self.history.insert(from, self.current);
        self.current = *self.history.get(&to).unwrap_or(&0);
    }

    /// Restore saved focus for `to` (the screen we are returning to).
    pub fn on_pop(&mut self, to: crate::context::ctx::Ctx) {
        self.current = *self.history.get(&to).unwrap_or(&0);
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

// ---------------------------------------------------------------------------
// screen_def — static navigation data for every screen
// ---------------------------------------------------------------------------

fn cmd(label: &'static str, hotkey: Option<char>, target: TransitionTarget) -> ScreenCommand {
    ScreenCommand {
        label,
        hotkey,
        target,
    }
}

/// Return the static definition for a screen.
pub fn ctx_def(screen: Ctx) -> ScreenDefinition {
    use Ctx::*;
    use TransitionTarget::*;

    match screen {
        MainMenu => ScreenDefinition {
            title: "CATACLYSM: DARK DAYS AHEAD",
            commands: vec![
                cmd("MOTD", Some('m'), Push(Ctx::Custom(0))),
                cmd("New Game", Some('n'), Push(NewGameHub)),
                cmd("Load Game", Some('l'), Push(Ctx::Custom(1))),
                cmd("World", Some('w'), Push(WorldMenu)),
                cmd("Special", Some('s'), Push(Ctx::DevWorldgen)),
                cmd("Settings", Some('t'), Push(SettingsMenu)),
                cmd("Help", Some('h'), Push(HelpScreen)),
                cmd("Credits", Some('c'), Push(CreditsScreen)),
                cmd("Quit", Some('q'), Quit),
            ],
        },

        NewGameHub => ScreenDefinition {
            title: "NEW GAME",
            commands: vec![
                cmd("Character", Some('c'), Push(ScenarioSelect)),
                cmd("World", Some('w'), Push(WorldMenu)),
                cmd("Start Game", Some('s'), Event(GameEvent::StartNewGame)),
            ],
        },

        ScenarioSelect => ScreenDefinition {
            title: "CHOOSE SCENARIO",
            commands: Vec::new(), // populated by ScreenListItem entities
        },

        ProfessionSelect => ScreenDefinition {
            title: "CHOOSE PROFESSION",
            commands: Vec::new(),
        },

        CharacterCreation => ScreenDefinition {
            title: "CHARACTER CREATION",
            commands: Vec::new(),
        },

        CharacterConfirm => ScreenDefinition {
            title: "CONFIRM CHARACTER",
            commands: vec![
                cmd("Start Game", Some('s'), Event(GameEvent::StartNewGame)),
                cmd("Go Back", Some('b'), Pop),
            ],
        },

        WorldMenu => ScreenDefinition {
            title: "WORLD SELECTION",
            commands: vec![
                cmd("Create World", Some('c'), Push(WorldSettings)),
                cmd("Select World", Some('s'), Replace(NewGameHub)),
            ],
        },

        WorldSettings => ScreenDefinition {
            title: "WORLD SETTINGS",
            commands: vec![cmd("Save & Return", Some('s'), Pop)],
        },

        SettingsMenu => ScreenDefinition {
            title: "SETTINGS",
            commands: Vec::new(),
        },

        HelpScreen => ScreenDefinition {
            title: "HELP",
            commands: Vec::new(),
        },

        CreditsScreen => ScreenDefinition {
            title: "CREDITS",
            commands: Vec::new(),
        },

        // In-game screens
        Gameplay => ScreenDefinition {
            title: "",
            commands: vec![
                cmd("Inventory", Some('i'), Push(Inventory)),
                cmd("Craft", Some('c'), Push(CraftingMenu)),
                cmd("Character", Some('@'), Push(CharacterSheet)),
                cmd("Pause", Some('p'), Push(PauseMenu)),
            ],
        },

        Inventory | ItemExamine | CraftingMenu | CharacterSheet | PauseMenu | ExamineLook
        | Dialog | DirectionSelect | TextInput | QuantityInput | VehicleInteraction => {
            ScreenDefinition {
                title: "",
                commands: Vec::new(),
            }
        }

        DevSpawnPanel => ScreenDefinition {
            title: "DEBUG: SPAWN ITEM",
            commands: Vec::new(),
        },

        DevWorldgen => ScreenDefinition {
            title: "DEV WORLDGEN — BUILDING SHOWCASE",
            commands: vec![
                cmd("Start Showcase", Some('s'), Event(GameEvent::StartNewGame)),
                cmd("Go Back", Some('b'), Pop),
            ],
        },

        Custom(_) => ScreenDefinition {
            title: "",
            commands: Vec::new(),
        },
    }
}

// ---------------------------------------------------------------------------
// dispatch — shared transition logic
// ---------------------------------------------------------------------------

fn dispatch(
    target: &TransitionTarget,
    current: Ctx,
    stack: &mut ContextStack,
    next: &mut NextState<Ctx>,
    focused: &mut FocusedCommandIndex,
    game_events: &mut MessageWriter<GameEvent>,
) {
    match target {
        TransitionTarget::Push(s) => push_ctx(current, *s, stack, next, focused),
        TransitionTarget::Replace(s) => next.set(*s),
        TransitionTarget::Pop => pop_ctx(stack, next, focused),
        TransitionTarget::Quit => std::process::exit(0),
        TransitionTarget::Event(e) => {
            let _ = game_events.write(*e);
        }
    }
}

// ---------------------------------------------------------------------------
// handle_navigation_input — the core navigation system
// ---------------------------------------------------------------------------

/// Reads `GameAction` events, checks the focused command, and dispatches.
///
/// This is the ONLY place navigation transitions happen. No `apply_screen`
/// god function with if-else chains — just `screen_def`, `ContextStack`, and
/// `TransitionTarget`.
pub fn handle_navigation_input(
    mut reader: MessageReader<InputAction>,
    mut stack: ResMut<ContextStack>,
    mut next: ResMut<NextState<Ctx>>,
    mut focused: ResMut<FocusedCommandIndex>,
    state: Res<State<Ctx>>,
    mut game_events: MessageWriter<GameEvent>,
    overlays: Res<OverlayStack>,
    list_items: Query<(), (With<super::ScreenListItem>,)>,
) {
    let current = *state.get();

    // If an overlay is active, only process Cancel (to dismiss it).
    // The actual pop happens via commands in the next frame.
    if overlays.input_blocked {
        for event in reader.read() {
            if matches!(&event.action, GameAction::Cancel) {
                // Signal to dismiss — exclusive system handles the rest.
            }
        }
        return;
    }

    let def = ctx_def(current);
    let item_count = list_items.iter().count();
    let total_items = def.commands.len() + item_count;

    for event in reader.read() {
        match &event.action {
            GameAction::NavigateUp | GameAction::NavigateLeft => {
                if total_items > 0 {
                    let idx = focused.current().saturating_sub(1);
                    focused.set(idx);
                }
            }
            GameAction::NavigateDown | GameAction::NavigateRight => {
                if total_items > 0 {
                    let idx = (focused.current() + 1).min(total_items.saturating_sub(1));
                    focused.set(idx);
                }
            }
            GameAction::NavigateHome => {
                focused.set(0);
            }
            GameAction::NavigateEnd => {
                if total_items > 0 {
                    let idx = total_items.saturating_sub(1);
                    focused.set(idx);
                }
            }
            GameAction::Confirm => {
                let cmd_idx = focused.current().saturating_sub(item_count);
                if let Some(cmd) = def.commands.get(cmd_idx) {
                    let target = cmd.target.clone();
                    dispatch(
                        &target,
                        current,
                        &mut stack,
                        &mut next,
                        &mut focused,
                        &mut game_events,
                    );
                }
            }
            GameAction::Cancel => {
                // Some screens handle Cancel in their own input systems (crafting
                // menu calls pop_ctx itself).  Don't double-pop for those.
                let screen_handles_cancel = matches!(current, Ctx::CraftingMenu | Ctx::ItemExamine);
                if !screen_handles_cancel {
                    pop_ctx(&mut stack, &mut next, &mut focused);
                }
            }
            GameAction::HotkeyPress(ch) => {
                if let Some((idx, cmd)) = def
                    .commands
                    .iter()
                    .enumerate()
                    .find(|(_, c)| c.hotkey == Some(*ch))
                {
                    focused.set(idx);
                    let target = cmd.target.clone();
                    dispatch(
                        &target,
                        current,
                        &mut stack,
                        &mut next,
                        &mut focused,
                        &mut game_events,
                    );
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// handle_panel_openers — OpenInventory / OpenCrafting / etc.
// ---------------------------------------------------------------------------

/// Maps `GameAction::Open*` actions to screen pushes.
///
/// These actions are emitted by context-specific keybindings (e.g. `i` in
/// Gameplay → `OpenInventory`) and need to transition to the appropriate
/// child screen. Runs in `PreUpdate` alongside `handle_navigation_input`.
pub fn handle_panel_openers(
    mut reader: MessageReader<InputAction>,
    state: Res<State<Ctx>>,
    mut stack: ResMut<ContextStack>,
    mut next: ResMut<NextState<Ctx>>,
    mut focused: ResMut<FocusedCommandIndex>,
) {
    let current = *state.get();
    for event in reader.read() {
        match &event.action {
            GameAction::OpenInventory => {
                push_ctx(current, Ctx::Inventory, &mut stack, &mut next, &mut focused);
            }
            GameAction::OpenCrafting => {
                push_ctx(
                    current,
                    Ctx::CraftingMenu,
                    &mut stack,
                    &mut next,
                    &mut focused,
                );
            }
            GameAction::OpenCharacterSheet => {
                push_ctx(
                    current,
                    Ctx::CharacterSheet,
                    &mut stack,
                    &mut next,
                    &mut focused,
                );
            }
            GameAction::OpenHelp => {
                push_ctx(
                    current,
                    Ctx::HelpScreen,
                    &mut stack,
                    &mut next,
                    &mut focused,
                );
            }
            GameAction::OpenCredits => {
                push_ctx(
                    current,
                    Ctx::CreditsScreen,
                    &mut stack,
                    &mut next,
                    &mut focused,
                );
            }
            GameAction::OpenWorldMenu => {
                push_ctx(current, Ctx::WorldMenu, &mut stack, &mut next, &mut focused);
            }
            // Custom(1) opens the debug spawn panel (bound to F2 in gameplay context).
            GameAction::Custom(1) if current == Ctx::Gameplay => {
                push_ctx(
                    current,
                    Ctx::DevSpawnPanel,
                    &mut stack,
                    &mut next,
                    &mut focused,
                );
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// ScreenListItem — marker for dynamic list items
// ---------------------------------------------------------------------------

/// Marker component for data-driven screen items (e.g. scenario list).
/// Renderers spawn/despawn these on `OnEnter`/`OnExit`.
#[derive(Component, Debug, Clone)]
pub struct ScreenListItem {
    pub index: usize,
    pub label: String,
}

/// System that synchronises `InputContextStack` to match the current screen.
/// Runs on state transitions so keyboard bindings switch with the screen.
pub fn sync_input_context(
    mut stack: ResMut<crate::input::context::InputContextStack>,
    screen: Res<State<Ctx>>,
) {
    use crate::input::InputContextId;
    use Ctx::*;

    let ctx = match *screen.get() {
        MainMenu | NewGameHub | HelpScreen | CreditsScreen | WorldMenu | WorldSettings
        | DevWorldgen | ScenarioSelect | ProfessionSelect | CharacterCreation
        | CharacterConfirm | Custom(_) => InputContextId::MainMenu,
        DevSpawnPanel => InputContextId::Inventory,
        SettingsMenu => InputContextId::Settings,
        Inventory | ItemExamine => InputContextId::Inventory,
        CraftingMenu => InputContextId::CraftingMenu,
        CharacterSheet => InputContextId::CharacterSheet,
        ExamineLook => InputContextId::ExamineLook,
        Dialog => InputContextId::Dialog,
        DirectionSelect => InputContextId::DirectionSelect,
        TextInput => InputContextId::TextInput,
        QuantityInput => InputContextId::QuantityInput,
        PauseMenu => InputContextId::PauseMenu,
        VehicleInteraction => InputContextId::VehicleInteraction,
        Gameplay => InputContextId::Gameplay,
    };
    stack.replace_top(ctx);
}
