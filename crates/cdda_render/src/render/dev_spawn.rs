//! Debug spawn panel renderer.
//!
//! Left panel: scrollable, filtered item list.
//! Right panel: full item detail (all def components for the selected entry).
//!
//! Spawned on `OnEnter(Ctx::DevSpawnPanel)`, auto-despawned via `DespawnOnExit`.
//! The layout skeleton (title / body-row / filter / footer) is built once and
//! persists; only the *content* inside each container is rebuilt each frame.

use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;

use super::FooterHint;
use crate::context::ctx::Ctx;
use crate::context::ContextActions;
use crate::input::ActiveKeybindings;
use crate::render::item_detail::{spawn_item_detail, ItemDetailQueries};
use crate::worldgen::dev_spawn::DevSpawnFocus;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.04, 0.04, 0.06);
const PANEL_BG: Color = Color::srgb(0.07, 0.07, 0.10);
const HEADER_BG: Color = Color::srgb(0.10, 0.10, 0.14);
const ITEM_BG: Color = Color::srgb(0.06, 0.06, 0.09);
const ITEM_FOCUS_BG: Color = Color::srgb(0.12, 0.35, 0.55);
const ACCENT: Color = Color::srgb(0.30, 0.70, 1.00);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
const TEXT_ID: Color = Color::srgb(0.50, 0.65, 0.50);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);
const FILTER_ACTIVE_BG: Color = Color::srgb(0.08, 0.18, 0.30);

// Rows visible at once in the list (controls centered-scroll window).
const VISIBLE_ROWS: usize = 22;

// ---------------------------------------------------------------------------
// Persistent-skeleton markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct SpawnListContainer;

#[derive(Component)]
pub(crate) struct SpawnTitleBar;

#[derive(Component)]
pub(crate) struct SpawnListPanel;

#[derive(Component)]
pub(crate) struct SpawnDetailPanel;

#[derive(Component)]
pub(crate) struct SpawnFilterBar;

// ---------------------------------------------------------------------------
// Shared item detail queries

// ---------------------------------------------------------------------------
// Spawn (OnEnter) — full layout skeleton, built once
// ---------------------------------------------------------------------------

pub fn spawn_dev_spawn_panel(
    mut commands: Commands,
    ctx_actions: Res<ContextActions>,
    active_keys: Res<ActiveKeybindings>,
    ui_font_handle: Res<super::UiFontHandle>,
) {
    commands
        .spawn((
            DespawnOnExit(Ctx::DevSpawnPanel),
            SpawnListContainer,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // Title bar — children rebuilt each frame
            root.spawn((
                SpawnTitleBar,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(HEADER_BG),
            ));

            // Body row: grows to fill all space between title and footer
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                overflow: Overflow::clip(),
                ..default()
            },))
                .with_children(|body| {
                    // Left: list panel — children rebuilt each frame
                    body.spawn((
                        SpawnListPanel,
                        Node {
                            width: Val::Percent(38.0),
                            min_width: Val::Percent(38.0),
                            max_width: Val::Percent(38.0),
                            min_height: Val::Px(0.0),
                            flex_shrink: 0.0,
                            flex_grow: 0.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::clip(),
                            border: UiRect::right(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                        BorderColor::all(DIVIDER),
                    ));

                    // Right: detail panel — children rebuilt each frame
                    body.spawn((
                        SpawnDetailPanel,
                        Node {
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(16.0)),
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ));
                });

            // Filter bar — background + child rebuilt each frame
            root.spawn((
                SpawnFilterBar,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(6.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(DIVIDER),
            ));

            // Footer — static, built once
            let cancel_key = active_keys.key_for(crate::input::BindableAction::Cancel);
            let mut hints = format!("[{}] close", cancel_key);
            for entry in &ctx_actions.actions {
                let key = active_keys.key_for(entry.action);
                hints.push_str(&format!("  [{}] {}", key, entry.label));
            }
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
                BorderColor::all(DIVIDER),
            ))
            .with_child((
                Text::new(hints),
                super::ui_font(&ui_font_handle.0, 12.0),
                TextColor(TEXT_DIM),
                FooterHint,
            ));
        });
}

