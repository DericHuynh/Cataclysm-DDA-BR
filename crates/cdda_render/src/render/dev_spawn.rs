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
use crate::render::item_detail::{spawn_item_detail, ItemDetailQueries};
use crate::render::theme::{self, UiTheme};
use cdda_components::context::ContextActions;
use cdda_components::def::{DefStrId, IsDef, ItemName};
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_data::interner::{
    AmmoTypeRegistry, BodyPartRegistry, ComestibleRegistry, ItemTypeRegistry, QualityRegistry,
    SkillRegistry,
};
use cdda_input::ActiveKeybindings;
use cdda_input::BindableAction;

// ---------------------------------------------------------------------------
// Dev spawn focus — migrated from the old cdda_worldgen crate
// ---------------------------------------------------------------------------

/// Entry in the dev-spawn item catalog.
#[derive(Debug, Clone)]
pub struct DevCatalogEntry {
    pub def_entity: Entity,
    pub name: String,
    pub def_id: String,
}

/// Tracks focus index and sorted item catalog for the debug spawn panel.
#[derive(Resource, Debug, Default)]
pub struct DevSpawnFocus {
    pub index: usize,
    pub catalog: Vec<DevCatalogEntry>,
    pub filter: String,
    pub filtering: bool,
}

impl DevSpawnFocus {
    pub fn filtered_entries(&self) -> Vec<&DevCatalogEntry> {
        if self.filter.is_empty() {
            self.catalog.iter().collect()
        } else {
            let q = self.filter.to_lowercase();
            self.catalog
                .iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&q) || e.def_id.to_lowercase().contains(&q)
                })
                .collect()
        }
    }
}

