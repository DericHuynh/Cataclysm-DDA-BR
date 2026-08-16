//! Settings screen — tabbed categories with keybinding editor.
//!
//! The five Settings tabs are Bevy **`SubStates`** (`cdda_context::substate::SettingsTab`),
//! which only exist while the `Ctx::SettingsMenu` screen is active. The frame
//! (title + tab bar + content panel) is spawned on `OnEnter(Ctx::SettingsMenu)`.
//! The active tab is a `State<SettingsTab>`; switching it (via keyboard
//! `NavigateNextTab`/`NavigatePrevTab/Left/Right`, or a mouse click on a tab
//! button observed as `On<Pointer<Click>>`) routes through
//! `NextState<SettingsTab>`, and the active tab's rows are re-populated from it.
//! Rows are child entities of the persistent content panel, scoped per tab so
//! Bevy's `DespawnOnExit` is not needed for correctness here (we rebuild
//! explicitly on change).

use crate::render::scroll::{FocusedRow, KeyboardScroll};
use crate::render::theme::{ThemePreset, UiTheme};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;
use bevy_state::prelude::{in_state, DespawnOnExit, OnEnter, State as BevyState};
use bevy_state::state::NextState;
use cdda_context::ctx::Ctx;
use cdda_context::substate::SettingsTab;
use cdda_input::bindings::ContextInputMaps;
use cdda_input::{
    BindableAction, GameAction, InputAction, InputContextId, RebindCapture, RebindCaptureInner,
};

// ---------------------------------------------------------------------------
// Settings state
// ---------------------------------------------------------------------------

/// Per-tab UI state that does not map to a screen. The active tab lives in
/// `State<SettingsTab>` (Bevy `SubStates`), so this resource holds the focused
/// row (current tab), an in-progress rebind, and the interface theme index.
#[derive(Resource, Debug, Clone)]
pub struct SettingsState {
    pub focused_row: usize,
    pub rebinding_action: Option<(InputContextId, BindableAction)>,
    /// Index into `ThemePreset::ALL` for the Interface tab color scheme setting.
    pub interface_theme: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            focused_row: 0,
            rebinding_action: None,
            interface_theme: 0,
        }
    }
}

/// Marker components.
#[derive(Component)]
pub struct TabButton(pub SettingsTab);
#[derive(Component)]
pub struct ContentPanel;
#[derive(Component)]
pub struct SettingsItem(usize);

// Colours.
const BG: Color = Color::srgb(0.05, 0.05, 0.07);
const PANEL: Color = Color::srgb(0.08, 0.08, 0.10);
const TAB_ACTIVE: Color = Color::srgb(0.25, 0.55, 0.15);
const TAB_INACTIVE: Color = Color::srgb(0.08, 0.08, 0.10);
const ITEM_BG: Color = Color::srgb(0.08, 0.08, 0.10);
const ITEM_FOCUS_BG: Color = Color::srgb(0.25, 0.55, 0.15);
const ACCENT: Color = Color::srgb(0.85, 0.6, 0.15);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const HIGHLIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const REBIND_PROMPT: Color = Color::srgb(0.9, 0.7, 0.1);

// ---------------------------------------------------------------------------
// Spawn — frame (title + tab bar + content panel)
// ---------------------------------------------------------------------------

pub fn spawn(mut commands: Commands) {
    let def = cdda_context::nav::ctx_def(Ctx::SettingsMenu);

    let mut content_entity = Entity::PLACEHOLDER;

    commands
        .spawn((
            DespawnOnExit(Ctx::SettingsMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(def.title),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(ACCENT),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
            ));

            // Tab bar — each button selects a tab via `NextState<SettingsTab>`.
            parent
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(8.0),
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                })
                .with_children(|bar| {
                    for tab in SettingsTab::all() {
                        bar.spawn((
                            TabButton(*tab),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(TAB_INACTIVE),
                        ))
                        .observe(
                            move |mut click: On<Pointer<Click>>,
                                  mut next: ResMut<NextState<SettingsTab>>| {
                                next.set(*tab);
                                click.propagate(false);
                            },
                        )
                        .with_child((
                            Text::new(tab.label()),
                            TextFont {
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(TEXT_BRIGHT),
                        ));
                    }
                });

            content_entity = parent
                .spawn((
                    ContentPanel,
                    KeyboardScroll,
                    FocusedRow::default(),
                    ScrollPosition::default(),
                    Node {
                        width: Val::Percent(80.0),
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                ))
                .id();
        });

    let _ = content_entity;
}

