//! Settings screen — tabbed categories with keybinding editor.
//!
//! Uses `DespawnOnExit(Ctx::SettingsMenu)` for automatic cleanup.
//! Tab content is rebuilt via `rebuild_content_panel`.

use crate::context::config::GameSettings;
use crate::context::ctx::Ctx;
use crate::context::nav::ctx_def;
use crate::input::bindings::ContextInputMaps;
use crate::input::{BindableAction, InputContextId, RebindCapture, RebindCaptureInner};
use crate::render::theme::{ThemePreset, UiTheme};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_state::state_scoped::DespawnOnExit;

// ---------------------------------------------------------------------------
// Settings tab
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    General,
    Graphics,
    Sound,
    Interface,
    Keybindings,
}

impl SettingsTab {
    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::General,
            SettingsTab::Graphics,
            SettingsTab::Sound,
            SettingsTab::Interface,
            SettingsTab::Keybindings,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Graphics => "Graphics",
            SettingsTab::Sound => "Sound",
            SettingsTab::Interface => "Interface",
            SettingsTab::Keybindings => "Keybindings",
        }
    }
}

// ---------------------------------------------------------------------------
// Settings state
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone)]
pub struct SettingsState {
    pub active_tab: SettingsTab,
    pub focused_row: usize,
    pub rebinding_action: Option<(InputContextId, BindableAction)>,
    pub tab_changed: bool,
    /// Index into `ThemePreset::ALL` for the Interface tab color scheme setting.
    pub interface_theme: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::General,
            focused_row: 0,
            rebinding_action: None,
            tab_changed: false,
            interface_theme: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct TabButton(pub SettingsTab);

/// Attached to the panel that holds per-tab content rows.
#[derive(Component)]
pub struct ContentPanel;

/// Attached to each selectable row within a tab.
#[derive(Component)]
pub struct SettingsItem(usize);

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.05, 0.05, 0.07);
const PANEL: Color = Color::srgb(0.08, 0.08, 0.10);
const TAB_ACTIVE: Color = Color::srgb(0.25, 0.55, 0.15);
const TAB_INACTIVE: Color = Color::srgb(0.08, 0.08, 0.10);
const ITEM_BG: Color = Color::srgb(0.08, 0.08, 0.10);
const ITEM_FOCUS_BG: Color = Color::srgb(0.25, 0.55, 0.15);
const ACCENT: Color = Color::srgb(0.85, 0.6, 0.15);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.6, 0.6, 0.6);
const HIGHLIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const REBIND_PROMPT: Color = Color::srgb(0.9, 0.7, 0.1);

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

pub fn spawn(
    mut commands: Commands,
    __settings: Res<GameSettings>,
    state: Res<SettingsState>,
    bindings: Res<ContextInputMaps>,
    ui_theme: Res<UiTheme>,
    ui_font_handle: Res<super::UiFontHandle>,
) {
    let def = ctx_def(Ctx::SettingsMenu);

    let mut panel_entity = Entity::PLACEHOLDER;

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
                        let is_active = *tab == state.active_tab;
                        bar.spawn((
                            TabButton(*tab),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(if is_active { TAB_ACTIVE } else { TAB_INACTIVE }),
                            BorderColor::all(if is_active { HIGHLIGHT } else { Color::NONE }),
                        ))
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

            panel_entity = parent
                .spawn((
                    ContentPanel,
                    Node {
                        width: Val::Percent(80.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(Color::srgba(0.2, 0.2, 0.3, 0.5)),
                ))
                .id();
        });

    let theme_label = ui_theme.preset.label().to_string();
    populate_into(
        commands,
        panel_entity,
        &state,
        Some(&bindings),
        &theme_label,
        &ui_font_handle,
    );
}

fn populate_into(
    mut commands: Commands,
    panel: Entity,
    state: &SettingsState,
    bindings: Option<&ContextInputMaps>,
    theme_label: &str,
    ui_font_handle: &super::UiFontHandle,
) {
    // Wrap everything in a single entity so `despawn_children` on the panel
    // removes all content atomically.
    commands
        .entity(panel)
        .with_children(|content| match state.active_tab {
            SettingsTab::General => general_tab(content, state),
            SettingsTab::Graphics => graphics_tab(content, state),
            SettingsTab::Sound => sound_tab(content, state),
            SettingsTab::Interface => interface_tab(content, state, theme_label),
            SettingsTab::Keybindings => keybindings_tab(content, state, bindings, ui_font_handle),
        });
}

fn general_tab(parent: &mut RelatedSpawnerCommands<'_, ChildOf>, state: &SettingsState) {
    let items = [
        ("Auto-save", "Enabled (every 5 min)"),
        ("Auto-notes", "Yes"),
        ("Circular distance", "No"),
    ];
    for (i, (label, value)) in items.iter().enumerate() {
        row(parent, i, &format!("{label}: {value}"), state);
    }
}