/// Populate [`DevSpawnFocus`]'s item catalog from the definition world.
///
/// Every item-definition entity (marked `IsDef` with an `ItemName`) becomes a
/// catalog entry.  Run on `OnEnter(DevSpawnPanel)`.  Idempotent — if the
/// catalog already has entries it returns early so the screen doesn't resize
/// itself on every keypress.
pub fn dev_spawn_populate(
    mut focus: ResMut<DevSpawnFocus>,
    items: Query<(Entity, &ItemName, &DefStrId), With<IsDef>>,
) {
    if !focus.catalog.is_empty() {
        return;
    }
    let mut entries: Vec<DevCatalogEntry> = items
        .iter()
        .map(|(def_entity, name, id)| DevCatalogEntry {
            def_id: id.0.clone(),
            name: name.0.clone(),
            def_entity,
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    focus.catalog = entries;
    focus.index = 0;
}

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

// ---------------------------------------------------------------------------
// CddaScreen trait impl
// ---------------------------------------------------------------------------

pub struct DevSpawnScreen;

impl CddaScreen for DevSpawnScreen {
    const CTX: Ctx = Ctx::DevSpawnPanel;
    const ACTIONS: &'static [(&'static str, BindableAction)] = &[
        ("navigate", BindableAction::NavigateUp),
        ("page", BindableAction::NavigatePageUp),
        ("first/last", BindableAction::NavigateHome),
        ("filter", BindableAction::Filter),
        ("spawn", BindableAction::Confirm),
    ];

    fn spawn(_world: &mut World) {
        // Handled by spawn_dev_spawn_panel in mod.rs.
        // TODO: migrate that system into this method.
    }

    fn update(_world: &mut World) {
        // Handled by update_dev_spawn_panel in mod.rs.
        // TODO: migrate that system into this method.
    }
}

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
            BackgroundColor(theme::BG),
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
                BackgroundColor(theme::HEADER_BG),
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
                        crate::render::scroll::KeyboardScroll,
                        crate::render::scroll::FocusedRow::default(),
                        crate::render::scroll::VirtualList {
                            row_height: 48.0,
                            ..default()
                        },
                        ScrollPosition::default(),
                        Node {
                            width: Val::Percent(38.0),
                            min_width: Val::Percent(38.0),
                            max_width: Val::Percent(38.0),
                            min_height: Val::Px(0.0),
                            flex_shrink: 0.0,
                            flex_grow: 0.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            border: UiRect::right(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::PANEL_BG),
                        BorderColor::all(theme::DIVIDER),
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
                        BackgroundColor(theme::PANEL_BG),
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
                BorderColor::all(theme::DIVIDER),
            ));

            // Footer — static, built once
            let cancel_key = active_keys.key_for(cdda_input::BindableAction::Cancel);
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
                BackgroundColor(theme::HEADER_BG),
                BorderColor::all(theme::DIVIDER),
            ))
            .with_child((
                Text::new(hints),
                super::ui_font(&ui_font_handle.0, 12.0),
                TextColor(theme::TEXT_DIM),
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
    mut list_panel: Query<(Entity, &mut crate::render::scroll::VirtualList), With<SpawnListPanel>>,
    detail_panel: Query<Entity, With<SpawnDetailPanel>>,
    filter_bar: Query<Entity, With<SpawnFilterBar>>,
    detail: ItemDetailQueries,
    quality_registry: Res<QualityRegistry>,
    skill_registry: Res<SkillRegistry>,
    ammo_registry: Res<AmmoTypeRegistry>,
    body_part_registry: Res<BodyPartRegistry>,
    comestible_registry: Res<ComestibleRegistry>,
    theme: Res<UiTheme>,
) {
    if !focus.is_changed()
        && !theme.is_changed()
        && !list_panel.iter_mut().any(|(_, list)| list.is_changed())
    {
        return;
    }
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
                    TextColor(theme.accent()),
                ));
                h.spawn((
                    Text::new(status),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
            });
    }

    // ── List panel ────────────────────────────────────────────────────────
    if let Ok((list_e, mut virtual_list)) = list_panel.single_mut() {
        // Keep the virtualization size in sync with the data; the window is
        // recomputed next frame from `ScrollPosition` by `update_virtual_windows`.
        if virtual_list.total_rows != total {
            virtual_list.total_rows = total;
        }

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
                            TextColor(theme::TEXT_DIM),
                        ));
                    return;
                }

                // Virtualized rows: a top spacer preserves scroll offset, then
                // only the visible window's rows are spawned (so a 40k-item
                // catalog stays cheap), then a bottom spacer keeps the scroll
                // range equal to the full data height.
                let (win_start, win_end) = virtual_list.window;
                let top_px = crate::render::scroll::virtual_top_spacer_px(&virtual_list);
                let bottom_px = crate::render::scroll::virtual_bottom_spacer_px(&virtual_list);
                if top_px > 0.0 {
                    list.spawn(Node {
                        height: Val::Px(top_px),
                        flex_shrink: 0.0,
                        ..default()
                    });
                }

                for i in win_start..win_end {
                    let Some(entry) = filtered.get(i) else {
                        break;
                    };
                    let is_focused = i == focus.index;
                    let row_bg = if is_focused {
                        theme.item_focus_bg()
                    } else {
                        theme::ITEM_BG
                    };

                    list.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            height: Val::Px(48.0),
                            flex_shrink: 0.0,
                            overflow: Overflow::clip(),
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(row_bg),
                        BorderColor::all(theme::DIVIDER),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(entry.name.clone()),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(theme::TEXT_BRIGHT),
                        ));
                        row.spawn((
                            Text::new(entry.def_id.clone()),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(theme::TEXT_ID),
                        ));
                    });
                }
                if bottom_px > 0.0 {
                    list.spawn(Node {
                        height: Val::Px(bottom_px),
                        flex_shrink: 0.0,
                        ..default()
                    });
                }
            });

        // Feed the focused row to the shared keep-visible scroll.
        if focus.is_changed() {
            commands
                .entity(list_e)
                .insert(crate::render::scroll::FocusedRow(focus.index));
        }
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
                        TextColor(theme::TEXT_DIM),
                    ));
                    return;
                };
                spawn_item_detail(
                    d,
                    selected_name,
                    selected_id,
                    def,
                    &detail,
                    &quality_registry,
                    &skill_registry,
                    &ammo_registry,
                    &body_part_registry,
                    &comestible_registry,
                );
            });
    }

    // ── Filter bar ────────────────────────────────────────────────────────
    if let Ok(filt_e) = filter_bar.single() {
        let filter_bg = if focus.filtering {
            theme::FILTER_ACTIVE_BG
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
                theme::TEXT_BRIGHT
            } else {
                theme::TEXT_DIM
            }),
        ));
    }
}
