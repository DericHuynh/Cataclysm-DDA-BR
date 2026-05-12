//! UI state machine systems — menu navigation, cursor movement.
//!
//! ## Schedule layout
//!
//! | Schedule   | System                        | Access                                       |
//! |------------|-------------------------------|----------------------------------------------|
//! | `Update`   | `menu_navigation`             | Query<&mut SelectedIndex>                    |
//! | `Update`   | `screen_and_cursor`           | Res<State<Screen>>, ResMut<ExamineCursor>    |
//!
//! Screen navigation is handled by `handle_navigation_input` in
//! `screen_nav.rs`.  This module only deals with menu scrolling and
//! the examine cursor.

use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

use cdda_core_types::core::coords::WorldPos;
use cdda_components::input::{GameAction, InputAction};

use crate::cursor::ExamineCursor;
use crate::menu::{MenuItem, SelectedIndex};
use crate::ctx::Ctx;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PAGE_SIZE: usize = 10;

// ---------------------------------------------------------------------------
// menu_navigation  (Update)
// ---------------------------------------------------------------------------

pub fn menu_navigation(
    mut action_reader: MessageReader<InputAction>,
    mut menus: Query<&mut SelectedIndex>,
    items: Query<&MenuItem>,
) {
    for event in action_reader.read() {
        match &event.action {
            GameAction::NavigateUp | GameAction::NavigateDown => {
                let dir: i32 = match &event.action {
                    GameAction::NavigateUp => -1,
                    _ => 1,
                };

                for mut selected in &mut menus {
                    let total = items.iter().filter(|item| item.enabled).count();
                    if total == 0 {
                        continue;
                    }

                    let start = selected.index as i32;
                    let mut new_idx = start;
                    for _ in 0..total {
                        new_idx = (new_idx + dir).rem_euclid(items.iter().count() as i32);
                        if items
                            .iter()
                            .nth(new_idx as usize)
                            .map_or(false, |i| i.enabled)
                        {
                            break;
                        }
                    }

                    selected.index = new_idx as usize;
                    adjust_scroll(&mut selected, total);
                }
            }
            GameAction::NavigatePageUp => {
                for mut selected in &mut menus {
                    let total = items.iter().filter(|item| item.enabled).count();
                    selected.index = selected.index.saturating_sub(PAGE_SIZE);
                    adjust_scroll(&mut selected, total);
                }
            }
            GameAction::NavigatePageDown => {
                for mut selected in &mut menus {
                    let total = items.iter().filter(|item| item.enabled).count();
                    selected.index = (selected.index + PAGE_SIZE).min(total.saturating_sub(1));
                    adjust_scroll(&mut selected, total);
                }
            }
            GameAction::NavigateHome => {
                for mut selected in &mut menus {
                    selected.index = 0;
                    selected.scroll_offset = 0;
                }
            }
            GameAction::NavigateEnd => {
                for mut selected in &mut menus {
                    let total = items.iter().filter(|item| item.enabled).count();
                    selected.index = total.saturating_sub(1);
                    adjust_scroll(&mut selected, total);
                }
            }
            _ => {}
        }
    }
}

fn adjust_scroll(selected: &mut SelectedIndex, total: usize) {
    if total == 0 {
        return;
    }
    let page_size = PAGE_SIZE;
    if selected.index < selected.scroll_offset {
        selected.scroll_offset = selected.index;
    } else if selected.index >= selected.scroll_offset + page_size {
        selected.scroll_offset = selected.index + 1 - page_size;
    }
}

// ---------------------------------------------------------------------------
// screen_and_cursor  (Update)
// ---------------------------------------------------------------------------

/// Moves the examine cursor when in `ExamineLook` mode.
///
/// Screen transitions are handled by `handle_navigation_input` —
/// this system only cares about directional movement of the examine cursor.
pub fn ctx_and_cursor(
    mut action_reader: MessageReader<InputAction>,
    screen: Res<State<Ctx>>,
    mut cursor: ResMut<ExamineCursor>,
) {
    for event in action_reader.read() {
        // ── Examine cursor movement ────────────────────────────
        if *screen.get() == Ctx::ExamineLook {
            if let Some((dx, dy)) = movement_delta(&event.action) {
                let current =
                    cursor
                        .tile
                        .unwrap_or(WorldPos::new(0, 0, cdda_core_types::core::coords::ZLevel::new(0)));
                cursor.tile = Some(WorldPos::new(current.x + dx, current.y + dy, current.z));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns (dx, dy) if the action is a directional movement.
fn movement_delta(action: &GameAction) -> Option<(i32, i32)> {
    match action {
        GameAction::Move(dir) => match dir {
            cdda_components::input::Direction::North => Some((0, -1)),
            cdda_components::input::Direction::South => Some((0, 1)),
            cdda_components::input::Direction::West => Some((-1, 0)),
            cdda_components::input::Direction::East => Some((1, 0)),
            cdda_components::input::Direction::NorthWest => Some((-1, -1)),
            cdda_components::input::Direction::NorthEast => Some((1, -1)),
            cdda_components::input::Direction::SouthWest => Some((-1, 1)),
            cdda_components::input::Direction::SouthEast => Some((1, 1)),
            _ => None,
        },
        _ => None,
    }
}
