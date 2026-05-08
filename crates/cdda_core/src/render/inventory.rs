//! Inventory screen — full-panel item list with keyboard navigation.
//!
//! Spawned on `OnEnter(Screen::Inventory)`, auto-despawned via
//! `DespawnOnExit`. Item rows are rebuilt every frame from the live
//! `Inventory` component so drops are reflected immediately.

use crate::core::components::item::ItemTypeId;
use crate::core::components::item::StackCount;
use crate::core::components::item::WieldedBy;
use crate::core::components::item::{Inventory, InventoryFocus};
use crate::render::tiles::TileRegistry;
use crate::screen::screen::Screen;
use crate::core::components::def::ItemSymbol;
use crate::sim::dev_worldgen::{DevGroundItemName, DevPlayer};
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;

// ---------------------------------------------------------------------------
// Colours (match dev_worldgen palette)
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.04, 0.04, 0.06);
const HEADER_BG: Color = Color::srgb(0.10, 0.10, 0.14);
const ITEM_BG: Color = Color::srgb(0.07, 0.07, 0.10);
const ITEM_FOCUS_BG: Color = Color::srgb(0.20, 0.50, 0.12);
/// Row background for items currently held in hand (wielded).
const ITEM_HELD_BG: Color = Color::srgb(0.10, 0.22, 0.40);
const ACCENT: Color = Color::srgb(0.85, 0.60, 0.15);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);
const ICON_BG: Color = Color::srgb(0.12, 0.12, 0.16);
const ICON_TEXT: Color = Color::srgb(0.90, 0.85, 0.25);

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

/// Root of the item list area — children are rebuilt every frame.
#[derive(Component)]
pub(crate) struct InvListContainer;

// ---------------------------------------------------------------------------
// Spawn (OnEnter)
// ---------------------------------------------------------------------------

pub fn spawn_inventory_screen(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(Screen::Inventory),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // ── Title bar ─────────────────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(14.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
            ))
            .with_child((
                Text::new("INVENTORY"),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(ACCENT),
            ));

            // ── Column headers ────────────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(6.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
                BorderColor::all(DIVIDER),
            ))
            .with_child((
                Text::new(format!("{:<4}  {:<28}  {}", "#", "Name", "Qty")),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(TEXT_DIM),
            ));

            // ── Scrollable item list ───────────────────────────────────────
            root.spawn((
                InvListContainer,
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
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
                BorderColor::all(DIVIDER),
            ))
            .with_child((
                Text::new("[j/k / ↑↓] navigate    [Enter / e] drop    [w] wield / unwield    [Esc / q] close"),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(TEXT_DIM),
            ));
        });
}

// ---------------------------------------------------------------------------
// Update — rebuild item rows each frame
// ---------------------------------------------------------------------------

pub(crate) fn update_inventory_screen(
    mut commands: Commands,
    focus: Res<InventoryFocus>,
    registry: Res<TileRegistry>,
    player_inv: Query<&Inventory, With<DevPlayer>>,
    item_names: Query<&DevGroundItemName>,
    item_counts: Query<&StackCount>,
    item_type_ids: Query<&ItemTypeId>,
    item_symbols: Query<&ItemSymbol>,
    container: Query<Entity, With<InvListContainer>>,
    wielded_by_check: Query<Entity, With<WieldedBy>>,
) {
    let Ok(container_entity) = container.single() else {
        return;
    };
    let Ok(inv) = player_inv.single() else {
        return;
    };

    // Rebuild the list from scratch each frame so drops appear instantly.
    commands.entity(container_entity).despawn_children();

    let mut items: Vec<(char, Entity)> = inv.invlets.iter().map(|(&c, &e)| (c, e)).collect();
    items.sort_by_key(|(c, _)| *c);

    if items.is_empty() {
        commands.entity(container_entity).with_children(|p| {
            p.spawn((Node {
                padding: UiRect::axes(Val::Px(24.0), Val::Px(14.0)),
                ..default()
            },))
                .with_child((
                    Text::new("(empty)"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));
        });
        return;
    }

    for (i, (invlet_char, item_entity)) in items.iter().enumerate() {
        let name = item_names
            .get(*item_entity)
            .map(|n| n.0.as_str())
            .unwrap_or("?");
        let qty = item_counts.get(*item_entity).map(|s| s.get()).unwrap_or(1);
        let is_focused = i == focus.index;
        let is_held = wielded_by_check.get(*item_entity).is_ok();

        let row_bg = if is_focused {
            ITEM_FOCUS_BG
        } else if is_held {
            ITEM_HELD_BG
        } else {
            ITEM_BG
        };
        let text_color = TEXT_BRIGHT;

        let cdda_id = item_type_ids
            .get(*item_entity)
            .map(|t| t.0.clone())
            .unwrap_or_default();
        let has_sprite = !cdda_id.is_empty() && registry.has_tile(&cdda_id);
        let sym: char = item_symbols
            .get(*item_entity)
            .map(|s| s.0)
            .or_else(|_| {
                item_names
                    .get(*item_entity)
                    .map(|n| n.0.chars().next().unwrap_or('?'))
            })
            .unwrap_or('?');

        let held_tag = if is_held { "[H] " } else { "    " };
        let label = format!("{held_tag}{:<4}  {:<28}  {}", invlet_char, name, qty);

        commands.entity(container_entity).with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(6.0), Val::Px(6.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(row_bg),
                BorderColor::all(DIVIDER),
            ))
            .with_children(|row| {
                // ── Icon slot ───────────────────────────────────────────────
                if has_sprite {
                    let info = registry.tile_info(&cdda_id);
                    row.spawn((
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            flex_shrink: 0.0,
                            margin: UiRect::right(Val::Px(10.0)),
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
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            flex_shrink: 0.0,
                            margin: UiRect::right(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(ICON_BG),
                    ))
                    .with_child((
                        Text::new(sym.to_string()),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(ICON_TEXT),
                    ));
                }

                // ── Item text ────────────────────────────────────────────────
                row.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(text_color),
                ));
            });
        });
    }
}
