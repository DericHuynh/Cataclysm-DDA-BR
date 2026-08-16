//! Nested-menu (`SubStates`) types for screens with tabbed depth.
//!
//! Bevy's idiomatic answer to "deeply nested menus" is [`SubStates`]: a top-level
//! screen (`Ctx`) owns a set of tab states that only exist while that screen is
//! active. Systems schedule against the tab state (`OnEnter` / `in_state`),
//! and entities are scoped to it via `DespawnOnExit` so Bevy cleans up the
//! previous tab automatically. This crate keeps them headless so `cdda_context`
//! stays free of any `bevy_ui` dependency.
//!
//! The regular (non-tabbed) screens continue to use `Ctx` + `ContextStack`.

use bevy_state::prelude::*;

use crate::ctx::Ctx;

/// Tabs of the Settings screen. Exists only while `Ctx::SettingsMenu` is active.
///
/// [`NextState::<SettingsTab>`] is used to switch tabs; each tab's systems run in
/// `OnEnter` / `in_state` and its UI subtree is tagged `DespawnOnExit(SettingsTab)`
/// so switching tabs scopes cleanup automatically.
#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(Ctx = Ctx::SettingsMenu)]
pub enum SettingsTab {
    /// General gameplay options.
    #[default]
    General,
    /// Graphics / rendering options.
    Graphics,
    /// Audio options.
    Sound,
    /// Interface / theme options.
    Interface,
    /// Key remapping.
    Keybindings,
}

impl SettingsTab {
    /// All tabs in display order.
    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::General,
            SettingsTab::Graphics,
            SettingsTab::Sound,
            SettingsTab::Interface,
            SettingsTab::Keybindings,
        ]
    }

    /// Human-readable tab heading.
    pub fn label(&self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Graphics => "Graphics",
            SettingsTab::Sound => "Sound",
            SettingsTab::Interface => "Interface",
            SettingsTab::Keybindings => "Keybindings",
        }
    }

    /// The next tab (wrapping), for `NavigateNextTab`.
    pub fn next(self) -> Self {
        let all = Self::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    /// The previous tab (wrapping), for `NavigatePrevTab`.
    pub fn prev(self) -> Self {
        let all = Self::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + all.len() - 1) % all.len()]
    }
}
