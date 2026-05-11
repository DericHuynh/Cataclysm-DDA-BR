//! Context actions — the authoritative list of available actions for the
//! current UI context.
//!
//! Populated via `OnEnter` state transitions (firing instantly, once per
//! screen entry) and `Changed<T>` resource watchers for dynamic updates.
//! Renderers read the resource to display key hints — no per-frame systems.

use bevy_ecs::prelude::*;

use crate::input::BindableAction;

// ---------------------------------------------------------------------------
// ContextAction
// ---------------------------------------------------------------------------

/// A single action available in the current UI context.
#[derive(Debug, Clone)]
pub struct ContextAction {
    /// Human-readable label (e.g. `"drop"`, `"wield"`, `"resume craft"`).
    pub label: String,
    /// The bindable action that triggers this behaviour.
    pub action: BindableAction,
}

// ---------------------------------------------------------------------------
// ContextActions resource
// ---------------------------------------------------------------------------

/// The list of actions available in the currently-active UI context.
///
/// Populated on screen entry via `OnEnter` and updated on relevant resource
/// changes.  Renderers read this to display action hints.
#[derive(Resource, Debug, Clone, Default)]
pub struct ContextActions {
    pub actions: Vec<ContextAction>,
}

impl ContextActions {
    /// Push an action onto the list.
    pub fn push(&mut self, label: impl Into<String>, action: BindableAction) {
        self.actions.push(ContextAction {
            label: label.into(),
            action,
        });
    }

    /// Clear and populate from a static list of (label, action) pairs.
    pub fn populate(&mut self, entries: &[(&str, BindableAction)]) {
        self.actions.clear();
        for (label, action) in entries {
            self.push(*label, *action);
        }
    }
}

// ---------------------------------------------------------------------------
// Static action definitions per screen
// ---------------------------------------------------------------------------
