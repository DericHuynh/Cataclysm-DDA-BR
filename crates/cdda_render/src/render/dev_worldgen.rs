//! ASCII renderer for the dev-worldgen building showcase.
//!
//! Uses `Text2d` (world-space) with explicit font loading — the same
//! pattern as the Bevy 0.18 Text2d example. Spawned on OnEnter(Gameplay),
//! updated on move.

use crate::context::ctx::Ctx as Screen;
use crate::context::nav::{ctx_def, FocusedCommandIndex};
use crate::data::interner::ItemTypeRegistry;
use crate::render::theme::{self, UiTheme};
use crate::render::tiles::TileRegistry;
use bevy::prelude::*;
use bevy::text::LineBreak;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_components::actor::HandCount;
use cdda_components::def::ItemSymbol;
use cdda_components::def::ItemVolume;
use cdda_components::dev::{DevCamera, DevGroundItemName, DevPlayer};
use cdda_components::item::{
    ContainerContents, Invlet, ItemTypeId, MountedPockets, WieldedItems, FLOOR_CAP_ML,
};
use cdda_components::sim::WorldPosition;
use std::collections::HashMap;
use tracing::info;

// Viewport size in OMT tiles

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct DevCmdButton(usize);

#[derive(Component)]
pub(crate) struct DevStatusBar;

pub const VIEW_COLS: usize = 40;
pub const VIEW_ROWS: usize = 24;

// ---------------------------------------------------------------------------
// DevWorldgen menu screen
// ---------------------------------------------------------------------------

pub fn spawn_dev_menu(
    mut commands: Commands,
    focused: Res<FocusedCommandIndex>,
    theme: Res<UiTheme>,
) {
    let def = ctx_def(Screen::DevWorldgen);

    commands
        .spawn((
            DespawnOnExit(Screen::DevWorldgen),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(theme::MENU_BG),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(def.title),
                TextFont { font_size: 34.0, ..default() },
                TextColor(theme.accent2()),
                TextLayout::new_with_justify(Justify::Center),
                Node { margin: UiRect::bottom(Val::Px(48.0)), ..default() },
            ));

            parent.spawn((
                Text::new("Generates a showcase world with one of every city building.\nArrow keys navigate, Enter to start."),
                TextFont { font_size: 18.0, ..default() },
                TextColor(theme::TEXT_DIM),
                TextLayout::new_with_justify(Justify::Center),
                Node { margin: UiRect::bottom(Val::Px(32.0)), ..default() },
            ));

            for (i, cmd) in def.commands.iter().enumerate() {
                let display = match cmd.hotkey {
                    Some(ch) => format!("{}) {}", ch, cmd.label),
                    None => format!("   {}", cmd.label),
                };
                let is_focused = i == focused.current();

                parent.spawn((
                    DevCmdButton(i),
                    Button,
                    Node {
                        width: Val::Percent(50.0),
                        height: Val::Px(54.0),
                        display: Display::Flex,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(24.0)),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(if is_focused { theme::BUTTON_FOCUS_BG } else { theme::BUTTON_BG }),
                    BorderColor::all(if is_focused { theme::TEXT_BRIGHT } else { Color::NONE }),
                ))
                .with_child((
                    Text::new(display),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(theme::TEXT_BRIGHT),
                ));
            }
        });
}

