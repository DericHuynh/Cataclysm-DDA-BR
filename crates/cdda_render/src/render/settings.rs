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

use crate::render::scroll::{sync_virtual_pane, FocusedRow, KeyboardScroll, VirtualList};
use crate::render::theme::{self, ThemePreset, UiTheme};
use bevy::prelude::*;
use bevy_state::prelude::{DespawnOnExit, State as BevyState};
use bevy_state::state::NextState;
use cdda_context::ctx::Ctx;
use cdda_context::substate::SettingsTab;
use cdda_input::bindings::ContextInputMaps;
use cdda_input::{
    BindableAction, GameAction, InputAction, InputContextId, RebindCapture, RebindCaptureInner,
};
use cdda_ui::{RetainedRows, RowCell, TextRow};

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
    pub ui_scale_percent: u16,
    pub fullscreen: bool,
    pub menu_art: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            focused_row: 0,
            ui_scale_percent: 100,
            fullscreen: false,
            menu_art: true,
            rebinding_action: None,
            interface_theme: ThemePreset::ALL
                .iter()
                .position(|preset| *preset == UiTheme::default().preset)
                .unwrap_or(0),
        }
    }
}

/// Marker components.
#[derive(Component)]
pub struct TabButton(pub SettingsTab);
#[derive(Component)]
pub struct ContentPanel;
#[derive(Component)]
pub struct SettingsItem;

const COLOR_SCHEME_ROW: usize = 0;

// Shared theme roles.

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
            theme::SurfacePaint(theme::Role::Canvas),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(def.title),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Accent),
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
                            BackgroundColor::default(),
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
                            theme::TextPaint(theme::Role::Text),
                        ));
                    }
                });

            content_entity = parent
                .spawn((
                    ContentPanel,
                    RetainedRows::<usize>::default(),
                    KeyboardScroll,
                    FocusedRow::default(),
                    VirtualList {
                        row_height: 48.0,
                        ..default()
                    },
                    ScrollPosition::default(),
                    Node {
                        width: Val::Percent(90.0),
                        max_width: px(960),
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    theme::SurfacePaint(theme::Role::Surface),
                    theme::BorderPaint(theme::Role::Border),
                ))
                .id();
            parent
                .spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(px(18), px(9)),
                        margin: UiRect::top(px(12)),
                        ..default()
                    },
                    super::cinematic::AccentMotion::default(),
                ))
                .observe(
                    |mut click: On<Pointer<Click>>, mut writer: MessageWriter<InputAction>| {
                        writer.write(InputAction::new(
                            GameAction::Cancel,
                            cdda_input::ActionSource::Mouse,
                        ));
                        click.propagate(false);
                    },
                )
                .with_child((
                    Text::new("Back"),
                    TextFont {
                        font_size: 18.,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Text),
                ));
        });

    let _ = content_entity;
}

// ---------------------------------------------------------------------------
// Row helper
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct SettingsPresentation {
    panel: Option<Entity>,
    rows: Vec<(String, bool)>,
    options: Option<(
        usize,
        Option<(InputContextId, BindableAction)>,
        u16,
        bool,
        bool,
    )>,
}

