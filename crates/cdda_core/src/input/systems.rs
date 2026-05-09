//! Input systems for the `cdda_input` plugin.
//!
//! Two complementary systems form the input pipeline:
//!
//! 1. **`handle_raw_input`** (PreUpdate) — handles special cases that cannot
//!    go through leafwing:
//!    - `RebindCapture` mode: intercepts the next non-modifier key and rewrites
//!      the binding in `ContextInputMaps`, then triggers `sync_leafwing_input_map`.
//!    - `TextInput` context: uses `KeyboardInput::logical_key` to generate
//!      `TextChar` / `TextBackspace` / `TextCommit` / `TextCancel` actions.
//!
//! 2. **`bridge_actionstate`** (Update) — reads `ActionState<BindableAction>`
//!    produced by leafwing's `InputManagerPlugin`, converts just-pressed
//!    actions to `InputAction` messages, and writes them to the message queue
//!    that downstream systems (`handle_navigation_input`, game logic) consume.

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Component, Query, Res, ResMut, Resource, With};
use bevy_input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy_input::{ButtonInput, ButtonState};
use leafwing_input_manager::prelude::{ActionState, ButtonlikeChord, InputMap, ModifierKey};
use leafwing_input_manager::user_input::Buttonlike;

use crate::context::ctx::Ctx;
use crate::input::actions::{ActionSource, BindableAction, GameAction, InputAction};
use crate::input::bindings::ContextInputMaps;
use crate::input::context::{InputContextId, InputContextStack};
use bevy_state::prelude::State;

// ---------------------------------------------------------------------------
// RebindCapture
// ---------------------------------------------------------------------------

/// Controls whether `handle_raw_input` is in rebind-capture mode.
///
/// When `pending` is `Some`, the next non-modifier key press is captured and
/// used to rebind the specified `BindableAction` in the specified context.
/// `just_captured` is set to `true` for one Update frame after capture so
/// the bridge can skip processing the captured key as a normal action.
#[derive(Resource, Debug, Clone, Default)]
pub struct RebindCapture {
    pub pending: Option<RebindCaptureInner>,
    /// True for exactly one Update frame after a rebind was captured.
    /// Prevents the bridge from dispatching the captured key as a normal
    /// action in the same frame the rebind was registered.
    pub just_captured: bool,
}

impl RebindCapture {
    /// Returns true when a rebind is actively waiting for a key.
    pub fn is_capturing(&self) -> bool {
        self.pending.is_some()
    }
}

/// Inner data for an in-progress rebind capture.
#[derive(Debug, Clone)]
pub struct RebindCaptureInner {
    /// The context whose bindings will be modified.
    pub context: InputContextId,
    /// The action to rebind.
    pub action: BindableAction,
}

// ---------------------------------------------------------------------------
// GlobalInputEntity marker
// ---------------------------------------------------------------------------

/// Marker component for the entity that holds `InputMap<BindableAction>` and
/// `ActionState<BindableAction>` for the current active context.
#[derive(Component)]
pub struct GlobalInputEntity;

// ---------------------------------------------------------------------------
// handle_raw_input — special-case input handler
// ---------------------------------------------------------------------------

/// Handles rebind capture mode and text input context.
///
/// All other input is handled by leafwing's `InputManagerPlugin` and then
/// forwarded to game systems via `bridge_actionstate`.
///
/// Runs in `PreUpdate` to see raw `KeyboardInput` events before leafwing
/// processes them in the same schedule.
pub fn handle_raw_input(
    mut keyboard_messages: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    context_stack: Res<InputContextStack>,
    mut context_maps: ResMut<ContextInputMaps>,
    mut action_writer: MessageWriter<InputAction>,
    mut rebind_capture: ResMut<RebindCapture>,
    mut input_map_query: Query<&mut InputMap<BindableAction>, With<GlobalInputEntity>>,
    ctx_state: Res<State<Ctx>>,
) {
    let active_context = context_stack.top();
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

    for ev in keyboard_messages.read() {
        if ev.state == ButtonState::Released {
            continue;
        }
        if ev.repeat {
            continue;
        }

        // ── Rebind capture mode ──────────────────────────────────────────
        if let Some(capture) = rebind_capture.pending.as_ref() {
            match ev.key_code {
                KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::ControlLeft
                | KeyCode::ControlRight
                | KeyCode::AltLeft
                | KeyCode::AltRight => {
                    continue;
                }
                KeyCode::Escape => {
                    rebind_capture.pending = None;
                    action_writer
                        .write(InputAction::new(GameAction::Cancel, ActionSource::Keyboard));
                    continue;
                }
                _ => {
                    let input = key_to_user_input(ev.key_code, shift, ctrl, alt);
                    let ctx = capture.context;
                    let action = capture.action;
                    context_maps.rebind(ctx, action, input.clone());
                    tracing::info!("Rebound {:?} in {:?} to {:?}", action, ctx, input);

                    // Refresh the entity's InputMap for the current context
                    let ctx_id = screen_to_input_ctx(*ctx_state.get());
                    let merged = context_maps.merged_for(&ctx_id);
                    if let Ok(mut map) = input_map_query.single_mut() {
                        *map = merged;
                    }

                    rebind_capture.pending = None;
                    rebind_capture.just_captured = true;
                    continue;
                }
            }
        }

        // ── Text input context: use logical_key ──────────────────────────
        if active_context == InputContextId::TextInput {
            match &ev.logical_key {
                Key::Character(ch) if !ch.chars().any(|c: char| c.is_control()) => {
                    action_writer.write(InputAction::new(
                        GameAction::TextChar(ch.to_string()),
                        ActionSource::Keyboard,
                    ));
                    continue;
                }
                Key::Backspace => {
                    action_writer.write(InputAction::new(
                        GameAction::TextBackspace,
                        ActionSource::Keyboard,
                    ));
                    continue;
                }
                Key::Enter => {
                    action_writer.write(InputAction::new(
                        GameAction::TextCommit,
                        ActionSource::Keyboard,
                    ));
                    continue;
                }
                Key::Escape => {
                    action_writer.write(InputAction::new(
                        GameAction::TextCancel,
                        ActionSource::Keyboard,
                    ));
                    continue;
                }
                _ => {}
            }
        }

        // All other contexts: leafwing bridge handles it (bridge_actionstate)
    }
}

