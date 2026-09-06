//! Crafting menu — Bevy UI rendering.
//!
//! Spawned on `OnEnter(Ctx::CraftingMenu)`, auto-despawned via `DespawnOnExit`.
//! The layout skeleton is built once in `spawn_crafting_ui`; content is rebuilt
//! on changes in `update_crafting_ui`.
//!
//! Layout (top to bottom):
//!   1. Header bar — "CRAFTING" title, recipe count / position
//!   2. Top-level category tabs (LEFT/RIGHT to switch)
//!   3. Subcategory tabs (for selected top category)
//!   4. Body — recipe list (left) + detail panel (middle) + item detail panel (right)
//!   5. Filter bar (at bottom)
//!   6. Footer with key hints

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_ui::{RetainedRows, RowCell, TextRow};

use super::scroll::{sync_virtual_pane, FocusedRow, VirtualList};
use super::FooterHint;
use crate::render::crafting_state::{CategoryIndex, CraftState};
use crate::render::item_detail::{spawn_item_detail, ItemDetailQueries};
use crate::render::theme::{self, UiTheme};
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_data::def_world::DefinitionWorld;
use cdda_data::interner::{
    AmmoTypeRegistry, BodyPartRegistry, ComestibleRegistry, ItemTypeRegistry, QualityRegistry,
    SkillRegistry,
};
use cdda_input::BindableAction;

// ---------------------------------------------------------------------------
// Marker components for targeted content rebuild
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct CraftMenuRoot;

#[derive(Component)]
pub struct HeaderContainer;

#[derive(Component)]
pub struct CategoryTabsContainer;

#[derive(Component)]
pub struct SubcategoryTabsContainer;

#[derive(Component)]
pub struct RecipeListContainer;

#[derive(Component)]
pub struct DetailPanelContainer;

#[derive(Component)]
pub struct ItemDetailPanelContainer;

#[derive(Component)]
pub struct FilterBarContainer;

#[derive(Component)]
pub struct FooterContainer;

// ---------------------------------------------------------------------------
// CddaScreen impl
// ---------------------------------------------------------------------------

pub struct CraftingScreen;

impl CddaScreen for CraftingScreen {
    const CTX: Ctx = Ctx::CraftingMenu;
    const ACTIONS: &'static [(&'static str, BindableAction)] = &[
        ("navigate", BindableAction::NavigateUp),
        ("category", BindableAction::NavigateNextTab),
        ("craft", BindableAction::Confirm),
        ("toggle all", BindableAction::HotkeyA),
        ("filter", BindableAction::Filter),
    ];

    fn spawn(world: &mut World) {
        let theme = world.resource::<UiTheme>().clone();
        spawn_crafting_ui(world.commands(), &theme);
    }

    fn update(_world: &mut World) {
        // Handled by update_crafting_ui in mod.rs.
        // TODO: migrate that system into this method.
    }
}

// ---------------------------------------------------------------------------
// SystemParam that bundles all container queries, keeping the system
// under Bevy's IntoSystem parameter limit.
// ---------------------------------------------------------------------------

#[derive(SystemParam)]
pub struct CraftingContainers<'w, 's> {
    pub root: Query<'w, 's, Entity, With<CraftMenuRoot>>,
    pub header: Query<'w, 's, Entity, With<HeaderContainer>>,
    pub cat_tabs: Query<'w, 's, Entity, With<CategoryTabsContainer>>,
    pub sub_tabs: Query<'w, 's, Entity, With<SubcategoryTabsContainer>>,
    pub counter: Query<'w, 's, &'static mut Text, With<RecipeCounter>>,
    pub heading: Query<'w, 's, &'static mut TextColor, With<RecipeHeading>>,
    pub list: Query<
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
        With<RecipeListContainer>,
    >,
    pub detail: Query<'w, 's, Entity, With<DetailPanelContainer>>,
    pub item_detail: Query<'w, 's, Entity, With<ItemDetailPanelContainer>>,
    pub filter: Query<'w, 's, Entity, With<FilterBarContainer>>,
    pub footer: Query<'w, 's, Entity, With<FooterContainer>>,
}