fn graphics_tab(parent: &mut RelatedSpawnerCommands<'_, ChildOf>, state: &SettingsState) {
    let items = [
        ("Terminal size", "80×25"),
        ("Font size", "16"),
        ("Fullscreen", "No"),
    ];
    for (i, (label, value)) in items.iter().enumerate() {
        row(parent, i, &format!("{label}: {value}"), state);
    }
}

fn sound_tab(parent: &mut RelatedSpawnerCommands<'_, ChildOf>, state: &SettingsState) {
    let items = [("Music volume", "80%"), ("SFX volume", "100%")];
    for (i, (label, value)) in items.iter().enumerate() {
        row(parent, i, &format!("{label}: {value}"), state);
    }
}

fn interface_tab(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    state: &SettingsState,
    theme_label: &str,
) {
    let items: Vec<(&str, String)> = vec![
        ("Sidebar style", "classic".to_string()),
        ("Show compass", "Yes".to_string()),
        ("Minimap height", "100".to_string()),
        ("Force capital Y/N", "Yes".to_string()),
        ("Color Scheme", format!("{} (←/→ to cycle)", theme_label)),
    ];
    for (i, (label, value)) in items.iter().enumerate() {
        row(parent, i, &format!("{label}: {value}"), state);
    }
}

fn keybindings_tab(
    parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
    state: &SettingsState,
    bindings: Option<&ContextInputMaps>,
    ui_font_handle: &super::UiFontHandle,
) {
    let Some(bindings) = bindings else {
        parent.spawn((
            Text::new("No bindings available."),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(TEXT_DIM),
        ));
        return;
    };

    let contexts = &[
        InputContextId::Gameplay,
        InputContextId::Inventory,
        InputContextId::MainMenu,
        InputContextId::Settings,
    ];

    let mut row_index = 0usize;

    for ctx in contexts {
        // list_bindings returns Vec<(BindableAction, String)> sorted by label
        let pairs = bindings.list_bindings(ctx);
        if pairs.is_empty() {
            continue;
        }

        // Section header
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

    if row_index == 0 {
        parent.spawn((
            Text::new("No keybindings loaded."),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(TEXT_DIM),
            Node {
                margin: UiRect::all(Val::Px(16.0)),
                ..default()
            },
        ));
    }
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

// ---------------------------------------------------------------------------
// Rebuild on tab switch
// ---------------------------------------------------------------------------

pub fn rebuild_content_panel(
    mut commands: Commands,
    mut state: ResMut<SettingsState>,
    __settings: Res<GameSettings>,
    bindings: Res<ContextInputMaps>,
    panel: Query<Entity, With<ContentPanel>>,
    mut ui_theme: ResMut<UiTheme>,
    ui_font_handle: Res<super::UiFontHandle>,
) {
    if !state.tab_changed {
        return;
    }
    state.tab_changed = false;

    let Ok(panel_entity) = panel.single() else {
        return;
    };

    // Sync the UiTheme from state
    let preset_idx = state.interface_theme % ThemePreset::ALL.len();
    ui_theme.preset = ThemePreset::ALL[preset_idx];

    // Atomic clear — removes all children of the panel in one command
    commands.entity(panel_entity).despawn_children();

    let snapshot = SettingsState {
        active_tab: state.active_tab,
        focused_row: state.focused_row,
        interface_theme: state.interface_theme,
        ..Default::default()
    };

    let theme_label = ui_theme.preset.label().to_string();
    populate_into(
        commands,
        panel_entity,
        &snapshot,
        Some(&bindings),
        &theme_label,
        &ui_font_handle,
    );
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

fn switch_tab(state: &mut SettingsState, dir: isize) {
    let tabs = SettingsTab::all();
    let idx = tabs
        .iter()
        .position(|t| *t == state.active_tab)
        .unwrap_or(0);
    let len = tabs.len() as isize;
    let new = ((idx as isize + dir).rem_euclid(len)) as usize;
    state.active_tab = tabs[new];
    state.focused_row = 0;
    state.tab_changed = true;
}

pub fn navigate(
    mut action_reader: bevy::ecs::message::MessageReader<crate::input::InputAction>,
    mut state: ResMut<SettingsState>,
    bindings: Res<ContextInputMaps>,
) {
    // Color Scheme row index in the Interface tab
    const COLOR_SCHEME_ROW: usize = 4;

    for event in action_reader.read() {
        match &event.action {
            crate::input::GameAction::NavigatePrevTab => {
                switch_tab(&mut state, -1);
            }
            crate::input::GameAction::NavigateNextTab => {
                switch_tab(&mut state, 1);
            }
            crate::input::GameAction::NavigateLeft => {
                // If on Interface tab and focused on Color Scheme, cycle theme left
                if state.active_tab == SettingsTab::Interface
                    && state.focused_row == COLOR_SCHEME_ROW
                {
                    let n = ThemePreset::ALL.len();
                    state.interface_theme = if state.interface_theme == 0 {
                        n - 1
                    } else {
                        state.interface_theme - 1
                    };
                    state.tab_changed = true;
                } else {
                    switch_tab(&mut state, -1);
                }
            }
            crate::input::GameAction::NavigateRight => {
                // If on Interface tab and focused on Color Scheme, cycle theme right
                if state.active_tab == SettingsTab::Interface
                    && state.focused_row == COLOR_SCHEME_ROW
                {
                    state.interface_theme = (state.interface_theme + 1) % ThemePreset::ALL.len();
                    state.tab_changed = true;
                } else {
                    switch_tab(&mut state, 1);
                }
            }
            crate::input::GameAction::NavigateUp => {
                let count = current_tab_row_count(&state, &bindings);
                if count > 0 {
                    state.focused_row = state.focused_row.saturating_sub(1);
                }
            }
            crate::input::GameAction::NavigateDown => {
                let count = current_tab_row_count(&state, &bindings);
                if count > 0 {
                    state.focused_row = (state.focused_row + 1).min(count.saturating_sub(1));
                }
            }
            crate::input::GameAction::NavigateHome => state.focused_row = 0,
            crate::input::GameAction::NavigateEnd => {
                let count = current_tab_row_count(&state, &bindings);
                if count > 0 {
                    state.focused_row = count.saturating_sub(1);
                }
            }
            _ => {}
        }
    }
}

pub fn handle_confirm(
    mut action_reader: bevy::ecs::message::MessageReader<crate::input::InputAction>,
    mut state: ResMut<SettingsState>,
    bindings: Res<ContextInputMaps>,
    mut rebind_capture: ResMut<RebindCapture>,
) {
    for event in action_reader.read() {
        if event.action == crate::input::GameAction::Confirm {
            if state.active_tab == SettingsTab::Keybindings {
                if let Some((ctx, action)) = find_binding_at_row(&state, &bindings) {
                    state.rebinding_action = Some((ctx, action.clone()));
                    rebind_capture.pending = Some(RebindCaptureInner {
                        context: ctx,
                        action: action.clone(),
                    });
                }
            }
        } else if event.action == crate::input::GameAction::Cancel {
            if state.rebinding_action.is_some() {
                state.rebinding_action = None;
                rebind_capture.pending.take();
            }
        }
    }
}

pub fn detect_rebind_complete(
    mut state: ResMut<SettingsState>,
    rebind_capture: Res<RebindCapture>,
) {
    if state.rebinding_action.is_some() && rebind_capture.pending.is_none() {
        // The rebind was completed (handle_raw_input captured a key and
        // cleared RebindCapture).  Clear the UI state so the row returns
        // to normal display.
        state.rebinding_action = None;
    }
}

// ---------------------------------------------------------------------------
// Visual sync
// ---------------------------------------------------------------------------

pub fn sync_tab_highlight(
    state: Res<SettingsState>,
    mut tabs: Query<(&TabButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (tab, mut bg, mut border) in &mut tabs {
        if tab.0 == state.active_tab {
            bg.0 = TAB_ACTIVE;
            *border = BorderColor::all(HIGHLIGHT);
        } else {
            bg.0 = TAB_INACTIVE;
            *border = BorderColor::all(Color::NONE);
        }
    }
}

pub fn sync_item_highlight(
    state: Res<SettingsState>,
    mut items: Query<(&SettingsItem, &mut BackgroundColor, &mut BorderColor)>,
    mut content_panel: Query<&mut ScrollPosition, With<ContentPanel>>,
) {
    const ROW_HEIGHT_PX: f32 = 40.0;
    for (item, mut bg, mut border) in &mut items {
        if item.0 == state.focused_row {
            bg.0 = ITEM_FOCUS_BG;
            *border = BorderColor::all(HIGHLIGHT);
        } else {
            bg.0 = ITEM_BG;
            *border = BorderColor::all(Color::NONE);
        }
    }
    if let Ok(mut scroll) = content_panel.single_mut() {
        scroll.y = state.focused_row as f32 * ROW_HEIGHT_PX;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn _yn(b: bool) -> String {
    if b {
        "Yes".into()
    } else {
        "No".into()
    }
}

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
    }
}

fn current_tab_row_count(state: &SettingsState, bindings: &ContextInputMaps) -> usize {
    match state.active_tab {
        SettingsTab::General => 3,
        SettingsTab::Graphics => 3,
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
                    count += 1 + n; // header + rows
                }
            }
            count
        }
    }
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
        // list_bindings returns Vec<(BindableAction, String)> sorted by label
        let pairs = bindings.list_bindings(ctx);
        if pairs.is_empty() {
            continue;
        }
        row += 1; // header
        for (action, _) in &pairs {
            if row == state.focused_row {
                return Some((*ctx, *action));
            }
            row += 1;
        }
    }
    None
}
