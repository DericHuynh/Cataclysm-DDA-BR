//! ASCII renderer for the dev-worldgen building showcase.
//!
//! Uses `Text2d` (world-space) with explicit font loading — the same
//! pattern as the Bevy 0.18 Text2d example. Spawned on OnEnter(Gameplay),
//! updated on move.

use bevy::prelude::*;
use bevy::text::LineBreak;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_map::WorldMap;
use cdda_sim::systems::dev_move::DevCamera;
use cdda_sim::world_setup::WorldMapResource;
use cdda_screen::screen::Screen;
use cdda_screen::screen_nav::{screen_def, FocusedCommandIndex};
use crate::tiles::TileRegistry;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.05, 0.05, 0.07);
const ITEM_BG: Color = Color::srgb(0.08, 0.08, 0.10);
const ITEM_FOCUS_BG: Color = Color::srgb(0.25, 0.55, 0.15);
const ACCENT: Color = Color::srgb(0.85, 0.6, 0.15);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.6, 0.6, 0.6);
const FOCUSED_BORDER: Color = Color::srgb(0.95, 0.95, 0.95);

// Viewport size in OMT tiles
const VIEW_COLS: usize = 40;
const VIEW_ROWS: usize = 24;

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct DevCmdButton(usize);

#[derive(Component)]
pub(crate) struct DevStatusBar;

// ---------------------------------------------------------------------------
// DevWorldgen menu screen
// ---------------------------------------------------------------------------

pub fn spawn_dev_menu(mut commands: Commands, focused: Res<FocusedCommandIndex>) {
    let def = screen_def(Screen::DevWorldgen);

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
            BackgroundColor(BG),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(def.title),
                TextFont { font_size: 34.0, ..default() },
                TextColor(ACCENT),
                TextLayout::new_with_justify(Justify::Center),
                Node { margin: UiRect::bottom(Val::Px(48.0)), ..default() },
            ));

            parent.spawn((
                Text::new("Generates a showcase world with one of every city building.\nArrow keys navigate, Enter to start."),
                TextFont { font_size: 18.0, ..default() },
                TextColor(TEXT_DIM),
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
                    BackgroundColor(if is_focused { ITEM_FOCUS_BG } else { ITEM_BG }),
                    BorderColor::all(if is_focused { FOCUSED_BORDER } else { Color::NONE }),
                ))
                .with_child((
                    Text::new(display),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(TEXT_BRIGHT),
                ));
            }
        });
}

pub fn sync_dev_menu_focus(
    focused: Res<FocusedCommandIndex>,
    mut buttons: Query<(&DevCmdButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    let current = focused.current();
    for (btn, mut bg, mut border) in &mut buttons {
        if btn.0 == current {
            bg.0 = ITEM_FOCUS_BG;
            let c = FOCUSED_BORDER;
            border.top = c;
            border.right = c;
            border.bottom = c;
            border.left = c;
        } else {
            bg.0 = ITEM_BG;
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

#[derive(Component)]
pub(crate) struct DevTile;

pub fn spawn_ascii_view(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    camera: Res<DevCamera>,
    world_map: Res<WorldMapResource>,
    registry: Res<TileRegistry>,
) {
    spawn_tiles(&mut commands, &registry, &world_map.0, camera.x, camera.y, camera.z);

    info!(
        "Dev-worldgen sprites: bubbles={} placements={}",
        world_map.0.bubble_count(), world_map.0.placements.len(),
    );

    let font_handle: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");
    commands.spawn((
        DevStatusBar,
        DespawnOnExit(Screen::Gameplay),
        Text2d::new(status_text(&world_map.0, camera.x, camera.y, camera.z)),
        TextFont { font: font_handle, font_size: 13.0, ..default() },
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        TextBackgroundColor(Color::BLACK.with_alpha(0.7)),
        Transform::from_translation(Vec3::new(-400.0, -340.0, 10.0)),
    ));
}

// Grass-green for empty loaded bubbles; building tiles use their real image.
const COLOR_EMPTY: Color = Color::srgb(0.13, 0.30, 0.09);
// Cursor cross-hair tint.
const COLOR_CURSOR: Color = Color::srgb(1.0, 0.3, 0.3);

fn spawn_tiles(
    commands: &mut Commands,
    registry: &TileRegistry,
    wm: &WorldMap,
    cx: i32, cy: i32, cz: i32,
) {
    for row in -VIEW_RADIUS..=VIEW_RADIUS {
        for col in -VIEW_RADIUS..=VIEW_RADIUS {
            let bx = cx + col;
            let by = cy - row;

            let is_cursor = bx == cx && by == cy;
            let placement = wm.placements.get(&(bx, by, cz));
            let has_bubble = wm.bubble(bx, by, cz).is_some();

            if !is_cursor && placement.is_none() && !has_bubble {
                continue;
            }

            let (sprite, tile_offset) = if let Some(p) = placement {
                // Look up by omt_id: the JSON manifests map overmap_terrain IDs
                // (e.g. "farm_3", "abstorefront_1") directly to sprites.
                let info = registry.tile_info(&p.omt_id);
                let color = if is_cursor { COLOR_CURSOR } else { Color::WHITE };
                let sprite = Sprite {
                    image: info.image.clone(),
                    custom_size: Some(info.sprite_size()),
                    color,
                    ..default()
                };
                (sprite, info.bevy_offset())
            } else {
                let color = if is_cursor { COLOR_CURSOR } else { COLOR_EMPTY };
                let sprite = Sprite {
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    color,
                    ..default()
                };
                (sprite, Vec2::ZERO)
            };

            commands.spawn((
                DevTile,
                DespawnOnExit(Screen::Gameplay),
                sprite,
                Transform::from_translation(Vec3::new(
                    col as f32 * TILE_SIZE + tile_offset.x,
                    row as f32 * TILE_SIZE + tile_offset.y,
                    0.0,
                )),
            ));
        }
    }
}

pub fn update_ascii_view(
    camera: Res<DevCamera>,
    world_map: Res<WorldMapResource>,
    registry: Res<TileRegistry>,
    tile_query: Query<Entity, With<DevTile>>,
    mut commands: Commands,
    mut status_query: Query<&mut Text2d, With<DevStatusBar>>,
) {
    for e in &tile_query {
        commands.entity(e).despawn();
    }
    spawn_tiles(&mut commands, &registry, &world_map.0, camera.x, camera.y, camera.z);
    if let Ok(mut t) = status_query.single_mut() {
        *t = Text2d::new(status_text(&world_map.0, camera.x, camera.y, camera.z));
    }
}

fn render_viewport(wm: &WorldMap, cx: i32, cy: i32, cz: i32) -> String {
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
            } else if let Some(p) = wm.placements.get(&(bx, by, cz)) {
                r.push(omt_symbol(&p.omt_id));
            } else if wm.bubble(bx, by, cz).is_some() {
                r.push('.');
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

fn omt_symbol(omt_id: &str) -> char {
    omt_id.chars().next().unwrap_or('?')
}

fn status_text(wm: &WorldMap, cx: i32, cy: i32, cz: i32) -> String {
    let mut building_name = String::from("none");
    if let Some(p) = wm.placements.get(&(cx, cy, cz)) {
        building_name = format!("{} (omt: {})", p.building_id, p.omt_id);
    }
    format!(
        "Pos: ({}, {}, z={}) | Building: {} | Bubbles: {} | Placements: {} | Arrows: move | </>: z-level | Esc: back",
        cx, cy, cz, building_name, wm.bubble_count(), wm.placements.len()
    )
}
