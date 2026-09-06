//! Illustrated command menus. Navigation remains owned by cdda_context.
use super::cinematic::{AccentMotion, ArtReveal, ScreenRegion};
use super::theme;
use bevy::prelude::*;
use bevy_state::{prelude::State, state_scoped::DespawnOnExit};
use cdda_context::{
    ctx::Ctx,
    nav::{ctx_def, FocusedCommandIndex},
    InputFocus,
};
use cdda_input::{ActionSource, ActiveKeybindings, GameAction, InputAction};

#[derive(Component)]
pub struct CommandButton(usize);
#[derive(Component)]
pub struct CommandPane;
#[derive(Component)]
pub struct MenuArtwork;
#[derive(Component)]
pub struct MenuTitle;
// Separate ownership from gameplay's ContextActions footer; no competing text writers.
#[derive(Component)]
pub struct MenuFooter;

pub fn spawn(
    mut commands: Commands,
    focused: Res<FocusedCommandIndex>,
    ui_font_handle: Res<super::UiFontHandle>,
    screen: Res<State<Ctx>>,
    assets: Res<AssetServer>,
    options: Res<super::settings::SettingsState>,
) {
    let ctx = *screen.get();
    let def = ctx_def(ctx);
    let font = &ui_font_handle.0;
    commands.spawn((DespawnOnExit(ctx), Node {
        width: percent(100), height: percent(100), overflow: Overflow::clip(), ..default()
    }, theme::SurfacePaint(theme::Role::Canvas))).with_children(|root| {
        root.spawn((MenuArtwork, ScreenRegion::MenuArt, ArtReveal::default(),
            ImageNode { color: Color::WHITE.with_alpha(0.), ..ImageNode::new(assets.load("gfx/loading_screens/loading_img_01.png")) },
            Node { display: if options.menu_art { Display::Flex } else { Display::None }, ..default() }));
        root.spawn((ScreenRegion::MenuContent, Node {
            flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center,
            overflow: Overflow::scroll_y(), ..default()
        }, ScrollPosition::default())).with_children(|parent| {
            parent.spawn((Text::new("D A R K   D A Y S   A H E A D"),
                super::ui_font(font, 13.), theme::TextPaint(theme::Role::Accent),
                Node { margin: UiRect::bottom(px(10)), flex_shrink: 0., ..default() }));
            parent.spawn((MenuTitle, Text::new(match ctx {
                Ctx::MainMenu => "CATACLYSM", Ctx::Custom(0) => "BULLETIN",
                Ctx::Custom(1) => "LOAD GAME", Ctx::PauseMenu => "PAUSED", _ => def.title,
            }), super::ui_font(font, 44.), theme::TextPaint(theme::Role::Text),
                Node { flex_shrink: 0., ..default() }));
            parent.spawn((Node { width: px(56), height: px(2),
                flex_shrink: 0., margin: UiRect::vertical(px(16)), ..default() }, theme::SurfacePaint(theme::Role::Accent)));
            parent.spawn((Text::new(menu_description(ctx)),
                super::ui_font(font, 16.), theme::TextPaint(theme::Role::Muted),
                Node { margin: UiRect::bottom(px(24)), flex_shrink: 0., ..default() }));
            parent.spawn((CommandPane, super::scroll::KeyboardScroll,
                super::scroll::FocusedRow(focused.current()), ScrollPosition::default(),
                super::scroll::VirtualList { row_height: 38., total_rows: def.commands.len(), ..default() },
                Node { width: percent(100), min_height: px(76), flex_shrink: 1.,
                    flex_direction: FlexDirection::Column, overflow: Overflow::scroll_y(), ..default() }
            )).with_children(|pane| {
                for (i, cmd) in def.commands.iter().enumerate() {
                    pane.spawn((CommandButton(i), Button,
                        AccentMotion::default(), Node {
                            width: percent(100), height: px(38), flex_shrink: 0., align_items: AlignItems::Center,
                            column_gap: px(16), padding: UiRect::horizontal(px(14)),
                            border: UiRect::left(px(2)), ..default()
                        }, BackgroundColor::default(), BorderColor::all(Color::NONE)))
                        .observe(move |mut click: On<Pointer<Click>>, mut focused: ResMut<FocusedCommandIndex>, mut writer: MessageWriter<InputAction>| {
                            focused.set(i);
                            writer.write(InputAction::new(GameAction::Confirm, ActionSource::Mouse));
                            click.propagate(false);
                        }).with_children(|button| {
                            button.spawn((Text::new(cmd.hotkey.map_or(" ".into(), |ch| ch.to_string())),
                                super::ui_font(font, 15.), theme::TextPaint(theme::Role::Accent),
                                Node { width: px(16), ..default() }));
                            button.spawn((Text::new(cmd.label), super::ui_font(font, 19.), theme::TextPaint(theme::Role::Text)));
                        });
                }
            });
            if ctx != Ctx::MainMenu {
                parent.spawn((Button, AccentMotion::default(), Node {
                    padding: UiRect::axes(px(14), px(6)), margin: UiRect::top(px(12)), flex_shrink: 0., ..default()
                })).observe(|mut click: On<Pointer<Click>>, mut writer: MessageWriter<InputAction>| {
                    writer.write(InputAction::new(GameAction::Cancel, ActionSource::Mouse)); click.propagate(false);
                }).with_child((Text::new("Back"), super::ui_font(font, 16.), theme::TextPaint(theme::Role::Text)));
            }
            parent.spawn((MenuFooter, Text::new(""), super::ui_font(font, 12.),
                theme::TextPaint(theme::Role::Muted), Node { margin: UiRect::top(px(24)), flex_shrink: 0., ..default() }));
        });
    });
}

