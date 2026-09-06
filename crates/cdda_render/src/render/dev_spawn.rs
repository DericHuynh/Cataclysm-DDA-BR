//! Debug spawn panel renderer.
//!
//! Left panel: scrollable, filtered item list.
//! Right panel: full item detail (all def components for the selected entry).
//!
//! Spawned on `OnEnter(Ctx::DevSpawnPanel)`, auto-despawned via `DespawnOnExit`.
//! The layout skeleton (title / body-row / filter / footer) is built once and
//! persists; keyed rows are recycled and details update independently of scrolling.

use bevy::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_ui::{sync_virtual_pane, FocusedRow, RetainedRows, RowCell, TextRow, VirtualList};

use super::FooterHint;
use crate::render::item_detail::{spawn_item_detail, ItemDetailQueries};
use crate::render::theme::{self, UiTheme};
use cdda_components::def::{DefStrId, IsDef, ItemName};
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_context::state::ContextActions;
use cdda_data::interner::{
    AmmoTypeRegistry, BodyPartRegistry, ComestibleRegistry, QualityRegistry, SkillRegistry,
};
use cdda_input::ActiveKeybindings;
use cdda_input::BindableAction;

// ---------------------------------------------------------------------------
// Dev spawn focus — migrated from the old cdda_worldgen crate
// ---------------------------------------------------------------------------

/// Entry in the dev-spawn item catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevCatalogEntry {
    pub def_entity: Entity,
    pub name: String,
    pub def_id: String,
}

/// Tracks focus index and sorted item catalog for the debug spawn panel.
#[derive(Resource, Debug, Default)]
pub struct DevSpawnFocus {
    pub index: usize,
    pub filter: String,
    pub filtering: bool,
}

/// Extracted item labels and identities, independent of focus/filter input.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct DevSpawnCatalog {
    pub entries: Vec<DevCatalogEntry>,
}

/// Membership is rebuilt only for a catalog revision or filter edit.
#[derive(Default)]
pub struct SpawnFilter {
    key: Option<(u32, String)>,
    pub indices: Vec<usize>,
    pub rebuilds: u64,
}
impl SpawnFilter {
    pub fn update(&mut self, catalog: &DevSpawnCatalog, filter: &str, version: u32) {
        if self
            .key
            .as_ref()
            .is_some_and(|(v, f)| *v == version && f == filter)
        {
            return;
        }
        let query = filter.to_lowercase();
        self.indices = catalog
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                (query.is_empty()
                    || e.name.to_lowercase().contains(&query)
                    || e.def_id.to_lowercase().contains(&query))
                .then_some(i)
            })
            .collect();
        self.key = Some((version, filter.to_string()));
        self.rebuilds += 1;
    }
}

/// Gate extraction while the screen is open, including removed components.
pub fn dev_spawn_catalog_changed(
    changed: Query<
        (),
        (
            With<IsDef>,
            Or<(Changed<ItemName>, Changed<DefStrId>, Added<IsDef>)>,
        ),
    >,
    mut removed_names: RemovedComponents<ItemName>,
    mut removed_ids: RemovedComponents<DefStrId>,
    mut removed_defs: RemovedComponents<IsDef>,
    mut initialized: Local<bool>,
) -> bool {
    // Drain every reader even if an earlier source was already dirty.
    let removed =
        removed_names.read().count() + removed_ids.read().count() + removed_defs.read().count();
    let dirty = !*initialized || !changed.is_empty() || removed != 0;
    *initialized = true;
    dirty
}

/// Refresh the item catalog; run unconditionally on entry so changes made while
/// the screen was closed are observed even after removal events have expired.
/// Selection follows the stable source ID across definition entity replacement.
pub fn dev_spawn_populate(
    mut focus: ResMut<DevSpawnFocus>,
    mut catalog: ResMut<DevSpawnCatalog>,
    mut view: Local<SpawnFilter>,
    items: Query<(Entity, &ItemName, &DefStrId), With<IsDef>>,
) {
    let mut entries: Vec<DevCatalogEntry> = items
        .iter()
        .map(|(def_entity, name, id)| DevCatalogEntry {
            def_id: id.0.clone(),
            name: name.0.clone(),
            def_entity,
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.def_id.cmp(&b.def_id)));
    if catalog.entries == entries {
        return;
    }
    view.update(&catalog, &focus.filter, catalog.last_changed().get());
    let selected = view
        .indices
        .get(focus.index)
        .map(|&i| catalog.entries[i].def_id.clone());
    catalog.entries = entries;
    // The resource tick may equal the prior extraction within one frame; force
    // membership refresh after replacing the source rather than assuming a tick.
    view.key = None;
    view.update(&catalog, &focus.filter, catalog.last_changed().get());
    let index = selected
        .as_ref()
        .and_then(|id| {
            view.indices
                .iter()
                .position(|&i| &catalog.entries[i].def_id == id)
        })
        .unwrap_or_else(|| focus.index.min(view.indices.len().saturating_sub(1)));
    if focus.index != index {
        focus.index = index;
    }
}