// ---------------------------------------------------------------------------
// bridge_actionstate — leafwing → InputAction messages
// ---------------------------------------------------------------------------

/// Reads `ActionState<BindableAction>` updated by leafwing each frame and
/// emits `InputAction` messages for every just-pressed action.
///
/// Skipped when:
/// - A rebind is in progress or was just completed (`RebindCapture`).
/// - The active context is `TextInput` (handled by `handle_raw_input`).
///
/// Runs in `Update`, after leafwing's `PreUpdate` action-state update and
/// after `handle_raw_input` sets `just_captured`.
pub fn bridge_actionstate(
    query: Query<&ActionState<BindableAction>, With<GlobalInputEntity>>,
    context_stack: Res<InputContextStack>,
    rebind_capture: Res<RebindCapture>,
    mut action_writer: MessageWriter<InputAction>,
) {
    if rebind_capture.is_capturing() || rebind_capture.just_captured {
        return;
    }
    if context_stack.top() == InputContextId::TextInput {
        return;
    }

    let Ok(state) = query.single() else {
        return;
    };

    for action in BindableAction::all() {
        if state.just_pressed(&action) {
            action_writer.write(InputAction::new(
                action.to_game_action(),
                ActionSource::Keyboard,
            ));
        }
    }
}

/// Clears the `just_captured` flag set by `handle_raw_input` after a rebind.
/// Must run in `Update` AFTER `bridge_actionstate`.
pub fn clear_rebind_flag(mut rebind_capture: ResMut<RebindCapture>) {
    rebind_capture.just_captured = false;
}

// ---------------------------------------------------------------------------
// sync_leafwing_input_map — keep InputMap in sync with active context
// ---------------------------------------------------------------------------

/// Updates the `InputMap<BindableAction>` on `GlobalInputEntity` whenever the
/// active `Ctx` state changes.  This keeps leafwing's action resolution
/// aligned with the current screen's bindings.
pub fn sync_leafwing_input_map(
    ctx_state: Res<State<Ctx>>,
    context_maps: Res<ContextInputMaps>,
    mut query: Query<&mut InputMap<BindableAction>, With<GlobalInputEntity>>,
) {
    if !ctx_state.is_changed() && !context_maps.is_changed() {
        return;
    }
    let ctx_id = screen_to_input_ctx(*ctx_state.get());
    let merged = context_maps.merged_for(&ctx_id);
    if let Ok(mut map) = query.single_mut() {
        *map = merged;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a raw key press + modifier state to a boxed buttonlike input.
fn key_to_user_input(key: KeyCode, shift: bool, ctrl: bool, alt: bool) -> Box<dyn Buttonlike> {
    match (ctrl, shift, alt) {
        (true, false, false) => Box::new(ButtonlikeChord::modified(ModifierKey::Control, key)),
        (false, true, false) => Box::new(ButtonlikeChord::modified(ModifierKey::Shift, key)),
        (false, false, true) => Box::new(ButtonlikeChord::modified(ModifierKey::Alt, key)),
        // Multiple modifiers or no modifier: plain key
        _ => Box::new(key),
    }
}

/// Mirror of `sync_input_context` (nav.rs) — maps a `Ctx` to its
/// `InputContextId`.  Kept private; external callers use `sync_input_context`.
pub(crate) fn screen_to_input_ctx(ctx: Ctx) -> InputContextId {
    use Ctx::*;
    match ctx {
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
    }
}
