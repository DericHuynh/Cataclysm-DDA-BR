//! Three-panel inventory screen.
//!
//! Layout (matches CDDA spirit):
//!   LEFT  — all items in body pockets (scrollable, keyboard-navigable)
//!   TOP-RIGHT  — wielded items
//!   BOTTOM-RIGHT — worn items
//!
//! Spawned on `OnEnter(Screen::Inventory)`, auto-despawned via `DespawnOnExit`.

use std::collections::HashMap;

use super::FooterHint;
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_context::ContextActions;
use cdda_data::interner::ItemTypeRegistry;
use cdda_input::ActiveKeybindings;
use cdda_input::BindableAction;
use crate::render::theme::{self, UiTheme};
use crate::render::tiles::TileRegistry;
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_components::def::{ItemName, ItemSymbol};
use cdda_components::dev::{DevGroundItemName, DevPlayer};
use cdda_components::item::{
    ContainerContents, InProgressCraft, InventoryFocus, Invlet, ItemType, MountedPockets,
    StackCount, WieldedBy, WieldedItems, WornBy,
};

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

/// Root of the pocket/item list area — rebuilt every frame.
#[derive(Component)]
pub(crate) struct InvListContainer;

/// Root of the wielded-items panel — rebuilt every frame.
#[derive(Component)]
pub(crate) struct InvWieldedContainer;

/// Root of the worn-items panel — rebuilt every frame.
#[derive(Component)]
pub(crate) struct InvWornContainer;

// ---------------------------------------------------------------------------
// CddaScreen impl

// ---------------------------------------------------------------------------
// InventoryScreen — CddaScreen trait implementation
// ---------------------------------------------------------------------------

pub struct InventoryScreen;

impl CddaScreen for InventoryScreen {
    const CTX: Ctx = Ctx::Inventory;
    const ACTIONS: &'static [(&'static str, BindableAction)] = &[
        ("navigate", BindableAction::NavigateUp),
        ("switch panel", BindableAction::NavigateNextTab),
        ("examine", BindableAction::Confirm),
        ("wield", BindableAction::UseItem),
        ("detail", BindableAction::Examine),
    ];

    fn spawn(world: &mut World) {
        spawn_inventory_screen(world);
    }

    fn update(world: &mut World) {
        update_inventory_screen(world);
    }
}

// ---------------------------------------------------------------------------
// Spawn (OnEnter)
// ---------------------------------------------------------------------------

pub fn spawn_inventory_screen(world: &mut World) {
    let theme = world.resource::<UiTheme>().clone();
    let active_keys = world.resource::<ActiveKeybindings>();
    let cancel_key_str = active_keys.key_for(BindableAction::Cancel);
    let mut hints = format!("[{}] close", cancel_key_str);
    for entry in &world.resource::<ContextActions>().actions {
        let key = active_keys.key_for(entry.action);
        hints.push_str(&format!("  [{}] {}", key, entry.label));
    }
    let font_handle = world.resource::<super::UiFontHandle>().0.clone();
    let mut cmds = world.commands();
    spawn_inventory_ui(&mut cmds, &hints, &font_handle, &theme);
}