// ---------------------------------------------------------------------------
// spawn_crafting_ui — OnEnter system (root shell only)
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct RecipeCounter;
#[derive(Component)]
pub struct RecipeHeading;

/// Spawn the persistent root wrapper for the crafting menu.
/// Content is retained until its data, selection, theme, or visible window changes.
pub fn spawn_crafting_ui(mut commands: Commands, _theme: &UiTheme) {
    commands
        .spawn((
            DespawnOnExit(Ctx::CraftingMenu),
            CraftMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            theme::SurfacePaint(theme::Role::Canvas),
        ))
        .with_children(|root| {
            // ── 1. Header ─────────────────────────────────────────────────
            root.spawn((
                HeaderContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Raised),
            ))
            .with_children(|header| {
                header.spawn((
                    RecipeHeading,
                    Text::new("CRAFTING"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Accent),
                ));
                header.spawn((
                    RecipeCounter,
                    Text::new(""),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
            });

            // ── 2. Top-level category tabs ────────────────────────────────
            root.spawn((
                CategoryTabsContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    column_gap: Val::Px(4.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Surface),
            ));

            // ── 3. Subcategory tabs ───────────────────────────────────────
            root.spawn((
                SubcategoryTabsContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                    column_gap: Val::Px(3.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Surface),
            ));

            // ── 4. Body: recipe list + detail panel ───────────────────────
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                min_height: Val::Px(0.0),
                overflow: Overflow::clip(),
                ..default()
            },))
                .with_children(|body| {
                    // Left: recipe list
                    body.spawn((
                        RecipeListContainer,
                        RetainedRows::<Option<String>>::default(),
                        crate::render::scroll::KeyboardScroll,
                        crate::render::scroll::FocusedRow::default(),
                        VirtualList {
                            row_height: 36.0,
                            ..default()
                        },
                        ScrollPosition::default(),
                        Node {
                            width: Val::Percent(45.0),
                            min_width: Val::Percent(45.0),
                            max_width: Val::Percent(45.0),
                            flex_shrink: 0.0,
                            flex_grow: 0.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        theme::SurfacePaint(theme::Role::Surface),
                    ));

                    // Middle: crafting detail panel
                    body.spawn((
                        DetailPanelContainer,
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(5.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        theme::SurfacePaint(theme::Role::Surface),
                    ));

                    // Right: item detail panel
                    body.spawn((
                        ItemDetailPanelContainer,
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        theme::SurfacePaint(theme::Role::Surface),
                    ));
                });

            // ── 5. Filter bar (at bottom) ─────────────────────────────────
            root.spawn((
                FilterBarContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Raised),
            ));

            // ── 6. Footer ─────────────────────────────────────────────────
            root.spawn((
                FooterContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Raised),
                theme::BorderPaint(theme::Role::Border),
            ))
            .with_child((
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Muted),
                FooterHint,
            ));
        });
}

// ---------------------------------------------------------------------------
// update_crafting_ui — regular Update system
// ---------------------------------------------------------------------------

