//! # Overmap viewer — navigable map with pan/zoom and tile info on hover.
//!
//! Port of CDDA master's `overmap_ui.cpp` rendered with Bevy UI.
//!
//! ## Architecture
//!
//! - **Tile grid**: Rendered as a `bevy_ui` `Node` with `Display::Grid`.
//!   Each tile is a `Button` (for hover detection) containing a `Text` glyph.
//! - **Pan**: Arrow keys / vi-keys move the `OvermapCamera` resource.
//! - **Hover info**: A sidebar panel shows the terrain ID, coordinates, and flags
//!   for the tile under the cursor.
//! - **Z-level**: `<` / `>` keys change the viewed z-level.

use bevy::prelude::*;
use bevy_ecs::message::MessageReader;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_components::input::{GameAction, InputAction};
use cdda_context::ctx::Ctx as Screen;
use cdda_overmap_gen::pipeline::OvermapGenConfig;
use cdda_overmap::camera::OvermapCamera;
use cdda_overmap::registry::{TerrainFlags, TerrainHandle};
use cdda_overmap::TerrainQuery;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of tiles in the grid (width × height).
const GRID_COLS: usize = 61;
const GRID_ROWS: usize = 41;
const HALF_COLS: i32 = 30;
const HALF_ROWS: i32 = 20;

/// Size of each tile cell in logical pixels.
const TILE_SIZE_PX: f32 = 16.0;

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

/// Root entity for the overmap viewer UI tree.
#[derive(Component)]
pub struct OvermapViewerRoot;

/// Marker for the tile info sidebar text entity.
#[derive(Component)]
pub struct OvermapInfoPanel;

/// Marker for tile grid cell entities, storing their OMT position.
#[derive(Component)]
pub struct OvermapTileCell {
    pub grid_col: usize,
    pub grid_row: usize,
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

/// Spawn the overmap viewer UI on `OnEnter(Screen::Overmap)`.
pub fn spawn_overmap_viewer(
    mut commands: Commands,
    camera: Res<OvermapCamera>,
    _font: Res<super::UiFontHandle>,
) {
    commands
        .spawn((
            DespawnOnExit(Screen::Overmap),
            OvermapViewerRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            // ── Tile grid (left side) ───────────────────────────────────
            root.spawn((
                Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::px(GRID_COLS as u16, TILE_SIZE_PX),
                    grid_template_rows: RepeatedGridTrack::px(GRID_ROWS as u16, TILE_SIZE_PX),
                    margin: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.05, 0.05, 0.08)),
            ))
            .with_children(|grid| {
                for row in 0..GRID_ROWS {
                    for col in 0..GRID_COLS {
                        grid.spawn((
                            Button,
                            OvermapTileCell {
                                grid_col: col,
                                grid_row: row,
                            },
                            Node {
                                width: Val::Px(TILE_SIZE_PX),
                                height: Val::Px(TILE_SIZE_PX),
                                display: Display::Flex,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.1, 0.1, 0.12)),
                        ))
                        .with_child((
                            Text::new("."),
                            TextFont {
                                font_size: TILE_SIZE_PX - 2.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.4, 0.45, 0.3)),
                        ));
                    }
                }
            });

            // ── Info sidebar (right side) ──────────────────────────────
            root.spawn((
                Node {
                    width: Val::Px(280.0),
                    height: Val::Percent(100.0),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.08, 0.08, 0.10)),
            ))
            .with_children(|sidebar| {
                sidebar.spawn((
                    Text::new("OVERMAP"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    Node {
                        margin: UiRect::bottom(Val::Px(16.0)),
                        ..default()
                    },
                ));

                sidebar.spawn((
                    OvermapInfoPanel,
                    Text::new("Hover a tile to see info"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    Node { ..default() },
                ));

                sidebar.spawn((
                    Text::new("Arrow keys: pan   </>: z-level   M: back"),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.4, 0.4, 0.4)),
                    Node { ..default() },
                ));
            });
        });
}

// ---------------------------------------------------------------------------
// Update — pan camera with keyboard input
// ---------------------------------------------------------------------------