// ---------------------------------------------------------------------------
// Row helper
// ---------------------------------------------------------------------------

fn row(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    index: usize,
    label: &str,
    state: &SettingsState,
) {
    let is_focused = index == state.focused_row;
    parent
        .spawn((
            SettingsItem(index),
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                margin: UiRect::vertical(Val::Px(2.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(if is_focused { ITEM_FOCUS_BG } else { ITEM_BG }),
            BorderColor::all(if is_focused { HIGHLIGHT } else { Color::NONE }),
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(TEXT_BRIGHT),
        ));
}

fn keybindings_rows(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    state: &SettingsState,
    bindings: &ContextInputMaps,
    ui_font_handle: &super::UiFontHandle,
) {
    let contexts = &[
        InputContextId::Gameplay,
        InputContextId::Inventory,
        InputContextId::MainMenu,
        InputContextId::Settings,
    ];

    let mut row_index = 0usize;

    for ctx in contexts {
        let pairs = bindings.list_bindings(ctx);
        if pairs.is_empty() {
            continue;
        }

        parent.spawn((
            Text::new(format!("——— {} ———", ctx_label(ctx))),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(ACCENT),
            Node {
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
        ));

        for (action, key_str) in &pairs {
            let is_focused = row_index == state.focused_row;
            let is_rebinding = state
                .rebinding_action
                .as_ref()
                .map(|(c, a)| c == ctx && a == action)
                .unwrap_or(false);

            let text = if is_rebinding {
                format!("{}  ⟶  PRESS A KEY... (Esc=cancel)", action.label())
            } else {
                format!("{}  ⟶  {}", action.label(), key_str)
            };

            parent
                .spawn((
                    SettingsItem(row_index),
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        margin: UiRect::vertical(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(if is_rebinding {
                        REBIND_PROMPT
                    } else if is_focused {
                        ITEM_FOCUS_BG
                    } else {
                        ITEM_BG
                    }),
                    BorderColor::all(if is_focused { HIGHLIGHT } else { Color::NONE }),
                ))
                .with_child((
                    Text::new(text),
                    super::ui_font(&ui_font_handle.0, 18.0),
                    TextColor(TEXT_BRIGHT),
                ));

            row_index += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Rebuild — populate the active tab's rows when the tab or its state changes.
// ---------------------------------------------------------------------------

/// (Re)build the active tab's rows into the `ContentPanel`. Runs on
/// `Changed<State<SettingsTab>>` or `Changed<SettingsState>`.
pub fn rebuild_content_panel(
    mut commands: Commands,
    tab_state: Res<BevyState<SettingsTab>>,
    state: Res<SettingsState>,
    bindings: Res<ContextInputMaps>,
    mut ui_theme: ResMut<UiTheme>,
    ui_font_handle: Res<super::UiFontHandle>,
    panel: Query<Entity, With<ContentPanel>>,
) {
    if !tab_state.is_changed() && !state.is_changed() {
        return;
    }

    // Sync the UiTheme from state.
    ui_theme.preset = ThemePreset::ALL[state.interface_theme % ThemePreset::ALL.len()];

    let Ok(panel_entity) = panel.single() else {
        return;
    };

    // Atomic clear, then repopulate.
    commands.entity(panel_entity).despawn_children();
    commands
        .entity(panel_entity)
        .with_children(|content| match *tab_state.get() {
            SettingsTab::General => {
                for (i, (l, v)) in [
                    ("Auto-save", "Enabled (every 5 min)"),
                    ("Auto-notes", "Yes"),
                    ("Circular distance", "No"),
                ]
                .iter()
                .enumerate()
                {
                    let label = format!("{l}: {v}");
                    row(content, i, &label, &state);
                }
            }
            SettingsTab::Graphics => {
                for (i, (l, v)) in [
                    ("Terminal size", "80×25"),
                    ("Font size", "16"),
                    ("Fullscreen", "No"),
                ]
                .iter()
                .enumerate()
                {
                    let label = format!("{l}: {v}");
                    row(content, i, &label, &state);
                }
            }
            SettingsTab::Sound => {
                for (i, (l, v)) in [("Music volume", "80%"), ("SFX volume", "100%")]
                    .iter()
                    .enumerate()
                {
                    let label = format!("{l}: {v}");
                    row(content, i, &label, &state);
                }
            }
            SettingsTab::Interface => {
                let items: Vec<(&str, String)> = vec![
                    ("Sidebar style", "classic".into()),
                    ("Show compass", "Yes".into()),
                    ("Minimap height", "100".into()),
                    ("Force capital Y/N", "Yes".into()),
                    (
                        "Color Scheme",
                        format!("{} (←/→ to cycle)", ui_theme.preset.label()),
                    ),
                ];
                for (i, (l, v)) in items.iter().enumerate() {
                    let label = format!("{l}: {v}");
                    row(content, i, &label, &state);
                }
            }
            SettingsTab::Keybindings => {
                keybindings_rows(content, &state, &bindings, &ui_font_handle);
            }
        });
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

pub fn navigate(
    mut action_reader: bevy::ecs::message::MessageReader<InputAction>,
    mut state: ResMut<SettingsState>,
    tab_state: Res<BevyState<SettingsTab>>,
    mut tab_next: ResMut<NextState<SettingsTab>>,
    bindings: Res<ContextInputMaps>,
) {
    const COLOR_SCHEME_ROW: usize = 4;

    for event in action_reader.read() {
        match &event.action {
            GameAction::NavigatePrevTab => {
                tab_next.set(tab_state.get().prev());
                state.focused_row = 0;
            }
            GameAction::NavigateNextTab => {
                tab_next.set(tab_state.get().next());
                state.focused_row = 0;
            }
            GameAction::NavigateLeft => {
                if *tab_state.get() == SettingsTab::Interface
                    && state.focused_row == COLOR_SCHEME_ROW
                {
                    let n = ThemePreset::ALL.len();
                    state.interface_theme = if state.interface_theme == 0 {
                        n - 1
                    } else {
                        state.interface_theme - 1
                    };
                } else {
                    tab_next.set(tab_state.get().prev());
                    state.focused_row = 0;
                }
            }
            GameAction::NavigateRight => {
                if *tab_state.get() == SettingsTab::Interface
                    && state.focused_row == COLOR_SCHEME_ROW
                {
                    state.interface_theme = (state.interface_theme + 1) % ThemePreset::ALL.len();
                } else {
                    tab_next.set(tab_state.get().next());
                    state.focused_row = 0;
                }
            }
            GameAction::NavigateUp => {
                let count = current_tab_row_count(tab_state.get(), &bindings);
                if count > 0 {
                    state.focused_row = state.focused_row.saturating_sub(1);
                }
            }
            GameAction::NavigateDown => {
                let count = current_tab_row_count(tab_state.get(), &bindings);
                if count > 0 {
                    state.focused_row = (state.focused_row + 1).min(count.saturating_sub(1));
                }
            }
            GameAction::NavigateHome => state.focused_row = 0,
            GameAction::NavigateEnd => {
                let count = current_tab_row_count(tab_state.get(), &bindings);
                if count > 0 {
                    state.focused_row = count.saturating_sub(1);
                }
            }
            _ => {}
        }
    }
}

pub fn handle_confirm(
    mut action_reader: bevy::ecs::message::MessageReader<InputAction>,
    mut state: ResMut<SettingsState>,
    tab_state: Res<BevyState<SettingsTab>>,
    bindings: Res<ContextInputMaps>,
    mut rebind_capture: ResMut<RebindCapture>,
) {
    for event in action_reader.read() {
        if event.action == GameAction::Confirm {
            if *tab_state.get() == SettingsTab::Keybindings {
                if let Some((ctx, action)) = find_binding_at_row(&state, &bindings) {
                    state.rebinding_action = Some((ctx, action.clone()));
                    rebind_capture.pending = Some(RebindCaptureInner {
                        context: ctx,
                        action: action.clone(),
                    });
                }
            }
        } else if event.action == GameAction::Cancel {
            if state.rebinding_action.is_some() {
                state.rebinding_action = None;
                rebind_capture.pending.take();
            }
        }
    }
}

pub fn detect_rebind_complete(mut state: ResMut<SettingsState>, rebind: Res<RebindCapture>) {
    if state.rebinding_action.is_some() && rebind.pending.is_none() {
        state.rebinding_action = None;
    }
}

// ---------------------------------------------------------------------------
// Visual sync
// ---------------------------------------------------------------------------

pub fn sync_tab_highlight(
    tab_state: Res<BevyState<SettingsTab>>,
    mut tabs: Query<(&TabButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (tab, mut bg, mut border) in &mut tabs {
        if tab.0 == *tab_state.get() {
            bg.set_if_neq(BackgroundColor(TAB_ACTIVE));
            *border = BorderColor::all(HIGHLIGHT);
        } else {
            bg.set_if_neq(BackgroundColor(TAB_INACTIVE));
            *border = BorderColor::all(Color::NONE);
        }
    }
}

pub fn sync_item_highlight(
    state: Res<SettingsState>,
    mut items: Query<(&SettingsItem, &mut BackgroundColor, &mut BorderColor)>,
    mut pane: Query<&mut FocusedRow, With<ContentPanel>>,
) {
    for (item, mut bg, mut border) in &mut items {
        if item.0 == state.focused_row {
            bg.set_if_neq(BackgroundColor(ITEM_FOCUS_BG));
            *border = BorderColor::all(HIGHLIGHT);
        } else {
            bg.set_if_neq(BackgroundColor(ITEM_BG));
            *border = BorderColor::all(Color::NONE);
        }
    }
    // Feed the shared keep-focused-visible scroll (scroll::scroll_to_focused_row).
    if let Ok(mut focus) = pane.single_mut() {
        focus.0 = state.focused_row;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ctx_label(ctx: &InputContextId) -> String {
    match ctx {
        InputContextId::Gameplay => "Gameplay".into(),
        InputContextId::Inventory => "Inventory".into(),
        InputContextId::CraftingMenu => "Crafting".into(),
        InputContextId::CharacterSheet => "Character".into(),
        InputContextId::ExamineLook => "Examine".into(),
        InputContextId::Dialog => "Dialog".into(),
        InputContextId::DirectionSelect => "Direction".into(),
        InputContextId::TextInput => "Text Input".into(),
        InputContextId::MainMenu => "Menus".into(),
        InputContextId::Settings => "Settings".into(),
        InputContextId::PauseMenu => "Pause".into(),
        InputContextId::QuantityInput => "Quantity".into(),
        InputContextId::VehicleInteraction => "Vehicle".into(),
        InputContextId::Custom(id) => format!("Custom({})", id),
        InputContextId::Overmap => "Overmap".to_string(),
    }
}

fn row_count(tab: &SettingsTab, bindings: &ContextInputMaps) -> usize {
    match tab {
        SettingsTab::General | SettingsTab::Graphics => 3,
        SettingsTab::Sound => 2,
        SettingsTab::Interface => 5,
        SettingsTab::Keybindings => {
            let contexts = &[
                InputContextId::Gameplay,
                InputContextId::Inventory,
                InputContextId::MainMenu,
                InputContextId::Settings,
            ];
            let mut count = 0;
            for ctx in contexts {
                let n = bindings.list_bindings(ctx).len();
                if n > 0 {
                    count += n;
                }
            }
            count
        }
    }
}

fn current_tab_row_count(tab: &SettingsTab, bindings: &ContextInputMaps) -> usize {
    row_count(tab, bindings)
}

fn find_binding_at_row(
    state: &SettingsState,
    bindings: &ContextInputMaps,
) -> Option<(InputContextId, BindableAction)> {
    let contexts = &[
        InputContextId::Gameplay,
        InputContextId::Inventory,
        InputContextId::MainMenu,
        InputContextId::Settings,
    ];
    let mut row = 0usize;
    for ctx in contexts {
        let pairs = bindings.list_bindings(ctx);
        if pairs.is_empty() {
            continue;
        }
        for (action, _) in &pairs {
            if row == state.focused_row {
                return Some((*ctx, *action));
            }
            row += 1;
        }
    }
    None
}