// ---------------------------------------------------------------------------
// Update — rebuild only the *content* inside each persistent container
// ---------------------------------------------------------------------------

pub(crate) fn update_dev_spawn_panel(
    mut commands: Commands,
    focus: Res<DevSpawnFocus>,
    title_bar: Query<Entity, With<SpawnTitleBar>>,
    list_panel: Query<Entity, With<SpawnListPanel>>,
    detail_panel: Query<Entity, With<SpawnDetailPanel>>,
    filter_bar: Query<Entity, With<SpawnFilterBar>>,
    detail: ItemDetailQueries,
) {
    let filtered = focus.filtered_entries();
    let total = filtered.len();

    // ── Title bar ─────────────────────────────────────────────────────────
    if let Ok(title_e) = title_bar.single() {
        let status = if focus.filter.is_empty() {
            format!("{} items", focus.catalog.len())
        } else {
            format!("{} / {} items", total, focus.catalog.len())
        };
        commands
            .entity(title_e)
            .despawn_children()
            .with_children(|h| {
                h.spawn((
                    Text::new("DEBUG: SPAWN ITEM"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(ACCENT),
                ));
                h.spawn((
                    Text::new(status),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));
            });
    }

    // ── List panel ────────────────────────────────────────────────────────
    if let Ok(list_e) = list_panel.single() {
        // Centered scroll: selected row stays near the middle.
        let scroll_start = focus.index.saturating_sub(VISIBLE_ROWS / 2);
        let scroll_end = (scroll_start + VISIBLE_ROWS).min(total);

        commands
            .entity(list_e)
            .despawn_children()
            .with_children(|list| {
                if filtered.is_empty() {
                    list.spawn((Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(14.0)),
                        ..default()
                    },))
                        .with_child((
                            Text::new(if focus.catalog.is_empty() {
                                "Loading..."
                            } else {
                                "No matches"
                            }),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(TEXT_DIM),
                        ));
                    return;
                }

                // Position indicator
                list.spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(14.0),
                            Val::Px(14.0),
                            Val::Px(3.0),
                            Val::Px(3.0),
                        ),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(DIVIDER),
                ))
                .with_child((
                    Text::new(format!("{} / {}", focus.index + 1, total)),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));

                for i in scroll_start..scroll_end {
                    let entry = &filtered[i];
                    let is_focused = i == focus.index;
                    let row_bg = if is_focused { ITEM_FOCUS_BG } else { ITEM_BG };

                    list.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(row_bg),
                        BorderColor::all(DIVIDER),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(entry.name.clone()),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(TEXT_BRIGHT),
                        ));
                        row.spawn((
                            Text::new(entry.def_id.clone()),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(TEXT_ID),
                        ));
                    });
                }
            });
    }

    // ── Detail panel (shared widget) ─────────────────────────────────────
    if let Ok(det_e) = detail_panel.single() {
        let selected_entity = filtered.get(focus.index).map(|e| e.def_entity);
        let selected_name = filtered
            .get(focus.index)
            .map(|e| e.name.as_str())
            .unwrap_or("");
        let selected_id = filtered
            .get(focus.index)
            .map(|e| e.def_id.as_str())
            .unwrap_or("");

        commands
            .entity(det_e)
            .despawn_children()
            .with_children(|d| {
                let Some(def) = selected_entity else {
                    d.spawn((
                        Text::new("Select an item"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                    return;
                };
                spawn_item_detail(d, selected_name, selected_id, def, &detail);
            });
    }

    // ── Filter bar ────────────────────────────────────────────────────────
    if let Ok(filt_e) = filter_bar.single() {
        let filter_bg = if focus.filtering {
            FILTER_ACTIVE_BG
        } else {
            Color::NONE
        };
        let filter_text = if focus.filtering {
            format!("Filter: {}_", focus.filter)
        } else if focus.filter.is_empty() {
            "[/] to filter".to_string()
        } else {
            format!("Filter: {}  (Enter=close  Esc=clear)", focus.filter)
        };
        commands.entity(filt_e).insert(BackgroundColor(filter_bg));
        commands.entity(filt_e).despawn_children().with_child((
            Text::new(filter_text),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(if focus.filtering {
                TEXT_BRIGHT
            } else {
                TEXT_DIM
            }),
        ));
    }
}