fn spawn_inventory_ui(
    commands: &mut Commands,
    hints: &str,
    ui_font: &Option<Handle<Font>>,
    theme: &UiTheme,
) {
    commands
        .spawn((
            DespawnOnExit(Ctx::Inventory),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(theme::BG),
        ))
        .with_children(|root| {
            // ── Title bar ─────────────────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::new(
                        Val::Px(20.0),
                        Val::Px(20.0),
                        Val::Px(10.0),
                        Val::Px(10.0),
                    ),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(theme::HEADER_BG),
            ))
            .with_children(|h| {
                h.spawn((
                    Text::new("INVENTORY"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(theme.accent()),
                ));
            });

            // ── Main area (left panel + right panels side-by-side) ────────
            root.spawn((Node {
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                overflow: Overflow::clip(),
                ..default()
            },))
                .with_children(|main| {
                    // ── LEFT PANEL — pocket items ─────────────────────────────
                    main.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            border: UiRect::right(Val::Px(1.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BorderColor::all(theme::DIVIDER),
                    ))
                    .with_children(|left| {
                        // Column header
                        left.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::new(
                                    Val::Px(14.0),
                                    Val::Px(14.0),
                                    Val::Px(4.0),
                                    Val::Px(4.0),
                                ),
                                border: UiRect::bottom(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(theme::PANEL_HEADER_BG),
                            BorderColor::all(theme::DIVIDER),
                        ))
                        .with_child((
                            Text::new("ITEMS"),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(theme::TEXT_DIM),
                        ));

                        left.spawn((
                            InvListContainer,
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                overflow: Overflow::clip_y(),
                                ..default()
                            },
                        ));
                    });

                    // ── RIGHT PANELS — wielded + worn stacked vertically ───────
                    main.spawn((Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(300.0),
                        flex_shrink: 0.0,
                        min_height: Val::Px(0.0),
                        ..default()
                    },))
                        .with_children(|right| {
                            // ── Wielded panel ──────────────────────────────────────
                            right
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        border: UiRect::bottom(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BorderColor::all(theme::DIVIDER),
                                    BackgroundColor(theme::PANEL_BG),
                                ))
                                .with_children(|wp| {
                                    wp.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            padding: UiRect::new(
                                                Val::Px(14.0),
                                                Val::Px(14.0),
                                                Val::Px(4.0),
                                                Val::Px(4.0),
                                            ),
                                            border: UiRect::bottom(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(theme::PANEL_HEADER_BG),
                                        BorderColor::all(theme::DIVIDER),
                                    ))
                                    .with_child((
                                        Text::new("WIELDED"),
                                        TextFont {
                                            font_size: 13.0,
                                            ..default()
                                        },
                                        TextColor(theme.accent2()),
                                    ));

                                    wp.spawn((
                                        InvWieldedContainer,
                                        Node {
                                            flex_direction: FlexDirection::Column,
                                            width: Val::Percent(100.0),
                                            flex_grow: 1.0,
                                            overflow: Overflow::clip_y(),
                                            ..default()
                                        },
                                    ));
                                });

                            // ── Worn panel ─────────────────────────────────────────
                            right
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        ..default()
                                    },
                                    BackgroundColor(theme::PANEL_BG),
                                ))
                                .with_children(|worn| {
                                    worn.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            padding: UiRect::new(
                                                Val::Px(14.0),
                                                Val::Px(14.0),
                                                Val::Px(4.0),
                                                Val::Px(4.0),
                                            ),
                                            border: UiRect::bottom(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(theme::PANEL_HEADER_BG),
                                        BorderColor::all(theme::DIVIDER),
                                    ))
                                    .with_child((
                                        Text::new("WORN"),
                                        TextFont {
                                            font_size: 13.0,
                                            ..default()
                                        },
                                        TextColor(theme.accent2()),
                                    ));

                                    worn.spawn((
                                        InvWornContainer,
                                        Node {
                                            flex_direction: FlexDirection::Column,
                                            width: Val::Percent(100.0),
                                            flex_grow: 1.0,
                                            overflow: Overflow::clip_y(),
                                            ..default()
                                        },
                                    ));
                                });
                        });
                });

            // ── Footer hint (pre-computed by caller) ────────────────────
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
                super::ui_font(ui_font, 13.0),
                TextColor(theme::TEXT_DIM),
                FooterHint,
            ));
        });
}

// ---------------------------------------------------------------------------
// Update — for CddaScreen trait
// ---------------------------------------------------------------------------

/// Pre-collected item display data, extracted from ECS queries before
/// `Commands` is obtained (avoids borrowing conflicts with `&mut World`).
struct ItemRowData {
    name: String,
    qty: u32,
    cdda_id: String,
    sym: char,
    craft_display: Option<String>,
}

fn collect_item_display_data(
    items: &[(char, Entity)],
    world: &mut World,
) -> Vec<(char, Entity, ItemRowData)> {
    let mut item_names = world.query::<&DevGroundItemName>();
    let mut item_name_fallback = world.query::<&ItemName>();
    let mut item_counts = world.query::<&StackCount>();
    let mut item_type_ids = world.query::<&ItemType>();
    let mut item_symbols = world.query::<&ItemSymbol>();
    let mut in_progress_crafts = world.query::<&InProgressCraft>();

    items
        .iter()
        .map(|&(invlet_char, item_entity)| {
            let craft = in_progress_crafts.get(world, item_entity).ok();
            let craft_display = craft.map(|c| c.display_name());
            let name: String = if let Some(ref s) = craft_display {
                s.clone()
            } else {
                item_names
                    .get(world, item_entity)
                    .ok()
                    .map(|n| n.0.clone())
                    .or_else(|| {
                        item_name_fallback
                            .get(world, item_entity)
                            .ok()
                            .map(|n| n.0.clone())
                    })
                    .unwrap_or_else(|| "?".to_string())
            };
            let qty = item_counts
                .get(world, item_entity)
                .map(|s| s.get())
                .unwrap_or(1);
            let cdda_id: String = item_type_ids
                .get(world, item_entity)
                .map(|t| {
                    world
                        .resource::<ItemTypeRegistry>()
                        .resolve(t.0)
                        .unwrap_or("?")
                        .to_string()
                })
                .unwrap_or_default();
            let sym: char = item_symbols
                .get(world, item_entity)
                .map(|s| s.0)
                .or_else(|_| {
                    item_names
                        .get(world, item_entity)
                        .map(|n| n.0.chars().next().unwrap_or('?'))
                })
                .unwrap_or('?');

            (
                invlet_char,
                item_entity,
                ItemRowData {
                    name,
                    qty,
                    cdda_id,
                    sym,
                    craft_display,
                },
            )
        })
        .collect()
}

