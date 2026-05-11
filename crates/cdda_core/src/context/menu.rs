//! Menu components — data attached to UI entities for rendering.
//!
//! `cdda_render` reads these components to draw menu frames and items.
//! `cdda_screen` systems mutate them (e.g. `SelectedIndex` changes on
//! `NavigateDown`).

use bevy_ecs::prelude::Component;

/// Marks an entity as a menu list (container for menu items).
#[derive(Component, Debug, Clone)]
pub struct MenuList {
    /// Label shown at the top of the menu frame.
    pub title: String,
}

/// A single selectable item within a menu.
#[derive(Component, Debug, Clone)]
pub struct MenuItem {
    /// Display text for this item.
    pub label: String,
    /// Shortcut key hint (e.g. "a", "b", …). Empty = no shortcut.
    pub hotkey: String,
    /// Whether this item is currently selectable.
    ///
    /// TODO: convert this `bool` to an `Enabled` / `Disabled` tag component
    /// so it's archetype-queryable (consistent with the AGENTS.md tag pattern).
    pub enabled: bool,
}

/// Index of the currently selected item in a menu list.
///
/// The entity with `MenuList` carries this component, or the overlay entity
/// uses it for paginated lists.
#[derive(Component, Debug, Clone)]
pub struct SelectedIndex {
    pub index: usize,
    pub scroll_offset: usize,
}

impl Default for SelectedIndex {
    fn default() -> Self {
        Self {
            index: 0,
            scroll_offset: 0,
        }
    }
}