// ---------------------------------------------------------------------------
// Persistent-skeleton markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct SpawnListContainer;

#[derive(Component)]
pub struct SpawnTitleBar;

#[derive(Component)]
pub struct SpawnListPanel;

#[derive(Component)]
pub struct SpawnDetailPanel;

#[derive(Component)]
pub struct SpawnFilterBar;

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
            theme::SurfacePaint(theme::Role::Canvas),
        ))
        .with_children(|root| {
            // Fixed title and count, updated in place
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
                theme::SurfacePaint(theme::Role::Raised),
            ))
            .with_children(|header| {
                header.spawn((
                    SpawnHeading,
                    Text::new("DEBUG: SPAWN ITEM"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Text),
                ));
                header.spawn((
                    SpawnCount,
                    Text::default(),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
            });

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
                        RetainedRows::<Option<String>>::default(),
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
                        theme::SurfacePaint(theme::Role::Surface),
                        theme::BorderPaint(theme::Role::Border),
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
                        theme::SurfacePaint(theme::Role::Surface),
                    ));
                });

            // Fixed filter text
            root.spawn((
                SpawnFilterBar,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(6.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                theme::BorderPaint(theme::Role::Border),
            ))
            .with_child((
                SpawnFilterText,
                Text::default(),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Muted),
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
                theme::SurfacePaint(theme::Role::Raised),
                theme::BorderPaint(theme::Role::Border),
            ))
            .with_child((
                Text::new(hints),
                super::ui_font(&ui_font_handle.0, 12.0),
                theme::TextPaint(theme::Role::Muted),
                FooterHint,
            ));
        });
}

// ---------------------------------------------------------------------------
// Update — independently synchronize rows, fixed text and selected detail.
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct SpawnHeading;
#[derive(Component)]
pub struct SpawnCount;
#[derive(Component)]
pub struct SpawnFilterText;

#[derive(SystemParam)]
pub struct SpawnPanels<'w, 's> {
    list: Query<
        'w,
        's,
        (
            Entity,
            &'static mut VirtualList,
            &'static mut FocusedRow,
            &'static mut ScrollPosition,
            &'static ComputedNode,
            &'static mut RetainedRows<Option<String>>,
        ),
        With<SpawnListPanel>,
    >,
    detail: Query<'w, 's, Entity, With<SpawnDetailPanel>>,
    filter: Query<'w, 's, &'static mut BackgroundColor, With<SpawnFilterBar>>,
    text: Query<
        'w,
        's,
        (
            &'static mut Text,
            &'static mut theme::TextPaint,
            Option<&'static SpawnHeading>,
            Option<&'static SpawnCount>,
        ),
        Or<(With<SpawnHeading>, With<SpawnCount>, With<SpawnFilterText>)>,
    >,
}
#[derive(Default)]
pub struct SpawnPresentation {
    pane: Option<Entity>,
    filter: String,
    membership: SpawnFilter,
    detail_key: Option<(Entity, Option<Entity>, u32, theme::ThemePreset)>,
}

