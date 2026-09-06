//! Shared UI theme / colour palette.
//!
//! Every render module reads `Res<UiTheme>` instead of hard-coding colours.
//! Three built-in presets (Blue, Green, Amber) are switchable via Settings.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Fixed colours (same across all presets)
// ---------------------------------------------------------------------------

pub const BG: Color = Color::srgb(0.025, 0.065, 0.070);
pub const PANEL_BG: Color = Color::srgb(0.045, 0.095, 0.100);
pub const HEADER_BG: Color = Color::srgb(0.075, 0.140, 0.145);
pub const ITEM_BG: Color = Color::srgb(0.040, 0.085, 0.090);
pub const DIVIDER: Color = Color::srgb(0.23, 0.36, 0.34);
pub const TEXT_BRIGHT: Color = Color::srgb(0.88, 0.88, 0.75);
pub const TEXT_DIM: Color = Color::srgb(0.57, 0.68, 0.63);
pub const TEXT_GREEN: Color = Color::srgb(0.35, 0.85, 0.40);
pub const TEXT_YELLOW: Color = Color::srgb(0.90, 0.80, 0.20);
pub const TEXT_RED: Color = Color::srgb(0.90, 0.30, 0.30);
pub const TEXT_ORANGE: Color = Color::srgb(0.95, 0.55, 0.15);
pub const TEXT_ID: Color = Color::srgb(0.50, 0.65, 0.50);

// Menu / button palette (main_menu, dev_worldgen)
pub const MENU_BG: Color = Color::srgb(0.025, 0.065, 0.070);
pub const BUTTON_BG: Color = Color::srgb(0.055, 0.115, 0.120);
pub const BUTTON_FOCUS_BG: Color = Color::srgb(0.15, 0.32, 0.28);

// Panel variants
pub const SIDE_PANEL_BG: Color = Color::srgb(0.035, 0.080, 0.085);
pub const PANEL_HEADER_BG: Color = Color::srgb(0.075, 0.140, 0.145);
pub const SECTION_HEADER_BG: Color = Color::srgb(0.060, 0.120, 0.125);

// Tabs
pub const TAB_BG: Color = Color::srgb(0.045, 0.095, 0.100);
pub const TAB_TEXT_ACTIVE: Color = Color::srgb(0.83, 0.73, 0.44);
pub const SUBTAB_BG: Color = Color::srgb(0.035, 0.080, 0.085);
pub const SUBTAB_ACTIVE_BG: Color = Color::srgb(0.12, 0.25, 0.23);
pub const ZONE_HIGHLIGHT_BG: Color = Color::srgb(0.15, 0.30, 0.27);

// Rows
pub const ROW_ALT_BG: Color = Color::srgb(0.055, 0.110, 0.110);

// Filters
pub const FILTER_ACTIVE_BG: Color = Color::srgb(0.10, 0.23, 0.22);

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
            Self::Green => Color::srgb(0.45, 0.80, 0.70),
            Self::Amber => Color::srgb(0.90, 0.70, 0.20),
        }
    }

    /// Secondary accent — section labels, panel headers.
    pub fn accent2(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.85, 0.60, 0.15),
            Self::Green => Color::srgb(0.83, 0.73, 0.44),
            Self::Amber => Color::srgb(0.60, 0.80, 0.90),
        }
    }

    /// Background of the focused/selected item row.
    pub fn item_focus_bg(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.12, 0.35, 0.55),
            Self::Green => Color::srgb(0.12, 0.30, 0.27),
            Self::Amber => Color::srgb(0.35, 0.25, 0.08),
        }
    }

    /// Active tab background.
    pub fn tab_active_bg(self) -> Color {
        match self {
            Self::Blue => Color::srgb(0.14, 0.24, 0.38),
            Self::Green => Color::srgb(0.09, 0.23, 0.22),
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
        Self {
            preset: ThemePreset::Green,
        }
    }
}

impl UiTheme {
    pub fn accent(&self) -> Color {
        self.preset.accent()
    }
    pub fn accent2(&self) -> Color {
        self.preset.accent2()
    }
    pub fn item_focus_bg(&self) -> Color {
        self.preset.item_focus_bg()
    }
    pub fn tab_active_bg(&self) -> Color {
        self.preset.tab_active_bg()
    }
    pub fn label_color(&self) -> Color {
        self.preset.label_color()
    }
}
