//! Shared illustrated-screen geometry and presentation-only motion.
//! Artwork is contained, never cropped or placed underneath interactive content.
use super::theme;
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub enum ScreenRegion {
    MenuContent,
    MenuArt,
    LoadingContent,
    LoadingArt,
}

/// Compute in logical UI pixels, so window resizing and interface scaling agree.
pub fn region_rect(region: ScreenRegion, viewport: Vec2) -> Rect {
    let margin = (viewport.x * 0.045).clamp(16., 64.);
    let width = (viewport.x - 2. * margin).max(1.).min(1440.);
    let left = (viewport.x - width) * 0.5;
    let height = (viewport.y - 2. * margin).max(1.);
    let compact = viewport.x < 760.;
    match region {
        ScreenRegion::MenuContent => {
            let w = if compact {
                width.min(460.)
            } else {
                (width * 0.36).min(440.)
            };
            Rect::from_corners(
                Vec2::new(if compact { (viewport.x - w) / 2. } else { left }, margin),
                Vec2::new(
                    if compact {
                        (viewport.x + w) / 2.
                    } else {
                        left + w
                    },
                    margin + height,
                ),
            )
        }
        ScreenRegion::MenuArt => {
            let content = region_rect(ScreenRegion::MenuContent, viewport);
            let available = Vec2::new((left + width - content.max.x - 32.).max(0.), height);
            let size = contain(available, 1365. / 1024.);
            let center = Vec2::new((content.max.x + 32. + left + width) / 2., viewport.y / 2.);
            Rect::from_center_size(center, if compact { Vec2::ZERO } else { size })
        }
        ScreenRegion::LoadingContent => {
            let w = width.min(760.);
            // Reserve space for controls even at 150% UI scale. Extra diagnostics scroll.
            let h = (height * 0.38).clamp(190., 240.).min(height);
            Rect::from_center_size(
                Vec2::new(viewport.x / 2., viewport.y - margin - h / 2.),
                Vec2::new(w, h),
            )
        }
        ScreenRegion::LoadingArt => {
            let panel = region_rect(ScreenRegion::LoadingContent, viewport);
            let available = Vec2::new(width, (panel.min.y - margin - 16.).max(0.));
            Rect::from_center_size(
                Vec2::new(viewport.x / 2., margin + available.y / 2.),
                contain(available, 1920. / 1460.),
            )
        }
    }
}
fn contain(available: Vec2, ratio: f32) -> Vec2 {
    let width = available.x.min(available.y * ratio);
    Vec2::new(width, width / ratio)
}

pub fn layout(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    options: Option<Res<super::settings::SettingsState>>,
    mut regions: Query<(&ScreenRegion, &mut Node)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height()) / scale.map_or(1., |s| s.0);
    for (region, mut node) in &mut regions {
        let mut rect = region_rect(*region, viewport);
        if matches!(region, ScreenRegion::MenuContent)
            && options.as_ref().is_some_and(|o| !o.menu_art)
        {
            rect = Rect::from_center_size(Vec2::new(viewport.x / 2., rect.center().y), rect.size());
        }
        let mut next = node.clone();
        next.position_type = PositionType::Absolute;
        next.left = px(rect.min.x);
        next.top = px(rect.min.y);
        next.width = px(rect.width());
        next.height = px(rect.height());
        node.set_if_neq(next);
    }
}

/// A target set by keyboard selection; hover also lights controls without stealing focus.
#[derive(Component, Default)]
#[require(BackgroundColor)]
pub struct AccentMotion {
    pub selected: bool,
    strength: f32,
}
pub fn animate_accents(
    palette: Res<theme::UiTheme>,
    time: Res<Time>,
    mut buttons: Query<(&mut AccentMotion, &Interaction, &mut BackgroundColor)>,
) {
    for (mut motion, interaction, mut color) in &mut buttons {
        let target = if motion.selected || *interaction != Interaction::None {
            1.
        } else {
            0.
        };
        let next =
            motion.strength + (target - motion.strength) * (1. - (-14. * time.delta_secs()).exp());
        let strength = if (next - target).abs() < 0.001 {
            target
        } else {
            next
        };
        if motion.strength != strength {
            motion.strength = strength;
        }
        color.set_if_neq(BackgroundColor(
            palette
                .color(theme::Role::Canvas)
                .mix(&palette.color(theme::Role::Selection), motion.strength),
        ));
    }
}

/// Artwork fades from black. No perpetual zoom or cropping of the source illustration.
#[derive(Component, Default)]
pub struct ArtReveal(pub f32);
pub fn reveal_art(
    mut commands: Commands,
    time: Res<Time>,
    mut art: Query<(Entity, &mut ArtReveal, &mut ImageNode)>,
) {
    for (entity, mut reveal, mut image) in &mut art {
        reveal.0 = (reveal.0 + time.delta_secs() / 1.2).min(1.);
        image.color = Color::WHITE.with_alpha(1. - (1. - reveal.0).powi(3));
        if reveal.0 >= 1. {
            commands.entity(entity).remove::<ArtReveal>();
        }
    }
}