pub fn sync_focus(
    theme: Res<theme::UiTheme>,
    focused: Res<FocusedCommandIndex>,
    mut input_focus: ResMut<InputFocus>,
    active_keys: Res<ActiveKeybindings>,
    options: Res<super::settings::SettingsState>,
    mut artwork: Query<&mut Node, With<MenuArtwork>>,
    mut footers: Query<&mut Text, With<MenuFooter>>,
    mut panes: Query<&mut super::scroll::FocusedRow, With<CommandPane>>,
    mut buttons: Query<(Entity, &CommandButton, &mut AccentMotion, &mut BorderColor)>,
) {
    if options.is_changed() {
        for mut node in &mut artwork {
            let display = if options.menu_art {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    for mut text in &mut footers {
        let nav = active_keys.key_for(cdda_input::BindableAction::NavigateUp);
        let select = active_keys.key_for(cdda_input::BindableAction::Confirm);
        text.set_if_neq(Text::new(format!(
            "[{nav}] navigate   [{select}] select\nLetter keys select directly"
        )));
    }
    let current = focused.current();
    for mut pane in &mut panes {
        if pane.0 != current {
            pane.0 = current;
        }
    }
    for (entity, button, mut motion, mut border) in &mut buttons {
        let selected = button.0 == current;
        if motion.selected != selected {
            motion.selected = selected;
        }
        border.set_if_neq(BorderColor::all(if selected {
            theme.color(theme::Role::Accent)
        } else {
            Color::NONE
        }));
        if selected && input_focus.entity != Some(entity) {
            input_focus.entity = Some(entity);
        }
    }
}

/// Shared frame for command menus; specialized gameplay panes keep their own presenters.
pub const COMMAND_MENUS: &[Ctx] = &[
    Ctx::DevWorldgen,
    Ctx::NewGameHub,
    Ctx::WorldMenu,
    Ctx::WorldSettings,
    Ctx::ScenarioSelect,
    Ctx::ProfessionSelect,
    Ctx::CharacterCreation,
    Ctx::CharacterConfirm,
    Ctx::HelpScreen,
    Ctx::CreditsScreen,
    Ctx::Custom(0),
    Ctx::Custom(1),
    Ctx::PauseMenu,
];
pub fn is_command_menu(screen: Res<State<Ctx>>) -> bool {
    *screen.get() == Ctx::MainMenu || COMMAND_MENUS.contains(screen.get())
}
fn menu_description(ctx: Ctx) -> &'static str {
    match ctx {
        Ctx::MainMenu => "The world ended. Your story didn't.",
        Ctx::DevWorldgen => "Explore the generated development world.",
        Ctx::NewGameHub => "Begin a new journey. Start Game loads your configured content and generates the world.",
        Ctx::HelpScreen => "Use arrow keys to navigate, Enter to select, and Escape to return. Tab changes panels. Item lists support mouse-wheel scrolling. Open Settings to inspect or change keybindings.",
        Ctx::CreditsScreen => "Cataclysm: Dark Days Ahead
Created by its community of contributors.

BR reimplementation in Rust and Bevy.
Original artwork from the bundled gfx/loading_screens collection. See LICENSE for project licensing.",
        Ctx::Custom(0) => "Welcome, survivor.

This port is under active development. Loading diagnostics identify content that could not be converted. Gameplay compatibility work is ongoing.",
        Ctx::Custom(1) => "Saved-world loading is not implemented yet. Return to the menu to start a new session.",
        Ctx::WorldMenu => "World generation currently uses the configured development world. Persistent world selection is not implemented yet.",
        Ctx::WorldSettings => "World options and mod selection are not editable here yet. The current build uses the default world configuration.",
        Ctx::ScenarioSelect | Ctx::ProfessionSelect | Ctx::CharacterCreation => "Character setup is not implemented yet. The current session uses the configured default survivor. Return to New Game and choose Start Game.",
        Ctx::CharacterConfirm => "Start a session with the configured default survivor.",
        Ctx::PauseMenu => "PAUSED
Press Escape to return to your game.",
        _ => "",
    }
}