fn build_item_panel_from_data(
    commands: &mut Commands,
    container: Entity,
    items: &[(char, Entity, ItemRowData)],
    has_focus: bool,
    focus_index: usize,
    compact: bool,
    registry: &TileRegistry,
    theme: &UiTheme,
) {
    if items.is_empty() {
        return;
    }

    let font_size = if compact { 14.0 } else { 15.0 };
    let icon_size = if compact { 20.0 } else { 24.0 };
    let pad_v = if compact { 4.0 } else { 5.0 };

    for (i, (invlet_char, _item_entity, data)) in items.iter().enumerate() {
        let is_focused = has_focus && i == focus_index;
        let is_crafting = data.craft_display.is_some();

        let row_bg = if is_focused {
            theme.item_focus_bg()
        } else if is_crafting {
            theme::ITEM_CRAFT_BG
        } else {
            theme::ITEM_BG
        };
        let text_color = if is_crafting {
            theme::TEXT_CRAFT
        } else {
            theme::TEXT_BRIGHT
        };

        let has_sprite = !data.cdda_id.is_empty() && registry.has_tile(&data.cdda_id);

        let label = format!("{:<4}  {:<28}  {}", invlet_char, data.name, data.qty);

        commands.entity(container).with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(
                        Val::Px(10.0),
                        Val::Px(10.0),
                        Val::Px(pad_v),
                        Val::Px(pad_v),
                    ),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(row_bg),
                BorderColor::all(theme::DIVIDER),
            ))
            .with_children(|row| {
                if has_sprite {
                    let info = registry.tile_info(&data.cdda_id);
                    row.spawn((
                        Node {
                            width: Val::Px(icon_size),
                            height: Val::Px(icon_size),
                            flex_shrink: 0.0,
                            margin: UiRect::right(Val::Px(8.0)),
                            ..default()
                        },
                        ImageNode {
                            image: info.image.clone(),
                            ..default()
                        },
                    ));
                } else {
                    row.spawn((
                        Node {
                            width: Val::Px(icon_size),
                            height: Val::Px(icon_size),
                            flex_shrink: 0.0,
                            margin: UiRect::right(Val::Px(8.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(theme::ICON_BG),
                    ))
                    .with_child((
                        Text::new(data.sym.to_string()),
                        TextFont {
                            font_size: font_size - 2.0,
                            ..default()
                        },
                        TextColor(theme::ICON_TEXT),
                    ));
                }

                row.spawn((
                    Text::new(label),
                    TextFont {
                        font_size,
                        ..default()
                    },
                    TextColor(text_color),
                ));
            });
        });
    }
}