/// Handle keyboard input for panning the overmap camera.
pub fn overmap_camera_input(
    mut actions: MessageReader<InputAction>,
    mut camera: ResMut<OvermapCamera>,
) {
    for action in actions.read() {
        match &action.action {
            GameAction::NavigateUp => camera.pan(0, -1),
            GameAction::NavigateDown => camera.pan(0, 1),
            GameAction::NavigateLeft => camera.pan(-1, 0),
            GameAction::NavigateRight => camera.pan(1, 0),
            // CDDA convention: < (Custom1) = ascend (z+), > (Custom2) = descend (z-)
            GameAction::Custom(1) => {
                let z = camera.z.saturating_add(1);
                camera.set_z(z);
            }
            GameAction::Custom(2) => {
                let z = camera.z.saturating_sub(1);
                camera.set_z(z);
            }
            // Cancel and OpenMap are handled by the nav system to pop the context
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Update — refresh info panel on hover
// ---------------------------------------------------------------------------

/// Updates the info panel text when the user hovers over a tile.
/// Separate system to avoid scheduler conflicts with tile refresh.
pub fn update_overmap_info_panel(
    terrain: TerrainQuery,
    camera: Res<OvermapCamera>,
    config: Res<OvermapGenConfig>,
    mut info_q: Query<&mut Text, With<OvermapInfoPanel>>,
    interaction_q: Query<(&OvermapTileCell, &Interaction), Changed<Interaction>>,
) {
    let (tl_x, tl_y) = camera.top_left();
    let mut hovered_omt: Option<(i32, i32)> = None;
    for (cell, interaction) in &interaction_q {
        if *interaction == Interaction::Hovered {
            let hx = tl_x + cell.grid_col as i32;
            let hy = tl_y + (GRID_ROWS as i32 - 1 - cell.grid_row as i32);
            hovered_omt = Some((hx, hy));
            break;
        }
    }

    if let Ok(mut info_text) = info_q.single_mut() {
        if let Some((hx, hy)) = hovered_omt {
            let handle = terrain.at(hx, hy, camera.z);
            let name = terrain.name_for(handle);
            let flags = terrain.registry.flags_for(handle);
            let flag_str = describe_flags(flags);
            let z = camera.z;

            **info_text = format!(
                "Tile: ({}, {}, {})\n\
                 Terrain: {}\n\
                 Type ID: {}\n\
                 Flags: {}\n\
                 Overmap: ({}, {})\n\
                 Seed: {}",
                hx,
                hy,
                z,
                name,
                handle.type_index(),
                flag_str,
                camera.center_overmap().x,
                camera.center_overmap().y,
                config.noise_seed,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Update — refresh tile colors and glyphs
// ---------------------------------------------------------------------------

/// Refreshes tile background colors and glyphs every frame.
pub fn update_overmap_tiles(
    terrain: TerrainQuery,
    camera: Res<OvermapCamera>,
    mut grid_tiles: Query<
        (&OvermapTileCell, &mut BackgroundColor, &Children),
        Without<OvermapInfoPanel>,
    >,
    mut text_q: Query<&mut Text>,
) {
    let (tl_x, tl_y) = camera.top_left();

    for (cell, mut bg, children) in &mut grid_tiles {
        // Compute world OMT position from stable grid coordinates, not query order
        let omt_x = tl_x + cell.grid_col as i32;
        let omt_y = tl_y + (GRID_ROWS as i32 - 1 - cell.grid_row as i32);

        let handle = terrain.at(omt_x, omt_y, camera.z);
        let is_center = omt_x == camera.center_x && omt_y == camera.center_y;
        let is_null = handle == TerrainHandle::NULL;

        let flags = terrain.registry.flags_for(handle);
        let color = if is_center {
            Color::srgb(1.0, 0.3, 0.3) // red — camera center
        } else if is_null {
            Color::srgb(0.05, 0.05, 0.08) // void
        } else if flags.contains(TerrainFlags::HIGHWAY) {
            Color::srgb(0.45, 0.40, 0.35) // tan-brown
        } else if flags.contains(TerrainFlags::ROAD) {
            Color::srgb(0.4, 0.35, 0.3) // dark tan
        } else if flags.contains(TerrainFlags::BRIDGE) {
            Color::srgb(0.5, 0.45, 0.35) // light bridge
        } else if flags.contains(TerrainFlags::RAILROAD) {
            Color::srgb(0.35, 0.32, 0.30) // rail brown
        } else if flags.contains(TerrainFlags::RIVER) {
            Color::srgb(0.2, 0.3, 0.7) // blue
        } else if flags.contains(TerrainFlags::OCEAN) {
            Color::srgb(0.1, 0.15, 0.55) // deep blue
        } else if flags.contains(TerrainFlags::LAKE) {
            Color::srgb(0.1, 0.2, 0.6) // medium blue
        } else if flags.contains(TerrainFlags::FOREST) && flags.contains(TerrainFlags::LAKE) {
            Color::srgb(0.2, 0.35, 0.25) // swamp green
        } else if flags.contains(TerrainFlags::FOREST) {
            Color::srgb(0.15, 0.45, 0.15) // forest green
        } else if flags.contains(TerrainFlags::UNDERGROUND)
            || flags.contains(TerrainFlags::SEWER)
            || flags.contains(TerrainFlags::SUBWAY)
        {
            Color::srgb(0.28, 0.25, 0.30) // underground gray-purple
        } else if flags.contains(TerrainFlags::IMPASSABLE) {
            Color::srgb(0.35, 0.20, 0.20) // dark red
        } else {
            Color::srgb(0.22, 0.22, 0.20) // default field
        };

        bg.0 = color;

        if let Some(&child) = children.first() {
            if let Ok(mut text) = text_q.get_mut(child) {
                let glyph = tile_glyph(handle, &terrain);
                **text = glyph;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a terrain handle to a Unicode glyph for the overmap tile grid.
///
/// Uses `TerrainFlags` for robust classification (no fragile string matching).
/// Priority order matches CDDA `overmap_ui.cpp` tile rendering conventions.
fn tile_glyph(handle: TerrainHandle, terrain: &TerrainQuery) -> String {
    if handle == TerrainHandle::NULL {
        return " ".to_string();
    }
    let flags = terrain.registry.flags_for(handle);

    if flags.contains(TerrainFlags::HIGHWAY) {
        "\u{2550}".to_string() // ═ double horizontal
    } else if flags.contains(TerrainFlags::BRIDGE) {
        "\u{2550}".to_string() // ═
    } else if flags.contains(TerrainFlags::ROAD) {
        "#".to_string()
    } else if flags.contains(TerrainFlags::RAILROAD) {
        "\u{2500}".to_string() // ─
    } else if flags.contains(TerrainFlags::RIVER) {
        "~".to_string()
    } else if flags.contains(TerrainFlags::OCEAN) {
        "~".to_string()
    } else if flags.contains(TerrainFlags::LAKE) {
        "~".to_string()
    } else if flags.contains(TerrainFlags::SEWER) {
        "\u{2591}".to_string() // ░
    } else if flags.contains(TerrainFlags::SUBWAY) {
        "\u{2592}".to_string() // ▒
    } else if flags.contains(TerrainFlags::UNDERGROUND) {
        "\u{2593}".to_string() // ▓
    } else if flags.contains(TerrainFlags::IMPASSABLE) {
        "\u{2588}".to_string() // █
    } else {
        // Fallback: first character of the terrain string ID.
        terrain
            .registry
            .string_id_for(handle)
            .and_then(|id| id.chars().next())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
    }
}

/// Build a human-readable flags string for the info panel.
///
/// Ordered by CDDA debug overmap display convention: infrastructure first,
/// then water, then biomes, then underground, then meta flags.
fn describe_flags(flags: TerrainFlags) -> String {
    let mut parts = Vec::new();
    if flags.contains(TerrainFlags::HIGHWAY) {
        parts.push("highway");
    }
    if flags.contains(TerrainFlags::ROAD) {
        parts.push("road");
    }
    if flags.contains(TerrainFlags::BRIDGE) {
        parts.push("bridge");
    }
    if flags.contains(TerrainFlags::RAILROAD) {
        parts.push("railroad");
    }
    if flags.contains(TerrainFlags::RIVER) {
        parts.push("river");
    }
    if flags.contains(TerrainFlags::LAKE) {
        parts.push("lake");
    }
    if flags.contains(TerrainFlags::OCEAN) {
        parts.push("ocean");
    }
    if flags.contains(TerrainFlags::FOREST) {
        parts.push("forest");
    }
    if flags.contains(TerrainFlags::SEWER) {
        parts.push("sewer");
    }
    if flags.contains(TerrainFlags::SUBWAY) {
        parts.push("subway");
    }
    if flags.contains(TerrainFlags::UNDERGROUND) {
        parts.push("underground");
    }
    if flags.contains(TerrainFlags::IMPASSABLE) {
        parts.push("impassable");
    }
    if flags.contains(TerrainFlags::MANHOLE) {
        parts.push("manhole");
    }
    if flags.contains(TerrainFlags::LINE_DRAWING) {
        parts.push("line");
    }
    if parts.is_empty() {
        "field".to_string()
    } else {
        parts.join(", ")
    }
}
