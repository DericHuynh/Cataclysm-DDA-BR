//! Bevy systems for the `cdda_input` plugin.
//!
//! Systems:
//!
//! 1. **`handle_raw_input`** — reads `KeyboardInput` messages, resolves keys
//!    against the active input context, and fires `InputAction` messages.
//!    Also checks for `RebindCapture` — if a rebind is in progress, the
//!    next non-modifier key press is captured and applied to `ContextBindings`.

use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Res, ResMut, Resource};
use bevy_input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy_input::{ButtonInput, ButtonState};

use crate::actions::{ActionSource, GameAction, InputAction};
use crate::bindings::{ContextBindings, KeyChord};
use crate::context::{InputContextId, InputContextStack};

// ---------------------------------------------------------------------------
// RebindCapture
// ---------------------------------------------------------------------------

/// Controls whether `handle_raw_input` is in rebind-capture mode.
///
/// When `Some`, the **next** non-modifier key press is captured and
/// used to rebind the specified action in the specified context.
///
/// Set by the settings screen when the user presses Enter on a
/// keybinding row.  Cleared automatically after one capture.
#[derive(Resource, Debug, Clone, Default)]
pub struct RebindCapture(pub Option<RebindCaptureInner>);

/// The inner data for an in-progress rebind capture.
#[derive(Debug, Clone)]
pub struct RebindCaptureInner {
    /// The context whose bindings will be modified.
    pub context: InputContextId,
    /// The action to rebind.
    pub action: GameAction,
}

// ---------------------------------------------------------------------------
// handle_raw_input
// ---------------------------------------------------------------------------

/// Reads `KeyboardInput` messages, resolves them against the active input
/// context, and writes semantic `InputAction` messages.
///
/// If `RebindCapture` is `Some`, the first non-modifier key press is
/// captured instead of being resolved normally — the chord is written
/// into `ContextBindings` via `rebind()`.
///
/// ## Design
///
/// - Uses `logical_key` for text input (layout-correct, cross-platform).
///   The old `key_to_printable_char(KeyCode, shift)` QWERTY table has been
///   **deleted** — `KeyboardInput::logical_key` replaces it entirely.
///
/// - Uses `key_code` (physical key position) for binding resolution.
///   Modifier state (`shift`/`ctrl`/`alt`) is read from `ButtonInput<KeyCode>`
///   because Bevy 0.18.1 does not expose `modifiers` directly on `KeyboardInput`.
///
/// - Ignores released keys and repeats.
pub fn handle_raw_input(
    mut keyboard_messages: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    context_stack: Res<InputContextStack>,
    mut bindings: ResMut<ContextBindings>,
    mut action_writer: MessageWriter<InputAction>,
    mut rebind_capture: ResMut<RebindCapture>,
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

        // ── Rebind capture mode: intercept the next key ───────────
        if let Some(capture) = rebind_capture.0.as_mut() {
            // Skip modifier-only keys (Shift, Ctrl, Alt)
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
                    // Escape cancels rebind without changing anything
                    rebind_capture.0.take();
                    action_writer
                        .write(InputAction::new(GameAction::Cancel, ActionSource::Keyboard));
                    continue;
                }
                _ => {
                    let chord = KeyChord {
                        key: ev.key_code,
                        shift,
                        ctrl,
                        alt,
                    };
                    bindings.rebind(capture.context.clone(), capture.action.clone(), chord);
                    tracing::info!(
                        "Rebound {:?} in {:?} to chord {:?}",
                        capture.action,
                        capture.context,
                        chord,
                    );
                    rebind_capture.0.take();
                    // Do NOT write Confirm here — doing so would cause
                    // settings::handle_confirm to immediately start a new
                    // rebind capture on the same row, creating an infinite
                    // loop.  detect_rebind_complete detects completion by
                    // polling rebind_capture instead.
                    continue;
                }
            }
        }

        // ── Text input context: use logical_key directly ──────────
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

        // ── All other contexts: resolve via binding table ─────────
        if let Some(action) = bindings.resolve(&active_context, ev.key_code, shift, ctrl, alt) {
            action_writer.write(InputAction::new(action, ActionSource::Keyboard));
        }
    }
}
