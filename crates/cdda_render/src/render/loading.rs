//! Retained loading screen, backed by the same operation records as the terminal.
use super::{
    cinematic::{AccentMotion, ArtReveal, ScreenRegion},
    theme, UiFontHandle,
};
use bevy::prelude::*;
use bevy_state::prelude::State;
use cdda_components::progress::{OperationCommand, OperationReport, ReportLevel};
use cdda_sim::runtime::state::AppState;

#[derive(Component)]
pub struct LoadingScreen;
#[derive(Component)]
pub struct LoadingText;
#[derive(Component)]
pub struct LoadingHeading;
#[derive(Component)]
pub struct LoadingUnits;
#[derive(Component)]
pub struct LoadingTrack;
/// Report target is separate from animation state; an unknown total is never a percentage.
#[derive(Component, Default)]
#[require(UiTransform)]
pub struct LoadingBar {
    pub target: Option<f32>,
    pub stopped: bool,
    displayed: f32,
    phase: f32,
    stage: String,
}
#[derive(Component)]
pub struct RetryButton;

pub fn spawn(mut commands: Commands, assets: Res<AssetServer>, font: Res<UiFontHandle>) {
    commands.spawn((LoadingScreen, GlobalZIndex(1000), Node {
        width: percent(100), height: percent(100), position_type: PositionType::Absolute,
        overflow: Overflow::clip(), ..default()
    }, theme::SurfacePaint(theme::Role::Canvas))).with_children(|root| {
        root.spawn((ScreenRegion::LoadingArt, ArtReveal::default(),
            ImageNode { color: Color::WHITE.with_alpha(0.), ..ImageNode::new(assets.load("gfx/loading_screens/loading_img_hub.png")) }, Node::default()));
        root.spawn((ScreenRegion::LoadingContent, Node {
            flex_direction: FlexDirection::Column, row_gap: px(10), ..default()
        })).with_children(|panel| {
            panel.spawn((LoadingHeading, Text::new("PREPARING YOUR WORLD"), super::ui_font(&font.0, 23.), theme::TextPaint(theme::Role::Accent),
                Node { flex_shrink: 0., ..default() }));
            // Only diagnostics scroll: the progress track and controls stay anchored.
            panel.spawn((Node { flex_grow: 1., min_height: px(0), overflow: Overflow::scroll_y(), ..default() }, ScrollPosition::default()))
                .with_child((LoadingText, Text::new("Starting…"), super::ui_font(&font.0, 15.), theme::TextPaint(theme::Role::Text)));
            panel.spawn((LoadingUnits, Text::new("Working · total not yet known"), super::ui_font(&font.0, 12.), theme::TextPaint(theme::Role::Muted),
                Node { flex_shrink: 0., ..default() }));
            panel.spawn((LoadingTrack, Node { width: percent(100), height: px(2), flex_shrink: 0., overflow: Overflow::clip(), ..default() }, theme::SurfacePaint(theme::Role::Border)))
                .with_child((LoadingBar::default(), Node { width: percent(100), height: percent(100), ..default() }, theme::SurfacePaint(theme::Role::Accent)));
            panel.spawn(Node { column_gap: px(16), flex_shrink: 0., ..default() }).with_children(|buttons| {
                for (label, command, retry) in [("Retry", OperationCommand::Retry, true), ("Return to menu", OperationCommand::ReturnToMenu, false)] {
                    let mut button = buttons.spawn((Button, AccentMotion::default(), Node {
                        display: if retry { Display::None } else { Display::Flex },
                        padding: UiRect::axes(px(8), px(7)), ..default()
                    }));
                    if retry { button.insert(RetryButton); }
                    button.observe(move |mut click: On<Pointer<Click>>, mut writer: MessageWriter<OperationCommand>| {
                        writer.write(command); click.propagate(false);
                    }).with_child((Text::new(label), super::ui_font(&font.0, 14.), theme::TextPaint(theme::Role::Muted)));
                }
            });
        });
    });
}

