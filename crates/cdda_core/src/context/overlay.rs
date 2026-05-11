//! Overlay stack — stacked modal overlays for confirmations, activity
//! progress, and interruptions.  The topmost overlay receives input first;
//! all gameplay input is suppressed while any overlay is active.

use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

/// A modal overlay displayed on top of the current screen.
#[derive(Debug, Clone)]
pub enum Overlay {
    /// Simple confirmation dialog with accept/cancel.
    Confirm { title: String, message: String },

    /// Activity progress display.  Blocks movement input so the player
    /// cannot walk away while crafting, reading, building, etc.
    ActivityProgress {
        activity_label: String,
        progress_pct: u32,
    },

    /// Interruption: something happened while the player was busy.
    /// Player must choose to stop or continue.
    Interrupt { title: String, message: String },
}

// ---------------------------------------------------------------------------
// OverlayStack resource
// ---------------------------------------------------------------------------

/// Stack of pending overlays.  The last element is the topmost (active) one.
/// When non-empty, `input_blocked` is `true` and gameplay movement/action
/// input is suppressed.  Only overlay-specific keys (Esc to dismiss, Enter
/// to confirm, etc.) are processed.
#[derive(Resource, Default, Debug, Clone)]
pub struct OverlayStack {
    pub stack: Vec<Overlay>,
    pub input_blocked: bool,
}

impl OverlayStack {
    /// Push an overlay onto the stack.
    pub fn push(&mut self, overlay: Overlay) {
        self.stack.push(overlay);
        self.input_blocked = true;
    }

    /// Pop the topmost overlay.
    pub fn pop(&mut self) -> Option<Overlay> {
        let result = self.stack.pop();
        self.input_blocked = !self.stack.is_empty();
        result
    }

    /// Peek at the top overlay without removing it.
    pub fn top(&self) -> Option<&Overlay> {
        self.stack.last()
    }

    /// Is the overlay stack empty?
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Push an `ActivityProgress` overlay when a `PlayerActivity` becomes active.
/// Runs after `start_pending_activities`.
pub fn sync_activity_overlay(world: &mut World) {
    use crate::activity::components::{ActivityPhase, PlayerActivity};

    let mut to_push: Vec<(Entity, String, u32)> = Vec::new();
    {
        let mut q = world.query::<(Entity, &PlayerActivity)>();
        for (entity, act) in q.iter(world) {
            if act.phase == ActivityPhase::Active {
                let pct = if act.moves_total > 0 {
                    ((act.moves_total - act.moves_left) as u32 * 100) / act.moves_total as u32
                } else {
                    0
                };
                to_push.push((entity, act.activity_type.0.clone(), pct));
            }
        }
    }

    let mut overlays = world.resource_mut::<OverlayStack>();
    // Only push if there isn't already an activity overlay.
    if !overlays
        .stack
        .iter()
        .any(|o| matches!(o, Overlay::ActivityProgress { .. }))
    {
        for (_, label, pct) in to_push {
            overlays.push(Overlay::ActivityProgress {
                activity_label: label,
                progress_pct: pct,
            });
        }
    }
}

/// Remove `ActivityProgress` overlays when no active `PlayerActivity` exists.
pub fn cleanup_activity_overlay(world: &mut World) {
    use crate::activity::components::{ActivityPhase, PlayerActivity};

    let has_active = {
        let mut q = world.query::<&PlayerActivity>();
        q.iter(world).any(|a| a.phase == ActivityPhase::Active)
    };

    if !has_active {
        let mut overlays = world.resource_mut::<OverlayStack>();
        overlays
            .stack
            .retain(|o| !matches!(o, Overlay::ActivityProgress { .. }));
        overlays.input_blocked = !overlays.stack.is_empty();
    }
}

// ---------------------------------------------------------------------------
// Input guard
// ---------------------------------------------------------------------------

/// Returns `true` if gameplay input should be blocked because an overlay
/// is active.
pub fn is_input_blocked(world: &World) -> bool {
    world.resource::<OverlayStack>().input_blocked
}

/// Pop the topmost overlay from the stack.
pub fn pop_overlay(world: &mut World) {
    let mut overlays = world.resource_mut::<OverlayStack>();
    overlays.pop();
}

/// Drains `GameAction::Cancel` messages and pops the top overlay when
/// one is active.  Registered as an exclusive system so it can call
/// `pop_overlay` with `&mut World` access.
pub fn handle_overlay_cancel(world: &mut World) {
    use crate::input::{GameAction, InputAction};
    use bevy_ecs::message::Messages;

    if !world.resource::<OverlayStack>().input_blocked {
        return;
    }

    let mut messages = world.resource_mut::<Messages<InputAction>>();
    messages.update();
    let has_cancel = messages
        .drain()
        .any(|a| matches!(a.action, GameAction::Cancel));
    if has_cancel {
        pop_overlay(world);
    }
}