/// Retain tab contents and virtualize the keybinding editor like other lists.
pub fn rebuild_content_panel(
    mut commands: Commands,
    tab_state: Res<BevyState<SettingsTab>>,
    state: Res<SettingsState>,
    bindings: Res<ContextInputMaps>,
    theme: Res<UiTheme>,
    ui_font_handle: Res<super::UiFontHandle>,
    mut panel: Query<
        (
            Entity,
            &mut VirtualList,
            &mut FocusedRow,
            &mut ScrollPosition,
            &ComputedNode,
            &mut RetainedRows<usize>,
        ),
        With<ContentPanel>,
    >,
    mut cache: Local<SettingsPresentation>,
) {
    let Ok((entity, mut list, mut focus, mut position, computed, mut retained)) =
        panel.single_mut()
    else {
        return;
    };
    let reset = cache.panel != Some(entity) || tab_state.is_changed();
    let options = (
        state.interface_theme,
        state.rebinding_action.clone(),
        state.ui_scale_percent,
        state.fullscreen,
        state.menu_art,
    );
    let model_changed = reset || bindings.is_changed() || cache.options.as_ref() != Some(&options);
    let dirty =
        model_changed || state.is_changed() || theme.is_changed() || ui_font_handle.is_changed();
    if !dirty && !list.is_changed() {
        return;
    }
    cache.panel = Some(entity);
    if model_changed {
        cache.options = Some(options);
        let preset = ThemePreset::ALL[state.interface_theme % ThemePreset::ALL.len()];
        cache.rows.clear();
        let labels: Vec<String> = match *tab_state.get() {
            SettingsTab::General => vec![
                "Auto-save: Not implemented".into(),
                "Auto-notes: Not implemented".into(),
                "Distance rules: Not configurable yet".into(),
            ],
            SettingsTab::Graphics => vec![
                format!("Interface scale: {}% (Left/Right)", state.ui_scale_percent),
                format!(
                    "Fullscreen: {} (Left/Right)",
                    if state.fullscreen { "Yes" } else { "No" }
                ),
                format!(
                    "Menu artwork: {} (Left/Right)",
                    if state.menu_art { "Yes" } else { "No" }
                ),
            ],
            SettingsTab::Sound => vec![
                "Music: Audio playback not implemented".into(),
                "Sound effects: Audio playback not implemented".into(),
            ],
            SettingsTab::Interface => vec![
                format!(
                    "Theme: {} (all views · Left/Right to cycle)",
                    preset.label()
                ),
                "Sidebar style: Not configurable yet".into(),
                "Compass: Not implemented".into(),
                "Minimap: Not implemented".into(),
                "Confirmation style: Not configurable yet".into(),
            ],
            SettingsTab::Keybindings => Vec::new(),
        };
        cache
            .rows
            .extend(labels.into_iter().map(|label| (label, false)));
        if *tab_state.get() == SettingsTab::Keybindings {
            for ctx in [
                InputContextId::Gameplay,
                InputContextId::Inventory,
                InputContextId::MainMenu,
                InputContextId::Settings,
            ] {
                for (action, key) in bindings.list_bindings(&ctx) {
                    let rebinding = state
                        .rebinding_action
                        .as_ref()
                        .is_some_and(|(c, a)| *c == ctx && *a == action);
                    let key = if rebinding {
                        "PRESS A KEY... (Esc=cancel)".to_string()
                    } else {
                        key
                    };
                    cache.rows.push((
                        format!("{} · {}  ->  {}", ctx_label(&ctx), action.label(), key),
                        rebinding,
                    ));
                }
            }
        }
    }
    sync_virtual_pane(
        &mut list,
        &mut focus,
        &mut position,
        computed,
        cache.rows.len(),
        state.focused_row,
        reset,
    );
    let rows = (list.window.0..list.window.1).map(|index| {
        let (label, rebinding) = &cache.rows[index];
        let focused = index == focus.0;
        (
            index,
            TextRow {
                node: Node {
                    padding: UiRect::horizontal(Val::Px(12.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..list.row_node()
                },
                background: if *rebinding {
                    theme::TEXT_YELLOW
                } else if focused {
                    theme.color(theme::Role::Selection)
                } else {
                    theme.color(theme::Role::Surface)
                },
                border: if focused {
                    theme.color(theme::Role::Accent)
                } else {
                    Color::NONE
                },
                cells: vec![RowCell {
                    text: label.clone(),
                    font: super::ui_font(&ui_font_handle.0, 18.0),
                    color: theme.color(theme::Role::Text),
                    grow: 0.0,
                }],
            },
        )
    });
    for entity in retained.sync(&mut commands, entity, &list, rows) {
        commands
            .entity(entity)
            .insert((SettingsItem, Button))
            .observe(
                |mut click: On<Pointer<Click>>,
                 keys: Query<&cdda_ui::RowKey<usize>>,
                 mut state: ResMut<SettingsState>,
                 mut writer: MessageWriter<InputAction>| {
                    if let Ok(key) = keys.get(click.entity) {
                        state.focused_row = key.0;
                        writer.write(InputAction::new(
                            GameAction::Confirm,
                            cdda_input::ActionSource::Mouse,
                        ));
                    }
                    click.propagate(false);
                },
            );
    }
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
                if *tab_state.get() == SettingsTab::Graphics {
                    adjust_graphics(&mut state, -1);
                    continue;
                }
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
                if *tab_state.get() == SettingsTab::Graphics {
                    adjust_graphics(&mut state, 1);
                    continue;
                }
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
            if *tab_state.get() == SettingsTab::Graphics {
                adjust_graphics(&mut state, 1);
            }
            if *tab_state.get() == SettingsTab::Interface && state.focused_row == COLOR_SCHEME_ROW {
                state.interface_theme = (state.interface_theme + 1) % ThemePreset::ALL.len();
            }
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
    theme: Res<UiTheme>,
    tab_state: Res<BevyState<SettingsTab>>,
    mut tabs: Query<(&TabButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (tab, mut bg, mut border) in &mut tabs {
        if tab.0 == *tab_state.get() {
            bg.set_if_neq(BackgroundColor(theme.color(theme::Role::Selection)));
            border.set_if_neq(BorderColor::all(theme.color(theme::Role::Accent)));
        } else {
            bg.set_if_neq(BackgroundColor(theme.color(theme::Role::Surface)));
            border.set_if_neq(BorderColor::all(Color::NONE));
        }
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

fn adjust_graphics(state: &mut SettingsState, direction: i32) {
    match state.focused_row {
        0 => {
            state.ui_scale_percent =
                (i32::from(state.ui_scale_percent) + direction * 10).clamp(70, 150) as u16
        }
        1 => state.fullscreen = !state.fullscreen,
        2 => state.menu_art = !state.menu_art,
        _ => {}
    }
}

/// Apply functional presentation controls independently of the Settings screen.
pub fn apply_display_options(
    state: Res<SettingsState>,
    mut scale: Option<ResMut<UiScale>>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut theme: ResMut<UiTheme>,
) {
    if !state.is_changed() {
        return;
    }
    let preset = ThemePreset::ALL[state.interface_theme % ThemePreset::ALL.len()];
    if theme.preset != preset {
        theme.preset = preset;
    }
    if let Some(ref mut scale) = scale {
        let value = f32::from(state.ui_scale_percent) / 100.;
        if scale.0 != value {
            scale.0 = value;
        }
    }
    for mut window in &mut windows {
        let mode = if state.fullscreen {
            bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
        } else {
            bevy::window::WindowMode::Windowed
        };
        if window.mode != mode {
            window.mode = mode;
        }
    }
}

/// Persisted display values, separate from transient selection/rebind state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DisplayPreferences {
    pub theme: usize,
    pub scale_percent: u16,
    pub fullscreen: bool,
    pub menu_art: bool,
}
impl Default for DisplayPreferences {
    fn default() -> Self {
        Self::from(&SettingsState::default())
    }
}
impl From<&SettingsState> for DisplayPreferences {
    fn from(state: &SettingsState) -> Self {
        Self {
            theme: state.interface_theme,
            scale_percent: state.ui_scale_percent,
            fullscreen: state.fullscreen,
            menu_art: state.menu_art,
        }
    }
}
impl DisplayPreferences {
    pub fn apply(self, state: &mut SettingsState) {
        state.interface_theme = self.theme % ThemePreset::ALL.len();
        state.ui_scale_percent = self.scale_percent.clamp(70, 150);
        state.fullscreen = self.fullscreen;
        state.menu_art = self.menu_art;
    }
}
