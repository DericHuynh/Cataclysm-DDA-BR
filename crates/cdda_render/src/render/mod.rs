//! # cdda_render — Bevy rendering plugin
//!
//! Everything visual: tiles, UI menus, ASCII mode.
//! Reads simulation state; never writes it.
//!
//! Uses standard Bevy UI (`Node`, `Button`, `Text`) for all UI.
//!
//! Screen transitions use `DespawnOnExit` to auto-cleanup old-state
//! entities atomically before the new state's `OnEnter` runs, preventing
//! overlay glitches.

use bevy::prelude::*;
use bevy_state::condition::in_state;
use bevy_state::state::OnEnter;
use cdda_components::schedule::GameSet;
use cdda_context::ctx::Ctx as Screen;

pub mod character;
pub mod crafting;
pub mod dev_spawn;
pub mod dev_worldgen;
pub mod examine;
pub mod inventory;
pub mod item_detail;
pub mod main_menu;
pub mod overmap;
pub mod registry;
pub mod settings;
pub mod theme;
pub mod tiles;

/// Marker component for footer hint text entities that should be
/// live-updated from ContextActions + ActiveKeybindings each frame
/// by the shared `refresh_all_footer_hints` system.
#[derive(Component)]
pub struct FooterHint;

// ---------------------------------------------------------------------------
// refresh_all_footer_hints — single shared system for all screens
// ---------------------------------------------------------------------------

/// Updates every `FooterHint` text entity from `ContextActions` +
/// `ActiveKeybindings`.  Registered once in `CddaRenderPlugin` — no
/// per-screen footer update systems needed.
pub fn refresh_all_footer_hints(
    ctx_actions: Res<cdda_components::context::ContextActions>,
    active_keys: Res<cdda_input::ActiveKeybindings>,
    mut footer_q: Query<&mut Text, With<FooterHint>>,
) {
    for mut text in &mut footer_q {
        let cancel_key = active_keys.key_for(cdda_input::BindableAction::Cancel);
        let mut hints = format!("[{}] close", cancel_key);
        for entry in &ctx_actions.actions {
            let key = active_keys.key_for(entry.action);
            hints.push_str(&format!("  [{}] {}", key, entry.label));
        }
        **text = hints;
    }
}

/// Handle to the UI font (ShareTechMono-Regular.ttf), loaded at startup.
/// Wraps `Option` because font loading happens in `Startup` but
/// `OnEnter(MainMenu)` fires earlier during state initialisation.
#[derive(Resource, Clone)]
pub struct UiFontHandle(pub Option<Handle<Font>>);

impl Default for UiFontHandle {
    fn default() -> Self {
        Self(None)
    }
}

/// Helper: build a `TextFont` using the shared UI font.
/// Falls back to default font if the handle isn't loaded yet.
pub fn ui_font(handle: &Option<Handle<Font>>, size: f32) -> TextFont {
    TextFont {
        font: handle.clone().unwrap_or_default(),
        font_size: size,
        ..default()
    }
}

/// Plugin that registers all CDDA render systems and components.
pub struct CddaRenderPlugin;

impl Plugin for CddaRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<settings::SettingsState>();
        app.init_resource::<theme::UiTheme>();
        app.init_resource::<character::CharacterSheetState>();
        app.init_resource::<dev_spawn::DevSpawnFocus>();
        app.init_resource::<UiFontHandle>();

        app.add_systems(Startup, (render_setup, tiles::load_tiles));

        // Shared footer hint refresh — updates ALL FooterHint texts
        // across all screens. No per-screen footer systems needed.
        app.add_systems(Update, refresh_all_footer_hints);

        // ── Main menu ─────────────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::MainMenu), main_menu::spawn);
        app.add_systems(
            Update,
            main_menu::sync_focus.run_if(in_state(Screen::MainMenu)),
        );

        // ── Settings menu ─────────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::SettingsMenu), settings::spawn);

        // Navigation & interaction
        app.add_systems(
            PreUpdate,
            (
                settings::navigate,
                settings::handle_confirm,
                settings::detect_rebind_complete,
            )
                .run_if(in_state(Screen::SettingsMenu)),
        );

        // Tab content rebuild + visual sync
        app.add_systems(
            Update,
            (
                settings::rebuild_content_panel,
                settings::sync_tab_highlight,
                settings::sync_item_highlight,
            )
                .chain()
                .run_if(in_state(Screen::SettingsMenu)),
        );

        // ── Crafting menu spawn via CddaScreen; Update still here ────
        app.add_systems(
            Update,
            crafting::update_crafting_ui.run_if(in_state(Screen::CraftingMenu)),
        );

        // ── Debug spawn panel ─────────────────────────────────────────────
        app.add_systems(
            OnEnter(Screen::DevSpawnPanel),
            dev_spawn::spawn_dev_spawn_panel,
        );
        app.add_systems(
            Update,
            dev_spawn::update_dev_spawn_panel.run_if(in_state(Screen::DevSpawnPanel)),
        );

        // ── Inventory screen — spawn and update handled by CddaScreen trait ──

        // ── Character sheet spawn via CddaScreen; Update still here ───
        app.add_systems(
            Update,
            (
                character::update_character_sheet_screen,
                character::character_sheet_input,
            )
                .run_if(in_state(Screen::CharacterSheet)),
        );

        // ── Item Examine overlay spawn via CddaScreen ──────────────────

        // ── Overmap viewer ────────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::Overmap), overmap::spawn_overmap_viewer);
        app.add_systems(
            Update,
            overmap::overmap_camera_input
                .in_set(GameSet::Input)
                .run_if(in_state(Screen::Overmap)),
        );
        app.add_systems(
            Update,
            overmap::update_overmap_tiles
                .in_set(GameSet::Render)
                .run_if(in_state(Screen::Overmap)),
        );
        app.add_systems(
            Update,
            overmap::update_overmap_info_panel
                .in_set(GameSet::Render)
                .run_if(in_state(Screen::Overmap)),
        );

        // ── Dev worldgen ───────────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::DevWorldgen), dev_worldgen::spawn_dev_menu);
        app.add_systems(
            Update,
            dev_worldgen::sync_dev_menu_focus.run_if(in_state(Screen::DevWorldgen)),
        );
        app.add_systems(OnEnter(Screen::Gameplay), dev_worldgen::spawn_ascii_view);
        app.add_systems(
            Update,
            dev_worldgen::update_ascii_view
                .in_set(GameSet::Render)
                .run_if(in_state(Screen::Gameplay)),
        );
    }
}

fn render_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle: Handle<Font> = asset_server.load("fonts/Inter-VariableFont.ttf");
    commands.insert_resource(UiFontHandle(Some(font_handle)));

    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 999.9),
    ));
}
