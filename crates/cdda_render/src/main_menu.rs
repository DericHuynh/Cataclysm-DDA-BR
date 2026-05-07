//! Main menu screen — rendered with standard bevy_ui.
//!
//! Spawned on `OnEnter(Screen::MainMenu)`, auto-despawned on `OnExit`
//! via `DespawnOnExit(Screen::MainMenu)`.

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_ui::screen::Screen;
use cdda_ui::screen_nav::{screen_def, FocusedCommandIndex};

/// Marks a command button, storing its index into the screen_def command list.
#[derive(Component)]
pub struct CommandButton(usize);

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

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

pub fn spawn(mut commands: Commands, focused: Res<FocusedCommandIndex>) {
    let def = screen_def(Screen::MainMenu);

    commands
        .spawn((
            DespawnOnExit(Screen::MainMenu),
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
            // Title
            parent.spawn((
                Text::new(def.title),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(ACCENT),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(48.0)),
                    ..default()
                },
            ));

            // Command buttons
            for (i, cmd) in def.commands.iter().enumerate() {
                let display = match cmd.hotkey {
                    Some(ch) => format!("{}) {}", ch, cmd.label),
                    None => format!("   {}", cmd.label),
                };
                let is_focused = i == focused.current();

                parent
                    .spawn((
                        CommandButton(i),
                        Button,
                        Node {
                            width: Val::Percent(60.0),
                            height: Val::Px(54.0),
                            display: Display::Flex,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(24.0)),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(if is_focused { ITEM_FOCUS_BG } else { ITEM_BG }),
                        BorderColor::all(if is_focused {
                            FOCUSED_BORDER
                        } else {
                            Color::NONE
                        }),
                    ))
                    .with_child((
                        Text::new(display),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(TEXT_BRIGHT),
                    ));
            }

            // Footer hint
            parent.spawn((
                Text::new("Arrow keys: navigate  |  Enter: select  |  Hotkey: quick-select"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(TEXT_DIM),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    margin: UiRect::top(Val::Px(64.0)),
                    ..default()
                },
            ));
        });
}

// ---------------------------------------------------------------------------
// sync_focus
// ---------------------------------------------------------------------------

pub fn sync_focus(
    focused: Res<FocusedCommandIndex>,
    mut input_focus: ResMut<InputFocus>,
    mut buttons: Query<(
        Entity,
        &CommandButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let current = focused.current();
    for (entity, btn, mut bg, mut border) in &mut buttons {
        if btn.0 == current {
            bg.0 = ITEM_FOCUS_BG;
            border.top = FOCUSED_BORDER;
            border.right = FOCUSED_BORDER;
            border.bottom = FOCUSED_BORDER;
            border.left = FOCUSED_BORDER;
            input_focus.0 = Some(entity);
        } else {
            bg.0 = ITEM_BG;
            border.top = Color::NONE;
            border.right = Color::NONE;
            border.bottom = Color::NONE;
            border.left = Color::NONE;
        }
    }
}