/// Rebuild crafting menu content whenever `CraftState` or `CategoryIndex` changes.
pub fn update_crafting_ui(
    mut commands: Commands,
    state: Res<CraftState>,
    model: Res<super::crafting_state::CraftModel>,
    cat_index: Res<CategoryIndex>,
    def_world: Res<DefinitionWorld>,
    mut containers: CraftingContainers,
    mut cache: Local<CraftingPresentation>,
    defs: ItemDetailQueries,
    quality_registry: Res<QualityRegistry>,
    skill_registry: Res<SkillRegistry>,
    ammo_registry: Res<AmmoTypeRegistry>,
    body_part_registry: Res<BodyPartRegistry>,
    comestible_registry: Res<ComestibleRegistry>,
    theme: Res<UiTheme>,
) {
    let Ok(root) = containers.root.single() else {
        return;
    };
    let Ok(_header) = containers.header.single() else {
        return;
    };
    let Ok(cat_tabs) = containers.cat_tabs.single() else {
        return;
    };
    let Ok(sub_tabs) = containers.sub_tabs.single() else {
        return;
    };
    let Ok((list, mut virtual_list, mut selected_row, mut position, computed, mut retained)) =
        containers.list.single_mut()
    else {
        return;
    };
    let Ok(detail) = containers.detail.single() else {
        return;
    };
    let Ok(item_detail) = containers.item_detail.single() else {
        return;
    };
    let Ok(filter_bar) = containers.filter.single() else {
        return;
    };
    let Ok(_footer) = containers.footer.single() else {
        return;
    };

    let root_changed = cache.root != Some(root);
    let tabs_changed = root_changed || cat_index.is_changed() || theme.is_changed();
    let content_changed = root_changed
        || state.is_changed()
        || model.is_changed()
        || cat_index.is_changed()
        || theme.is_changed()
        || def_world.is_changed()
        || quality_registry.is_changed()
        || skill_registry.is_changed()
        || ammo_registry.is_changed()
        || body_part_registry.is_changed()
        || comestible_registry.is_changed();
    if !content_changed && !virtual_list.is_changed() {
        return;
    }
    cache.filter.update(
        &model,
        &state,
        &cat_index,
        (model.last_changed().get(), cat_index.last_changed().get()),
    );
    let category = (
        cat_index.selected_top,
        cat_index.selected_sub,
        state.filter.clone(),
        state.show_all,
    );
    let reset = cache.root != Some(root) || cache.category != category;
    cache.root = Some(root);
    cache.category = category;
    sync_virtual_pane(
        &mut virtual_list,
        &mut selected_row,
        &mut position,
        computed,
        cache.filter.indices.len(),
        state.focus,
        reset,
    );

    let focus = state.focus;
    let show_all = state.show_all;
    let filtering = state.filtering;
    let filter = state.filter.clone();
    let focus_zone = cat_index.focus_zone;
    let last_message = state.last_message.clone();

    // Determine current category/subcategory
    let sel_top = cat_index
        .selected_top
        .min(cat_index.top_categories.len().saturating_sub(1));
    let current_top = cat_index
        .top_categories
        .get(sel_top)
        .cloned()
        .unwrap_or_default();

    // Collect subcategories for the selected top category
    let subcats_for_top: Vec<String> = cat_index
        .sub_recipes
        .keys()
        .filter(|(top, _)| top == &current_top)
        .map(|(_, sub)| sub.clone())
        .collect();

    let sel_sub = cat_index
        .selected_sub
        .min(subcats_for_top.len().saturating_sub(1));

    let total_in_cat = cache.filter.indices.len();
    let focused_entry = cache
        .filter
        .indices
        .get(focus.min(total_in_cat.saturating_sub(1)))
        .map(|&i| &model.entries[i]);

    if content_changed {
        let status = if total_in_cat > 0 {
            format!(
                "Recipe {} of {}  [{}]",
                (focus + 1).min(total_in_cat),
                total_in_cat,
                if show_all { "ALL" } else { "CRAFTABLE" }
            )
        } else {
            format!(
                "{}  [{}]",
                if filter.is_empty() {
                    "No recipes"
                } else {
                    "No matching recipes"
                },
                if show_all { "ALL" } else { "CRAFTABLE" }
            )
        };
        if let Ok(mut counter) = containers.counter.single_mut() {
            counter.set_if_neq(Text::new(status));
        }
        if let Ok(mut heading) = containers.heading.single_mut() {
            heading.set_if_neq(TextColor(theme.accent()));
        }
    }
    if tabs_changed {
        // ── Top-level category tabs ────────────────────────────────────────────
        commands
            .entity(cat_tabs)
            .despawn_children()
            .with_children(|tabs| {
                for (i, cat_name) in cat_index.top_categories.iter().enumerate() {
                    let is_active = i == sel_top;
                    let zone_highlight = focus_zone == 1 && is_active;
                    let tab_bg = if zone_highlight {
                        theme.color(theme::Role::Selection)
                    } else if is_active {
                        theme.tab_active_bg()
                    } else {
                        Color::NONE
                    };
                    let text_color = if is_active {
                        theme.accent()
                    } else {
                        theme.color(theme::Role::Muted)
                    };
                    tabs.spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                            border: UiRect::bottom(if is_active {
                                Val::Px(2.0)
                            } else {
                                Val::Px(0.0)
                            }),
                            ..default()
                        },
                        BackgroundColor(tab_bg),
                        BorderColor::all(if is_active {
                            theme.accent()
                        } else {
                            Color::NONE
                        }),
                    ))
                    .with_child((
                        Text::new(cat_name.clone()),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(text_color),
                    ));
                }
            });

        // ── Subcategory tabs ───────────────────────────────────────────────────
        commands
            .entity(sub_tabs)
            .despawn_children()
            .with_children(|tabs| {
                for (i, sub_name) in subcats_for_top.iter().enumerate() {
                    let is_active = i == sel_sub;
                    let zone_highlight = focus_zone == 2 && is_active;
                    let tab_bg = if zone_highlight {
                        theme.color(theme::Role::Selection)
                    } else if is_active {
                        theme.color(theme::Role::Selection)
                    } else {
                        Color::NONE
                    };
                    let text_color = if is_active {
                        theme.color(theme::Role::Text)
                    } else {
                        theme.color(theme::Role::Muted)
                    };
                    tabs.spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(tab_bg),
                    ))
                    .with_child((
                        Text::new(sub_name.clone()),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(text_color),
                    ));
                }
            });
    }

    // Update only visible row cells/styles; stable recipe keys retain entities.
    let mut rows = Vec::new();
    if cache.filter.indices.is_empty() {
        rows.push((
            None,
            TextRow {
                node: virtual_list.row_node(),
                background: theme.color(theme::Role::Surface),
                border: Color::NONE,
                cells: vec![RowCell::new(
                    "No recipes in this category.",
                    14.0,
                    theme.color(theme::Role::Muted),
                )],
            },
        ));
    }
    for i in virtual_list.window.0..virtual_list.window.1 {
        let entry = &model.entries[cache.filter.indices[i]];
        let mark = if entry.craftable { "+" } else { "-" };
        let label = if entry.result_count > 1 {
            format!("[{mark}] {}  x{}", entry.result_name, entry.result_count)
        } else {
            format!("[{mark}] {}", entry.result_name)
        };
        let mut name = RowCell::new(label, 15.0, theme.color(theme::Role::Text));
        name.grow = 1.0;
        rows.push((
            Some(entry.recipe_key.clone()),
            TextRow {
                node: Node {
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::horizontal(Val::Px(12.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..virtual_list.row_node()
                },
                background: if i == focus.min(total_in_cat.saturating_sub(1)) {
                    theme.item_focus_bg()
                } else {
                    theme.color(theme::Role::Surface)
                },
                border: theme.color(theme::Role::Border),
                cells: vec![
                    name,
                    RowCell::new(
                        format!("  [{}]", entry.result_id),
                        13.0,
                        theme.color(theme::Role::Muted),
                    ),
                ],
            },
        ));
    }
    retained.sync(&mut commands, list, &virtual_list, rows);

    if !content_changed {
        return;
    }

    // ── Detail panel ───────────────────────────────────────────────────────
    commands
        .entity(detail)
        .despawn_children()
        .with_children(|det| {
            if let Some(entry) = &focused_entry {
                // Title
                det.spawn((
                    Text::new(format!(
                        "{}{}",
                        entry.result_name,
                        if entry.result_count > 1 {
                            format!("  x{}", entry.result_count)
                        } else {
                            String::new()
                        }
                    )),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Accent),
                ));

                // ID
                det.spawn((
                    Text::new(format!("[{}]", entry.result_id)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));

                // Craftability
                if entry.craftable {
                    det.spawn((
                        Text::new("Craftable: YES"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        theme::TextPaint(theme::Role::Positive),
                    ));
                }

                // Time
                if entry.time_turns > 0 {
                    det.spawn((
                        Text::new(format!("Time: {} turns", entry.time_turns)),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        theme::TextPaint(theme::Role::Muted),
                    ));
                }

                // Divider
                det.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    theme::SurfacePaint(theme::Role::Border),
                ));

                // Components
                det.spawn((
                    Text::new("Components:"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
                if entry.components_text.is_empty() {
                    det.spawn((
                        Text::new("  (none)"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        theme::TextPaint(theme::Role::Muted),
                    ));
                } else {
                    for line in &entry.components_text {
                        det.spawn((
                            Text::new(line.clone()),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            theme::TextPaint(theme::Role::Text),
                        ));
                    }
                }

                // Tool qualities
                if !entry.qualities_text.is_empty() {
                    det.spawn((
                        Text::new("Tool qualities:"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        theme::TextPaint(theme::Role::Muted),
                    ));
                    for line in &entry.qualities_text {
                        det.spawn((
                            Text::new(line.clone()),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            theme::TextPaint(theme::Role::Text),
                        ));
                    }
                }

                // Cannot craft reason
                if !entry.craftable {
                    det.spawn((Node {
                        margin: UiRect::top(Val::Px(6.0)),
                        ..default()
                    },))
                        .with_child((
                            Text::new(format!("Cannot craft: {}", entry.reason)),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            theme::TextPaint(theme::Role::Danger),
                        ));
                }

                // Last craft message floats to bottom
                if let Some(msg) = &last_message {
                    det.spawn((Node {
                        flex_grow: 1.0,
                        align_items: AlignItems::End,
                        ..default()
                    },))
                        .with_child((
                            Text::new(msg.clone()),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            theme::TextPaint(theme::Role::Positive),
                        ));
                }
            } else {
                det.spawn((
                    Text::new("Select a recipe"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
            }
        });

    // ── Item detail panel (shared widget) ────────────────────────────────
    commands
        .entity(item_detail)
        .despawn_children()
        .with_children(|d| {
            let result_id = focused_entry
                .as_ref()
                .map(|e| e.result_id.as_str())
                .unwrap_or("");
            let def_entity = if result_id.is_empty() {
                None
            } else {
                def_world.entity_by_str(result_id)
            };

            let Some(def) = def_entity else {
                d.spawn((
                    Text::new("Item info"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
                return;
            };

            let name_str = focused_entry
                .as_ref()
                .map(|e| e.result_name.as_str())
                .unwrap_or("");
            spawn_item_detail(
                d,
                name_str,
                result_id,
                def,
                &defs,
                &quality_registry,
                &skill_registry,
                &ammo_registry,
                &body_part_registry,
                &comestible_registry,
            );
        });

    // ── Filter bar (bottom) ────────────────────────────────────────────────
    commands
        .entity(filter_bar)
        .despawn_children()
        .with_children(|fb| {
            let filter_bg = if filtering {
                theme.color(theme::Role::Selection)
            } else {
                Color::NONE
            };
            let filter_text = if filtering {
                format!("Filter: {}_", filter)
            } else if filter.is_empty() {
                "[/] to filter".to_string()
            } else {
                format!("Filter: {}  (Enter=close  Esc=clear)", filter)
            };
            fb.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(filter_bg),
            ))
            .with_child((
                Text::new(filter_text),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(if filtering {
                    theme.color(theme::Role::Text)
                } else {
                    theme.color(theme::Role::Muted)
                }),
            ));
        });
}

#[derive(Default)]
pub struct CraftingPresentation {
    root: Option<Entity>,
    filter: super::crafting_state::RecipeFilter,
    category: (usize, usize, String, bool),
}