pub fn update_dev_spawn_panel(
    mut commands: Commands,
    focus: Res<DevSpawnFocus>,
    catalog: Res<DevSpawnCatalog>,
    mut panels: SpawnPanels,
    detail: ItemDetailQueries,
    quality_registry: Res<QualityRegistry>,
    skill_registry: Res<SkillRegistry>,
    ammo_registry: Res<AmmoTypeRegistry>,
    body_part_registry: Res<BodyPartRegistry>,
    comestible_registry: Res<ComestibleRegistry>,
    defs: Option<Res<cdda_data::def_world::DefinitionWorld>>,
    theme: Res<UiTheme>,
    mut cache: Local<SpawnPresentation>,
) {
    let Ok((pane, mut list, mut selected, mut position, computed, mut rows)) =
        panels.list.single_mut()
    else {
        return;
    };
    let reset = cache.pane != Some(pane) || cache.filter != focus.filter;
    cache
        .membership
        .update(&catalog, &focus.filter, catalog.last_changed().get());
    let source_changed = defs.as_ref().is_some_and(|d| d.is_changed())
        || quality_registry.is_changed()
        || skill_registry.is_changed()
        || ammo_registry.is_changed()
        || body_part_registry.is_changed()
        || comestible_registry.is_changed();
    if !reset
        && !focus.is_changed()
        && !catalog.is_changed()
        && !theme.is_changed()
        && !source_changed
        && !list.is_changed()
    {
        return;
    }
    cache.pane = Some(pane);
    cache.filter.clone_from(&focus.filter);
    let total = cache.membership.indices.len();
    sync_virtual_pane(
        &mut list,
        &mut selected,
        &mut position,
        computed,
        total,
        focus.index,
        reset,
    );
    let values = (list.window.0..list.window.1)
        .map(|i| {
            let entry = &catalog.entries[cache.membership.indices[i]];
            (
                Some(entry.def_id.clone()),
                TextRow {
                    node: Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Start,
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..list.row_node()
                    },
                    background: if i == selected.0 {
                        theme.item_focus_bg()
                    } else {
                        theme.color(theme::Role::Surface)
                    },
                    border: theme.color(theme::Role::Border),
                    cells: vec![
                        RowCell::new(&entry.name, 15.0, theme.color(theme::Role::Text)),
                        RowCell::new(&entry.def_id, 11.0, theme.color(theme::Role::Muted)),
                    ],
                },
            )
        })
        .collect::<Vec<_>>();
    let values = if total == 0 {
        vec![(
            None,
            TextRow {
                node: list.row_node(),
                background: theme.color(theme::Role::Surface),
                border: theme.color(theme::Role::Border),
                cells: vec![RowCell::new(
                    if catalog.entries.is_empty() {
                        "No item definitions"
                    } else {
                        "No matches"
                    },
                    16.0,
                    theme.color(theme::Role::Muted),
                )],
            },
        )]
    } else {
        values
    };
    rows.sync(&mut commands, pane, &list, values);

    for (mut text, mut color, heading, count) in &mut panels.text {
        let (label, tint) = if heading.is_some() {
            ("DEBUG: SPAWN ITEM".into(), theme::Role::Accent)
        } else if count.is_some() {
            (
                if focus.filter.is_empty() {
                    format!("{} items", total)
                } else {
                    format!("{} / {} items", total, catalog.entries.len())
                },
                theme::Role::Muted,
            )
        } else {
            (
                if focus.filtering {
                    format!("Filter: {}_", focus.filter)
                } else if focus.filter.is_empty() {
                    "[/] to filter".into()
                } else {
                    format!("Filter: {}  (Enter=close  Esc=clear)", focus.filter)
                },
                if focus.filtering {
                    theme::Role::Text
                } else {
                    theme::Role::Muted
                },
            )
        };
        text.set_if_neq(Text::new(label));
        color.set_if_neq(theme::TextPaint(tint));
    }
    if let Ok(mut bg) = panels.filter.single_mut() {
        bg.set_if_neq(BackgroundColor(if focus.filtering {
            theme.color(theme::Role::Selection)
        } else {
            Color::NONE
        }));
    }
    let Ok(panel) = panels.detail.single() else {
        return;
    };
    let entry = cache
        .membership
        .indices
        .get(focus.index)
        .map(|&i| &catalog.entries[i]);
    let key = (
        panel,
        entry.map(|e| e.def_entity),
        catalog.last_changed().get(),
        theme.preset,
    );
    if cache.detail_key == Some(key) && !source_changed {
        return;
    }
    cache.detail_key = Some(key);
    commands
        .entity(panel)
        .despawn_children()
        .with_children(|parent| {
            if let Some(entry) = entry {
                spawn_item_detail(
                    parent,
                    &entry.name,
                    &entry.def_id,
                    entry.def_entity,
                    &detail,
                    &quality_registry,
                    &skill_registry,
                    &ammo_registry,
                    &body_part_registry,
                    &comestible_registry,
                );
            } else {
                parent.spawn((
                    Text::new("Select an item"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
            }
        });
}
