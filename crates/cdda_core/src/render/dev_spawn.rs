//! Debug spawn panel renderer.
//!
//! Reads the dynamically-built `DevSpawnFocus.catalog` (populated from live
//! `IsDef` entities by `build_dev_spawn_catalog`) and renders a scrollable
//! item list with focus highlight.
//!
//! Spawned on `OnEnter(Screen::DevSpawnPanel)`, auto-despawned via
//! `DespawnOnExit`. Rows are rebuilt each frame so the focus is always current.

use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;
use crate::context::ctx::Ctx as Screen;
use crate::core::components::def::ContainerData;
use crate::worldgen::dev_spawn::DevSpawnFocus;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.04, 0.04, 0.06);
const HEADER_BG: Color = Color::srgb(0.10, 0.10, 0.14);
const ITEM_BG: Color = Color::srgb(0.07, 0.07, 0.10);
const ITEM_FOCUS_BG: Color = Color::srgb(0.12, 0.35, 0.55);
const ACCENT: Color = Color::srgb(0.30, 0.70, 1.00);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
const TEXT_ID: Color = Color::srgb(0.50, 0.65, 0.50);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);

// Rows visible at once without scrolling.
const VISIBLE_ROWS: usize = 20;

// ---------------------------------------------------------------------------
// Marker
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct SpawnListContainer;

// ---------------------------------------------------------------------------
// Spawn (OnEnter)
// ---------------------------------------------------------------------------

pub fn spawn_dev_spawn_panel(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(Screen::DevSpawnPanel),
            Node {
                width: Val::Percent(65.0),
                height: Val::Percent(90.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::axes(Val::Auto, Val::Auto),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // ── Title bar ─────────────────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
            ))
            .with_children(|h| {
                h.spawn((
                    Text::new("DEBUG: SPAWN ITEM"),
                    TextFont { font_size: 26.0, ..default() },
                    TextColor(ACCENT),
                ));
                h.spawn((
                    Text::new("  —  spawns 1× at camera tile"),
                    TextFont { font_size: 15.0, ..default() },
                    TextColor(TEXT_DIM),
                ));
            });

            // ── Item list ─────────────────────────────────────────────────
            root.spawn((
                SpawnListContainer,
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    overflow: Overflow::clip_y(),
                    ..default()
                },
            ));

            // ── Footer hint ───────────────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
                BorderColor::all(DIVIDER),
            ))
            .with_child((
                Text::new(
                    "[j/k / ↑↓] navigate    [PgUp/PgDn] scroll    [Enter / e] spawn    [Esc / q] close",
                ),
                TextFont { font_size: 13.0, ..default() },
                TextColor(TEXT_DIM),
            ));
        });
}

// ---------------------------------------------------------------------------
// Update — rebuild visible rows each frame
// ---------------------------------------------------------------------------

pub(crate) fn update_dev_spawn_panel(
    mut commands: Commands,
    focus: Res<DevSpawnFocus>,
    container: Query<Entity, With<SpawnListContainer>>,
    world: &World,
) {
    let Ok(container_entity) = container.single() else {
        return;
    };

    commands.entity(container_entity).despawn_children();

    if focus.catalog.is_empty() {
        commands.entity(container_entity).with_children(|p| {
            p.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(14.0)),
                    ..default()
                },
            ))
            .with_child((
                Text::new("Loading item definitions…"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(TEXT_DIM),
            ));
        });
        return;
    }

    // Compute the scroll window so the focused row is always visible.
    let scroll_start = if focus.index >= VISIBLE_ROWS {
        focus.index + 1 - VISIBLE_ROWS
    } else {
        0
    };
    let scroll_end = (scroll_start + VISIBLE_ROWS).min(focus.catalog.len());

    // Header: show position in catalog
    commands.entity(container_entity).with_children(|p| {
        p.spawn((
            Node {
                padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(4.0), Val::Px(4.0)),
                ..default()
            },
        ))
        .with_child((
            Text::new(format!(
                "Item {} of {} — sorted by name",
                focus.index + 1,
                focus.catalog.len()
            )),
            TextFont { font_size: 13.0, ..default() },
            TextColor(TEXT_DIM),
        ));
    });

    for (i, entry) in focus.catalog[scroll_start..scroll_end].iter().enumerate() {
        let abs_index = scroll_start + i;
        let is_focused = abs_index == focus.index;
        let row_bg = if is_focused { ITEM_FOCUS_BG } else { ITEM_BG };

        let row_label = format!("{}", entry.name);
        let id_label = format!("  [{}]", entry.def_id);

        commands.entity(container_entity).with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(row_bg),
                BorderColor::all(DIVIDER),
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(row_label),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(TEXT_BRIGHT),
                    Node { flex_grow: 1.0, ..default() },
                ));
                row.spawn((
                    Text::new(id_label),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(TEXT_ID),
                ));
            });

            // Pocket detail panel below focused item
            if is_focused {
                if let Some(container) = world.get::<ContainerData>(entry.def_entity) {
                    let pocket_count = container.pockets.len();
                    p.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::axes(Val::Px(24.0), Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.08, 0.12, 0.18)),
                    ))
                    .with_children(|det| {
                        det.spawn((
                            Text::new(format!(
                                "Pockets: {} total  |  max vol {} mL  |  max wt {} g",
                                pocket_count, container.max_volume, container.max_weight
                            )),
                            TextFont { font_size: 13.0, ..default() },
                            TextColor(ACCENT),
                        ));
                        if pocket_count > 0 {
                            for (pi, pocket) in container.pockets.iter().enumerate() {
                                let sealed = if pocket.sealed { " [sealed]" } else { "" };
                                let rigid = if pocket.rigid { " [rigid]" } else { "" };
                                det.spawn((
                                    Text::new(format!(
                                        "  {}: type={}  vol={} mL  wt={} g  max_len={}{}{}",
                                        pi + 1,
                                        pocket.pocket_type,
                                        pocket.max_volume,
                                        pocket.max_weight,
                                        pocket.max_item_length,
                                        sealed,
                                        rigid,
                                    )),
                                    TextFont { font_size: 12.0, ..default() },
                                    TextColor(TEXT_BRIGHT),
                                ));
                            }
                        }
                    });
                }
            }
        });
    }
}