pub fn update(
    report: Res<OperationReport>,
    mut texts: Query<
        (
            &mut Text,
            Has<LoadingText>,
            Has<LoadingHeading>,
            Has<LoadingUnits>,
        ),
        Or<(With<LoadingText>, With<LoadingHeading>, With<LoadingUnits>)>,
    >,
    mut bars: Query<&mut LoadingBar>,
    mut retry: Query<&mut Node, With<RetryButton>>,
) {
    if !report.is_changed() && !texts.iter().any(|(t, _, _, _)| t.0 == "Starting…") {
        return;
    }
    let Some(current) = &report.current else {
        return;
    };
    let mut lines = vec![
        current.stage.clone(),
        current.message.clone(),
        report.summary(),
    ];
    lines.extend(
        report
            .history
            .iter()
            .rev()
            .filter(|e| matches!(e.level, ReportLevel::Error | ReportLevel::Warning))
            .take(6)
            .map(ToString::to_string),
    );
    if report.failed() {
        lines.push("Fix the reported data and retry, or return to the menu.".into());
    }
    let detail = lines.join("\n");
    let units = if report.failed() {
        "Stopped · see diagnostics above".into()
    } else if report.cancelled {
        "Cancelled".into()
    } else if report.finished {
        "Complete".into()
    } else {
        current.units.filter(|(_, total)| *total > 0).map_or_else(
            || "Working · total not yet known".into(),
            |(done, total)| {
                format!(
                    "{done} / {total}   ·   {:.0}% of this stage",
                    100. * done as f32 / total as f32
                )
            },
        )
    };
    for (mut text, details, heading, _) in &mut texts {
        let value = if details {
            detail.clone()
        } else if heading {
            if report.failed() {
                "UNABLE TO PREPARE WORLD"
            } else {
                "PREPARING YOUR WORLD"
            }
            .into()
        } else {
            units.clone()
        };
        text.set_if_neq(Text::new(value));
    }
    for mut bar in &mut bars {
        // Never ease a previous category's progress into a new category's total.
        if bar.stage != current.stage {
            bar.displayed = 0.;
            bar.stage = current.stage.clone();
        }
        bar.target = current
            .units
            .filter(|(_, total)| *total > 0)
            .map(|(done, total)| (done as f32 / total as f32).clamp(0., 1.));
        bar.stopped = report.failed() || report.cancelled || report.finished;
        if report.finished && !report.failed() {
            bar.target = Some(1.);
        }
    }
    for mut node in &mut retry {
        let display = if report.failed() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

pub fn animate_progress(time: Res<Time>, mut bars: Query<(&mut LoadingBar, &mut UiTransform)>) {
    for (mut bar, mut transform) in &mut bars {
        let (width, center) = if let Some(target) = bar.target {
            let next =
                bar.displayed + (target - bar.displayed) * (1. - (-12. * time.delta_secs()).exp());
            bar.displayed = if (next - target).abs() < 0.001 {
                target
            } else {
                next
            };
            (bar.displayed, (bar.displayed - 1.) * 50.)
        } else if bar.stopped {
            (0., -50.)
        } else {
            bar.phase = (bar.phase + time.delta_secs() * 0.65).fract();
            (0.18, -41. * (bar.phase * std::f32::consts::TAU).cos())
        };
        transform.set_if_neq(UiTransform {
            scale: Vec2::new(width, 1.),
            translation: Val2::new(percent(center), px(0)),
            ..default()
        });
    }
}

pub fn cleanup(mut commands: Commands, roots: Query<Entity, With<LoadingScreen>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

pub fn is_loading(state: Option<Res<State<AppState>>>) -> bool {
    state.is_some_and(|s| matches!(s.get(), AppState::DataLoading | AppState::WorldGen))
}

pub fn input(
    mut reader: MessageReader<cdda_input::InputAction>,
    report: Res<OperationReport>,
    mut writer: MessageWriter<OperationCommand>,
) {
    for event in reader.read() {
        match event.action {
            cdda_input::GameAction::Cancel => {
                writer.write(OperationCommand::ReturnToMenu);
            }
            cdda_input::GameAction::Confirm if report.failed() => {
                writer.write(OperationCommand::Retry);
            }
            _ => {}
        }
    }
}

#[derive(Component)]
pub struct ReportNotice;
pub fn spawn_notice(mut commands: Commands, font: Res<UiFontHandle>) {
    commands.spawn((
        ReportNotice,
        GlobalZIndex(1100),
        Text::new(""),
        super::ui_font(&font.0, 16.),
        theme::TextPaint(theme::Role::Warning),
        theme::SurfacePaint(theme::Role::Surface),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            right: px(18),
            top: px(12),
            max_width: percent(75),
            padding: UiRect::all(px(12)),
            ..default()
        },
    ));
}
pub fn update_notice(
    report: Res<OperationReport>,
    state: Option<Res<State<AppState>>>,
    mut notice: Query<(&mut Text, &mut Node), With<ReportNotice>>,
) {
    if !report.is_changed() && !state.as_ref().is_some_and(|s| s.is_changed()) {
        return;
    }
    let loading =
        state.is_some_and(|s| matches!(s.get(), AppState::DataLoading | AppState::WorldGen));
    let event = report
        .current
        .as_ref()
        .filter(|e| !loading && matches!(e.level, ReportLevel::Warning | ReportLevel::Error));
    for (mut text, mut node) in &mut notice {
        node.display = if event.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(event) = event {
            text.set_if_neq(Text::new(event.to_string()));
        }
    }
}