pub(crate) fn update_inventory_screen(world: &mut World) {
    // ── Phase 1: Extract all query data into local variables ────────────

    // Resources
    let theme = world.resource::<UiTheme>().clone();
    let (focus_panel, focus_index) = {
        let focus = world.resource::<InventoryFocus>();
        (focus.panel, focus.index)
    };

    // Container entities
    let container_entity = {
        let mut q = world.query_filtered::<Entity, With<InvListContainer>>();
        match q.single(world) {
            Ok(e) => e,
            Err(_) => return,
        }
    };
    let wielded_entity = {
        let mut q = world.query_filtered::<Entity, With<InvWieldedContainer>>();
        match q.single(world) {
            Ok(e) => e,
            Err(_) => return,
        }
    };
    let worn_entity = {
        let mut q = world.query_filtered::<Entity, With<InvWornContainer>>();
        match q.single(world) {
            Ok(e) => e,
            Err(_) => return,
        }
    };

    // Player data — use relationships instead of Inventory hashmap
    let (mounted_pockets_entities, wielded_items_entities, worn_by_entities) = {
        let mut q = world.query_filtered::<(
            Option<&MountedPockets>,
            Option<&WieldedItems>,
            Option<&WornBy>,
        ), With<DevPlayer>>();
        match q.single(world) {
            Ok((mp, wi, wb)) => {
                let mp_entities: Vec<Entity> = mp.map(|m| m.iter().collect()).unwrap_or_default();
                let wi_entities: Vec<Entity> = wi.map(|w| w.iter().collect()).unwrap_or_default();
                let wb_entities: Vec<Entity> = wb.map(|w| w.iter().collect()).unwrap_or_default();
                (mp_entities, wi_entities, wb_entities)
            }
            Err(_) => return,
        }
    };

    // ── Build pocket item list ─────────────────────────────────────────
    let mut pocket_items: Vec<(char, Entity)> = Vec::new();

    // Collect items from mounted pockets via ContainerContents relationship
    {
        let mut pocket_contents_q = world.query::<&ContainerContents>();
        let mut invlet_q = world.query::<&Invlet>();
        let mut wielded_by_check = world.query_filtered::<Entity, With<WieldedBy>>();

        for pocket in &mounted_pockets_entities {
            if let Ok(cc) = pocket_contents_q.get(world, *pocket) {
                for item in cc.iter() {
                    // Skip wielded items
                    if wielded_by_check.get(world, item).is_ok() {
                        continue;
                    }
                    let c = invlet_q.get(world, item).map(|i| i.0).unwrap_or('?');
                    pocket_items.push((c, item));
                }
            }
        }

        // Also include items directly in the player's ContainerContents
        let player_entity = {
            let mut q = world.query_filtered::<Entity, With<DevPlayer>>();
            q.single(world).ok()
        };
        if let Some(player) = player_entity {
            if let Ok(cc) = pocket_contents_q.get(world, player) {
                for item in cc.iter() {
                    let already_listed = pocket_items.iter().any(|(_, ent)| *ent == item);
                    if !already_listed && wielded_by_check.get(world, item).is_err() {
                        let c = invlet_q.get(world, item).map(|i| i.0).unwrap_or('?');
                        pocket_items.push((c, item));
                    }
                }
            }
        }
    }
    pocket_items.sort_by_key(|(c, _)| *c);

    // ── Collect display data for pocket items ──────────────────────────
    let pocket_data = collect_item_display_data(&pocket_items, world);

    // ── Build wielded item list ─────────────────────────────────────────
    let wielded: Vec<(char, Entity)> = {
        let mut invlet_q = world.query::<&Invlet>();
        let mut v: Vec<(char, Entity)> = wielded_items_entities
            .iter()
            .map(|&e| (invlet_q.get(world, e).map(|i| i.0).unwrap_or('?'), e))
            .collect();
        v.sort_by_key(|(c, _)| *c);
        v
    };
    let wielded_data = collect_item_display_data(&wielded, world);

    // ── Build worn item list ────────────────────────────────────────────
    let worn_with_chars: Vec<(char, Entity)> = {
        let mut invlet_q = world.query::<&Invlet>();
        worn_by_entities
            .iter()
            .map(|&e| (invlet_q.get(world, e).map(|i| i.0).unwrap_or('?'), e))
            .collect()
    };
    let worn_data = collect_item_display_data(&worn_with_chars, world);

    // Clone TileRegistry for sprite lookups
    let registry = world.resource::<TileRegistry>().clone();

    // ── Phase 2: Build UI with Commands ─────────────────────────────────
    let mut cmds = world.commands();
    cmds.entity(container_entity).despawn_children();
    cmds.entity(wielded_entity).despawn_children();
    cmds.entity(worn_entity).despawn_children();

    // Left panel
    build_item_panel_from_data(
        &mut cmds,
        container_entity,
        &pocket_data,
        focus_panel == 0,
        focus_index,
        false,
        &registry,
        &theme,
    );

    if pocket_data.is_empty() {
        cmds.entity(container_entity).with_children(|p| {
            p.spawn((Node {
                padding: UiRect::axes(Val::Px(14.0), Val::Px(14.0)),
                ..default()
            },))
                .with_child((
                    Text::new("(empty)"),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
        });
    }

    // Wielded panel
    if wielded_data.is_empty() {
        cmds.entity(wielded_entity).with_children(|p| {
            p.spawn((Node {
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                ..default()
            },))
                .with_child((
                    Text::new("(nothing wielded)"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
        });
    } else {
        build_item_panel_from_data(
            &mut cmds,
            wielded_entity,
            &wielded_data,
            focus_panel == 1,
            focus_index,
            true,
            &registry,
            &theme,
        );
    }

    // Worn panel
    if worn_data.is_empty() {
        cmds.entity(worn_entity).with_children(|p| {
            p.spawn((Node {
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                ..default()
            },))
                .with_child((
                    Text::new("(nothing worn)"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
        });
    } else {
        build_item_panel_from_data(
            &mut cmds,
            worn_entity,
            &worn_data,
            false,
            0,
            true,
            &registry,
            &theme,
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
