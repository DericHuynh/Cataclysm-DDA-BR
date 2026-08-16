//! Main menu screen — rendered with standard bevy_ui.
//!
//! Spawned on `OnEnter(Ctx::MainMenu)`, auto-despawned on `OnExit`
//! via `DespawnOnExit(Ctx::MainMenu)`.

use super::FooterHint;
use crate::render::theme::{self, UiTheme};
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_context::ctx::Ctx;
use cdda_context::nav::{ctx_def, FocusedCommandIndex};
use cdda_context::InputFocus;
use cdda_input::{ActionSource, ActiveKeybindings, BindableAction, GameAction, InputAction};

/// Marks a command button, storing its index into the screen_def command list.
#[derive(Component)]
pub struct CommandButton(usize);

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

pub fn spawn(
    mut commands: Commands,
    focused: Res<FocusedCommandIndex>,
    active_keys: Res<ActiveKeybindings>,
    ui_font_handle: Res<super::UiFontHandle>,
    theme: Res<UiTheme>,
) {
    let def = ctx_def(Ctx::MainMenu);

    commands
        .spawn((
            DespawnOnExit(Ctx::MainMenu),
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
            // Title
            parent.spawn((
                Text::new(def.title),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(theme.accent2()),
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
                        BackgroundColor(if is_focused {
                            theme::BUTTON_FOCUS_BG
                        } else {
                            theme::BUTTON_BG
                        }),
                        BorderColor::all(if is_focused {
                            theme::TEXT_BRIGHT
                        } else {
                            Color::NONE
                        }),
                    ))
                    // Mouse: set the focused command to this button, then emit
                    // a `Confirm` `InputAction` so navigation reuses the single
                    // dispatch path in `handle_navigation_input` (keyboard does
                    // the same).
                    .observe(
                        move |mut click: On<Pointer<Click>>,
                              mut focused: ResMut<FocusedCommandIndex>,
                              mut writer: MessageWriter<InputAction>| {
                            focused.set(i);
                            writer
                                .write(InputAction::new(GameAction::Confirm, ActionSource::Mouse));
                            click.propagate(false);
                        },
                    )
                    .with_child((
                        Text::new(display),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(theme::TEXT_BRIGHT),
                    ));
            }

            // Footer hint
            let nav_key = active_keys.key_for(cdda_input::BindableAction::NavigateUp);
            let confirm_key = active_keys.key_for(cdda_input::BindableAction::Confirm);
            let hints = format!(
                "[{}] navigate  |  [{}] select  |  Hotkey: quick-select",
                nav_key, confirm_key
            );
            parent.spawn((
                Text::new(hints),
                super::ui_font(&ui_font_handle.0, 18.0),
                TextColor(theme::TEXT_DIM),
                FooterHint,
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
    active_keys: Res<ActiveKeybindings>,
    _ui_font_handle: Res<super::UiFontHandle>,
    _theme: Res<UiTheme>,
    mut footer_hint_q: Query<&mut Text, With<FooterHint>>,
    mut buttons: Query<(
        Entity,
        &CommandButton,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    // Live-update footer hints
    if let Ok(mut text) = footer_hint_q.single_mut() {
        let nav_key = active_keys.key_for(cdda_input::BindableAction::NavigateUp);
        let confirm_key = active_keys.key_for(cdda_input::BindableAction::Confirm);
        let hints = format!(
            "[{}] navigate  |  [{}] select  |  Hotkey: quick-select",
            nav_key, confirm_key
        );
        **text = hints;
    }
    let current = focused.current();
    for (entity, btn, mut bg, mut border) in &mut buttons {
        if btn.0 == current {
            bg.set_if_neq(BackgroundColor(theme::BUTTON_FOCUS_BG));
            let c = theme::TEXT_BRIGHT;
            border.top = c;
            border.right = c;
            border.bottom = c;
            border.left = c;
            input_focus.entity = Some(entity);
        } else {
            bg.set_if_neq(BackgroundColor(theme::BUTTON_BG));
            border.top = Color::NONE;
            border.right = Color::NONE;
            border.bottom = Color::NONE;
            border.left = Color::NONE;
        }
    }
}