pub(crate) fn sync_dev_menu_focus(
    focused: Res<FocusedCommandIndex>,
    _theme: Res<UiTheme>,
    mut buttons: Query<(&DevCmdButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    let current = focused.current();
    for (btn, mut bg, mut border) in &mut buttons {
        if btn.0 == current {
            bg.0 = theme::BUTTON_FOCUS_BG;
            let c = theme::TEXT_BRIGHT;
            border.top = c;
            border.right = c;
            border.bottom = c;
            border.left = c;
        } else {
            bg.0 = theme::BUTTON_BG;
            border.top = Color::NONE;
            border.right = Color::NONE;
            border.bottom = Color::NONE;
            border.left = Color::NONE;
        }
    }
}
// ---------------------------------------------------------------------------
// Tilemap viewport — individual Sprites using real tileset images
// ---------------------------------------------------------------------------

const VIEW_RADIUS: i32 = 15;
const TILE_SIZE: f32 = 32.0;

/// Tracks which viewport cell and z-level a terrain tile entity renders.
#[derive(Component, Clone)]
pub(crate) struct DevTileCell {
    pub col: i32,
    pub row: i32,
    pub z: i32,
}

/// Links a ground-item visual entity back to its source ground-item entity.
#[derive(Component)]
pub(crate) struct DevGroundItemVisual(Entity);

pub fn spawn_ascii_view(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    camera: Res<DevCamera>,
) {
    // Tiles and ground items are spawned on first update_ascii_view call,
    // then updated in-place every frame.  No initial spawn here.

    info!("Dev-worldgen sprites: viewport initialised");

    let font_handle: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");
    commands.spawn((
        DevStatusBar,
        DespawnOnExit(Screen::Gameplay),
        Text2d::new(status_text(camera.x, camera.y, camera.z)),
        TextFont {
            font: font_handle,
            font_size: 13.0,
            ..default()
        },
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        TextBackgroundColor(Color::BLACK.with_alpha(0.7)),
        Transform::from_translation(Vec3::new(-400.0, -340.0, 10.0)),
    ));
}

// Grass-green for empty loaded bubbles; building tiles use their real image.
// Cursor cross-hair tint.
const COLOR_CURSOR: Color = Color::srgb(1.0, 0.3, 0.3);
/// Bright yellow for item ASCII glyphs rendered in the world viewport.
const COLOR_ITEM_GLYPH: Color = Color::srgb(0.95, 0.85, 0.10);
/// Light stone/concrete for empty floor tiles (all non-building positions).
const COLOR_FLOOR: Color = Color::srgb(0.22, 0.22, 0.20);

/// Spawn or update a single terrain tile entity for a viewport cell.
///
/// If `entity` is `Some`, the existing entity's components are updated in-place
/// (children are despawned/recreated as needed for ASCII-fallback text).
/// If `None`, a new entity is spawned.
fn place_terrain_tile(
    commands: &mut Commands,
    entity: Option<Entity>,
    _registry: &TileRegistry,
    cx: i32,
    cy: i32,
    cz: i32,
    col: i32,
    row: i32,
) -> Entity {
    let bx = cx + col;
    let by = cy - row;
    let is_cursor = bx == cx && by == cy;
    let base_x = col as f32 * TILE_SIZE;
    let base_y = row as f32 * TILE_SIZE;
    let cell = DevTileCell { col, row, z: cz };

    // WorldMap was deleted — render empty floor tiles for now.
    let color = if is_cursor { COLOR_CURSOR } else { COLOR_FLOOR };
    let sprite = Sprite {
        custom_size: Some(Vec2::splat(TILE_SIZE)),
        color,
        ..default()
    };
    let transform = Transform::from_translation(Vec3::new(base_x, base_y, 0.0));
    if let Some(e) = entity {
        commands
            .entity(e)
            .insert(cell)
            .insert(sprite)
            .insert(transform);
        commands.entity(e).despawn_related::<Children>();
        e
    } else {
        commands
            .spawn((cell, DespawnOnExit(Screen::Gameplay), sprite, transform))
            .id()
    }
}

/// Spawn or update a single ground-item visual entity.
///
/// If `visual_entity` is `Some`, the existing visual entity is updated in-place.
/// If `None`, a new entity is spawned and linked to `source_entity` via
/// `DevGroundItemVisual`.
fn place_ground_item_tile(
    commands: &mut Commands,
    visual_entity: Option<Entity>,
    source_entity: Entity,
    registry: &TileRegistry,
    type_id: Option<&ItemTypeId>,
    symbol: Option<&ItemSymbol>,
    name: Option<&DevGroundItemName>,
    col: i32,
    row: i32,
    item_type_registry: &ItemTypeRegistry,
) -> Entity {
    let base_x = col as f32 * TILE_SIZE;
    let base_y = row as f32 * TILE_SIZE;
    let cdda_id = type_id
        .and_then(|t| item_type_registry.resolve(t.0))
        .unwrap_or("");

    if !cdda_id.is_empty() && registry.has_tile(cdda_id) {
        let info = registry.tile_info(cdda_id);
        let sprite = Sprite {
            image: info.image.clone(),
            custom_size: Some(info.sprite_size()),
            ..default()
        };
        let transform = Transform::from_translation(Vec3::new(
            base_x + info.bevy_offset().x,
            base_y + info.bevy_offset().y,
            1.0,
        ));
        if let Some(e) = visual_entity {
            commands.entity(e).insert(sprite).insert(transform);
            commands.entity(e).remove::<(Text2d, TextFont, TextColor)>();
            e
        } else {
            commands
                .spawn((
                    DevGroundItemVisual(source_entity),
                    DespawnOnExit(Screen::Gameplay),
                    sprite,
                    transform,
                ))
                .id()
        }
    } else {
        let sym = symbol
            .map(|s| s.0)
            .or_else(|| name.and_then(|n| n.0.chars().next()))
            .unwrap_or('?');
        let text = Text2d::new(sym.to_string());
        let font = TextFont {
            font_size: 22.0,
            ..default()
        };
        let color = TextColor(COLOR_ITEM_GLYPH);
        let transform = Transform::from_translation(Vec3::new(base_x, base_y, 1.0));
        if let Some(e) = visual_entity {
            commands
                .entity(e)
                .insert(text)
                .insert(font)
                .insert(color)
                .insert(transform);
            commands.entity(e).remove::<Sprite>();
            e
        } else {
            commands
                .spawn((
                    DevGroundItemVisual(source_entity),
                    DespawnOnExit(Screen::Gameplay),
                    text,
                    font,
                    color,
                    transform,
                ))
                .id()
        }
    }
}

pub(crate) fn update_ascii_view(
    camera: Res<DevCamera>,
    registry: Res<TileRegistry>,
    terrain_query: Query<(Entity, &DevTileCell)>,
    ground_visual_query: Query<(Entity, &DevGroundItemVisual)>,
    mut commands: Commands,
    mut status_query: Query<&mut Text2d, With<DevStatusBar>>,
    ground_items: Query<(
        Entity,
        &WorldPosition,
        Option<&ItemTypeId>,
        Option<&ItemSymbol>,
        Option<&ItemVolume>,
        Option<&DevGroundItemName>,
    )>,
    player_inv: Query<(&ContainerContents, Option<&MountedPockets>), With<DevPlayer>>,
    item_names: Query<&DevGroundItemName>,
    player_hands: Query<(&HandCount, Option<&WieldedItems>), With<DevPlayer>>,
    item_type_registry: Res<ItemTypeRegistry>,
) {
    let cz = camera.z;

    // ── Terrain tiles: update existing, spawn missing, despawn stale ──
    let mut existing_terrain: HashMap<(i32, i32, i32), Entity> = HashMap::new();
    for (e, cell) in &terrain_query {
        existing_terrain.insert((cell.col, cell.row, cell.z), e);
    }

    for row in -VIEW_RADIUS..=VIEW_RADIUS {
        for col in -VIEW_RADIUS..=VIEW_RADIUS {
            let key = (col, row, cz);
            let entity = existing_terrain.remove(&key);
            place_terrain_tile(
                &mut commands,
                entity,
                &registry,
                camera.x,
                camera.y,
                cz,
                col,
                row,
            );
        }
    }

    // Any terrain entities still in the map are no longer visible.
    for (_key, entity) in existing_terrain {
        commands.entity(entity).despawn();
    }

    // ── Ground items: update existing, spawn missing, despawn stale ──
    let mut existing_ground: HashMap<Entity, Entity> = HashMap::new();
    for (visual_e, src) in &ground_visual_query {
        existing_ground.insert(src.0, visual_e);
    }

    for (item_e, wp, type_id, symbol, _vol, name) in ground_items.iter() {
        let omt_x = wp.0.x.div_euclid(24);
        let omt_y = wp.0.y.div_euclid(24);
        let omt_z = wp.0.z.0 as i32;

        if omt_z != cz {
            continue;
        }

        let col = omt_x - camera.x;
        let row = camera.y - omt_y;

        if col.abs() > VIEW_RADIUS || row.abs() > VIEW_RADIUS {
            continue;
        }

        let visual_e = existing_ground.remove(&item_e);
        place_ground_item_tile(
            &mut commands,
            visual_e,
            item_e,
            &registry,
            type_id,
            symbol,
            name,
            col,
            row,
            &item_type_registry,
        );
    }

    // Any ground-item visuals still in the map have no matching source item.
    for (_item_e, visual_e) in existing_ground {
        commands.entity(visual_e).despawn();
    }

    // ── Status bar ──
    if let Ok(mut t) = status_query.single_mut() {
        *t = Text2d::new(status_text_with_items(
            camera.x,
            camera.y,
            cz,
            &ground_items,
            &player_inv,
            &item_names,
            &player_hands,
        ));
    }
}

fn _render_viewport(cx: i32, cy: i32, _cz: i32) -> String {
    let half_cols = (VIEW_COLS / 2) as i32;
    let half_rows = (VIEW_ROWS / 2) as i32;
    let start_x = cx - half_cols;
    let start_y = cy - half_rows;

    let mut r = String::with_capacity((VIEW_ROWS + 2) * (VIEW_COLS + 2));
    r.push('+');
    for _ in 0..VIEW_COLS {
        r.push('=');
    }
    r.push_str("+\n");

    for row in 0..VIEW_ROWS {
        r.push('|');
        for col in 0..VIEW_COLS {
            let bx = start_x + col as i32;
            let by = start_y + row as i32;
            if bx == cx && by == cy {
                r.push('@');
            } else {
                r.push(' ');
            }
        }
        r.push_str("|\n");
    }

    r.push('+');
    for _ in 0..VIEW_COLS {
        r.push('=');
    }
    r.push('+');
    r
}

fn status_text(cx: i32, cy: i32, cz: i32) -> String {
    format!("Pos: ({cx}, {cy}, z={cz}) | Building: n/a | Bubbles: n/a | Placements: n/a")
}

fn status_text_with_items(
    cx: i32,
    cy: i32,
    cz: i32,
    ground_items: &Query<(
        Entity,
        &WorldPosition,
        Option<&ItemTypeId>,
        Option<&ItemSymbol>,
        Option<&ItemVolume>,
        Option<&DevGroundItemName>,
    )>,
    player_inv: &Query<(&ContainerContents, Option<&MountedPockets>), With<DevPlayer>>,
    item_names: &Query<&DevGroundItemName>,
    player_hands: &Query<(&HandCount, Option<&WieldedItems>), With<DevPlayer>>,
) -> String {
    let base = status_text(cx, cy, cz);

    // Items at current tile
    let at_tile: Vec<(&str, u32)> = ground_items
        .iter()
        .filter(|(_, wp, _, _, _, _)| {
            wp.0.x.div_euclid(24) == cx && wp.0.y.div_euclid(24) == cy && wp.0.z.0 as i32 == cz
        })
        .map(|(_, _, _, _, vol, name)| {
            (
                name.map(|n| n.0.as_str()).unwrap_or("?"),
                vol.map(|v| v.0).unwrap_or(0),
            )
        })
        .collect();

    let floor_vol_ml: u32 = at_tile.iter().map(|(_, v)| *v).sum();
    let floor_pct = (floor_vol_ml * 100) / FLOOR_CAP_ML.max(1);
    let names: Vec<&str> = at_tile.iter().map(|(n, _)| *n).collect();
    let ground_str = if names.is_empty() {
        format!(
            "none  [floor {}/{} L ({}%)]",
            floor_vol_ml / 1000,
            FLOOR_CAP_ML / 1000,
            floor_pct
        )
    } else {
        format!(
            "{}  [floor {}/{} L ({}%)]",
            names.join(", "),
            floor_vol_ml / 1000,
            FLOOR_CAP_ML / 1000,
            floor_pct
        )
    };

    // Inventory contents — collect items with Invlet from ContainerContents
    let inv_contents: Vec<String> = player_inv
        .single()
        .map(|(cc, _mp)| {
            let items: Vec<Entity> = cc.iter().collect();
            let mut pairs: Vec<(char, String)> = items
                .iter()
                .filter_map(|&entity| {
                    let name = item_names
                        .get(entity)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|_| "?".to_string());
                    Some(('?', format!("?:{name}")))
                })
                .collect();
            pairs.sort_by_key(|(c, _)| *c);
            pairs.into_iter().map(|(_, s)| s).collect()
        })
        .unwrap_or_default();
    let inv_str = if inv_contents.is_empty() {
        "empty".to_string()
    } else {
        inv_contents.join(" ")
    };

    // Hand slot summary
    let hand_str = player_hands
        .single()
        .map(|(hc, wi)| {
            let held = wi.map(|w| w.iter().count()).unwrap_or(0);
            format!("{}/{} hands used", held, hc.0)
        })
        .unwrap_or_else(|_| "? hands".to_string());

    format!(
        "{base}\nHere: {ground_str}\nInv: {inv_str}  [{hand_str}]\nArrows:move | g:pickup | d:drop | Esc:back"
    )
}
