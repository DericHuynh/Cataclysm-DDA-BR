//! Shared UI theme / colour palette.
//!
//! Every render module reads `Res<UiTheme>` instead of hard-coding colours.
//! Three built-in presets (Blue, Green, Amber) are switchable via Settings.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Fixed colours (same across all presets)
// ---------------------------------------------------------------------------

pub const BG: Color = Color::srgb(0.04, 0.04, 0.06);
pub const PANEL_BG: Color = Color::srgb(0.07, 0.07, 0.10);
pub const HEADER_BG: Color = Color::srgb(0.10, 0.10, 0.14);
pub const ITEM_BG: Color = Color::srgb(0.06, 0.06, 0.09);
pub const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);
pub const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
pub const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
pub const TEXT_GREEN: Color = Color::srgb(0.35, 0.85, 0.40);
pub const TEXT_YELLOW: Color = Color::srgb(0.90, 0.80, 0.20);
pub const TEXT_RED: Color = Color::srgb(0.90, 0.30, 0.30);
pub const TEXT_ORANGE: Color = Color::srgb(0.95, 0.55, 0.15);
pub const TEXT_ID: Color = Color::srgb(0.50, 0.65, 0.50);

// Menu / button palette (main_menu, dev_worldgen)
pub const MENU_BG: Color = Color::srgb(0.05, 0.05, 0.07);
pub const BUTTON_BG: Color = Color::srgb(0.08, 0.08, 0.10);
pub const BUTTON_FOCUS_BG: Color = Color::srgb(0.25, 0.55, 0.15);

// Panel variants
pub const SIDE_PANEL_BG: Color = Color::srgb(0.05, 0.05, 0.08);
pub const PANEL_HEADER_BG: Color = Color::srgb(0.09, 0.09, 0.13);
pub const SECTION_HEADER_BG: Color = Color::srgb(0.08, 0.08, 0.12);

// Tabs
pub const TAB_BG: Color = Color::srgb(0.08, 0.08, 0.14);
pub const TAB_TEXT_ACTIVE: Color = Color::srgb(0.40, 0.70, 0.90);
pub const SUBTAB_BG: Color = Color::srgb(0.06, 0.06, 0.12);
pub const SUBTAB_ACTIVE_BG: Color = Color::srgb(0.10, 0.18, 0.28);
pub const ZONE_HIGHLIGHT_BG: Color = Color::srgb(0.15, 0.30, 0.45);

// Rows
pub const ROW_ALT_BG: Color = Color::srgb(0.09, 0.09, 0.12);

// Filters
pub const FILTER_ACTIVE_BG: Color = Color::srgb(0.08, 0.18, 0.30);

// Inventory-specific
pub const ITEM_CRAFT_BG: Color = Color::srgb(0.18, 0.12, 0.05);
pub const TEXT_CRAFT: Color = Color::srgb(0.85, 0.65, 0.20);
pub const ICON_BG: Color = Color::srgb(0.12, 0.12, 0.16);
pub const ICON_TEXT: Color = Color::srgb(0.90, 0.85, 0.25);

// ---------------------------------------------------------------------------
// Theme preset
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Reflect)]
pub enum ThemePreset {
    #[default]
    Blue,
    Green,
    Amber,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 3] = [ThemePreset::Blue, ThemePreset::Green, ThemePreset::Amber];

    pub fn label(self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Green => "Green",
            Self::Amber => "Amber",
        }
    }

    /// Primary accent — titles, header text.
    pub fn accent(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.30, 0.70, 1.00),
            Self::Green => Color::srgb(0.40, 0.85, 0.40),
            Self::Amber => Color::srgb(0.90, 0.70, 0.20),
        }
    }

    /// Secondary accent — section labels, panel headers.
    pub fn accent2(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.85, 0.60, 0.15),
            Self::Green => Color::srgb(0.85, 0.75, 0.20),
            Self::Amber => Color::srgb(0.60, 0.80, 0.90),
        }
    }

    /// Background of the focused/selected item row.
    pub fn item_focus_bg(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.12, 0.35, 0.55),
            Self::Green => Color::srgb(0.10, 0.35, 0.15),
            Self::Amber => Color::srgb(0.35, 0.25, 0.08),
        }
    }

    /// Active tab background.
    pub fn tab_active_bg(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.14, 0.24, 0.38),
            Self::Green => Color::srgb(0.10, 0.25, 0.12),
            Self::Amber => Color::srgb(0.28, 0.20, 0.06),
        }
    }

    /// Label / field-name colour.
    pub fn label_color(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.50, 0.75, 0.90),
            Self::Green => Color::srgb(0.55, 0.80, 0.55),
            Self::Amber => Color::srgb(0.80, 0.65, 0.40),
        }
    }
}

// ---------------------------------------------------------------------------
// UiTheme resource
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Debug)]
pub struct UiTheme {
    pub preset: ThemePreset,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self { preset: ThemePreset::Blue }
    }
}

impl UiTheme {
    pub fn accent(&self) -> Color { self.preset.accent() }
    pub fn accent2(&self) -> Color { self.preset.accent2() }
    pub fn item_focus_bg(&self) -> Color { self.preset.item_focus_bg() }
    pub fn tab_active_bg(&self) -> Color { self.preset.tab_active_bg() }
    pub fn label_color(&self) -> Color { self.preset.label_color() }
}
