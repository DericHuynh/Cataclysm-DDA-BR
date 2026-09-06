//! One semantic palette for all UI views. Source artwork and terrain keep their own colors.
use bevy::prelude::*;

// Signals retain their meaning across themes.
pub const TEXT_GREEN: Color = Color::srgb(0.35, 0.85, 0.40);
pub const TEXT_YELLOW: Color = Color::srgb(0.90, 0.80, 0.20);
pub const TEXT_RED: Color = Color::srgb(0.90, 0.30, 0.30);
pub const TEXT_ORANGE: Color = Color::srgb(0.95, 0.55, 0.15);

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Reflect)]
pub enum ThemePreset {
    Blue,
    #[default]
    Green,
    Amber,
}
impl ThemePreset {
    // Preserve persisted indices.
    pub const ALL: [Self; 3] = [Self::Blue, Self::Green, Self::Amber];
    pub fn label(self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Green => "Green",
            Self::Amber => "Amber",
        }
    }
}
#[derive(Resource, Clone, Debug, Default)]
pub struct UiTheme {
    pub preset: ThemePreset,
}
impl UiTheme {
    pub fn accent(&self) -> Color {
        self.color(Role::Accent)
    }
    pub fn accent2(&self) -> Color {
        self.color(Role::Accent)
    }
    pub fn item_focus_bg(&self) -> Color {
        self.color(Role::Selection)
    }
    pub fn tab_active_bg(&self) -> Color {
        self.color(Role::Selection)
    }
    pub fn label_color(&self) -> Color {
        self.color(Role::Accent)
    }
}

/// Semantic roles shared by every view. Map/terrain colors are domain data, not UI roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Canvas,
    Surface,
    Raised,
    Alternate,
    Selection,
    Border,
    Text,
    Muted,
    Accent,
    Positive,
    Warning,
    Danger,
    Hot,
    Transparent,
}
impl UiTheme {
    pub fn color(&self, role: Role) -> Color {
        let accent = match self.preset {
            ThemePreset::Blue => Color::srgb(0.55, 0.73, 0.86),
            ThemePreset::Green => Color::srgb(0.62, 0.76, 0.55),
            ThemePreset::Amber => Color::srgb(0.82, 0.66, 0.39),
        };
        let text = match self.preset {
            ThemePreset::Blue => Color::srgb(0.82, 0.87, 0.90),
            ThemePreset::Green => Color::srgb(0.85, 0.88, 0.77),
            ThemePreset::Amber => Color::srgb(0.86, 0.83, 0.75),
        };
        // Mix UI surfaces in sRGB; mixing from linear black lifts dark surfaces excessively.
        let canvas = Color::srgb(0., 0., 0.);
        match role {
            // Original artwork has a baked black backdrop; keep one canvas across all views.
            Role::Canvas => canvas,
            Role::Surface => canvas.mix(&accent, 0.065),
            Role::Raised => canvas.mix(&accent, 0.12),
            Role::Alternate => canvas.mix(&accent, 0.09),
            Role::Selection => canvas.mix(&accent, 0.24),
            Role::Border => canvas.mix(&accent, 0.32),
            Role::Text => text,
            Role::Muted => canvas.mix(&text, 0.65),
            Role::Accent => accent,
            Role::Positive => TEXT_GREEN,
            Role::Warning => TEXT_YELLOW,
            Role::Danger => TEXT_RED,
            Role::Hot => TEXT_ORANGE,
            Role::Transparent => Color::NONE,
        }
    }
}

/// Persistent style intent, separate from Bevy's resolved presentation components.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(TextColor)]
pub struct TextPaint(pub Role);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(BackgroundColor)]
pub struct SurfacePaint(pub Role);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(BorderColor)]
pub struct BorderPaint(pub Role);

/// Refresh retained chrome on theme changes and paint newly spawned/reassigned roles.
/// Presenters resolve virtual-row colors from the same palette; no catalog rebuild is needed.
pub fn apply_palette(
    theme: Res<UiTheme>,
    mut texts: Query<(Ref<TextPaint>, &mut TextColor)>,
    mut surfaces: Query<(Ref<SurfacePaint>, &mut BackgroundColor)>,
    mut borders: Query<(Ref<BorderPaint>, &mut BorderColor)>,
) {
    for (paint, mut color) in &mut texts {
        if theme.is_changed() || paint.is_changed() {
            color.set_if_neq(TextColor(theme.color(paint.0)));
        }
    }
    for (paint, mut color) in &mut surfaces {
        if theme.is_changed() || paint.is_changed() {
            color.set_if_neq(BackgroundColor(theme.color(paint.0)));
        }
    }
    for (paint, mut color) in &mut borders {
        if theme.is_changed() || paint.is_changed() {
            color.set_if_neq(BorderColor::all(theme.color(paint.0)));
        }
    }
}
