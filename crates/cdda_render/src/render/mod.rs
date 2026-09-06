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
pub mod cinematic;
pub mod crafting;
pub mod crafting_state;
pub mod dev_spawn;
pub mod dev_worldgen;
pub mod examine;
pub mod input;
pub mod inventory;
pub mod item_detail;
pub mod loading;
pub mod main_menu;
pub mod overmap;
pub mod registry;
pub mod scroll;
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
    ctx_actions: Res<cdda_context::state::ContextActions>,
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
        if **text != hints {
            **text = hints;
        }
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

/// Resolve shared presentation after deferred screen updates, before Bevy measures text.
/// Kept separate from screen/game setup so headless fixtures use the same ordering.
pub struct UiPresentationPlugin;

impl Plugin for UiPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiFontHandle>()
            .init_resource::<theme::UiTheme>()
            .add_systems(
                PostUpdate,
                (apply_ui_font, theme::apply_palette).before(bevy::ui::UiSystems::Content),
            );
    }
}

/// Plugin that registers all CDDA render systems and components.
pub struct CddaRenderPlugin;

impl Plugin for CddaRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<cdda_components::progress::OperationReport>();
        app.add_message::<cdda_components::progress::OperationCommand>();
        app.add_systems(
            OnEnter(cdda_sim::runtime::state::AppState::DataLoading),
            (loading::cleanup, loading::spawn).chain(),
        );
        app.add_systems(
            OnEnter(cdda_sim::runtime::state::AppState::MainMenu),
            loading::cleanup,
        );
        app.add_systems(
            OnEnter(cdda_sim::runtime::state::AppState::InGame),
            loading::cleanup,
        );
        app.add_systems(
            Update,
            (loading::update, loading::animate_progress)
                .chain()
                .run_if(loading::is_loading),
        );
        app.add_systems(
            Update,
            (
                cinematic::layout.after(settings::apply_display_options),
                cinematic::reveal_art,
                cinematic::animate_accents.after(main_menu::sync_focus),
            ),
        );
        app.add_systems(PreUpdate, loading::input.run_if(loading::is_loading));
        app.add_systems(Startup, loading::spawn_notice.after(render_setup));
        app.add_systems(Update, loading::update_notice);
        app.init_resource::<settings::SettingsState>();
        app.add_plugins(UiPresentationPlugin);
        app.init_resource::<character::CharacterSheetState>();
        app.init_resource::<dev_spawn::DevSpawnFocus>();
        app.init_resource::<dev_spawn::DevSpawnCatalog>();
        app.init_resource::<inventory::InventoryFocus>();
        app.init_resource::<crafting_state::CraftState>();
        app.init_resource::<crafting_state::CraftModel>();
        app.init_resource::<crafting_state::CategoryIndex>();
        app.add_systems(
            OnEnter(Screen::CraftingMenu),
            crafting_state::build_craft_state,
        );
        app.add_systems(
            Update,
            crafting_state::refresh_craft_state
                .run_if(in_state(Screen::CraftingMenu))
                .run_if(crafting_state::craft_model_changed)
                .in_set(GameSet::Render)
                .before(crafting::update_crafting_ui),
        );

        app.add_systems(Startup, (render_setup, tiles::load_tiles));

        // Shared footer hint refresh — updates ALL FooterHint texts
        // across all screens. No per-screen footer systems needed.
        app.add_systems(Update, refresh_all_footer_hints);

        // Shared scroll primitives: arrow-key + wheel + focus-keep scrolling for
        // any node tagged `scroll::KeyboardScroll`. Global because any pane may
        // opt in; inactive panes just have no scrollable nodes.
        app.add_systems(
            PreUpdate,
            (scroll::scroll_with_wheel, scroll::scroll_with_keyboard),
        );
        app.add_systems(
            PostUpdate,
            (
                scroll::scroll_to_focused_row,
                scroll::update_virtual_windows,
            )
                .chain()
                .before(bevy::ui::UiSystems::Layout),
        );

        // ── Main menu ─────────────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::MainMenu), main_menu::spawn);
        for &screen in main_menu::COMMAND_MENUS {
            app.add_systems(OnEnter(screen), main_menu::spawn);
        }
        app.add_systems(Update, settings::apply_display_options);
        app.add_systems(
            Update,
            main_menu::sync_focus
                .after(settings::apply_display_options)
                .run_if(main_menu::is_command_menu),
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

        // Tab content rebuild + visual sync. Rebuild runs on
        // `Changed<SettingsTab>` / `Changed<SettingsState>`
        app.add_systems(
            Update,
            (
                settings::rebuild_content_panel,
                settings::sync_tab_highlight,
            )
                .chain()
                .after(settings::apply_display_options)
                .run_if(in_state(Screen::SettingsMenu)),
        );

        // ── Crafting menu spawn via CddaScreen; Update still here ────
        app.add_systems(
            Update,
            crafting::update_crafting_ui
                .after(input::crafting_menu_input)
                .run_if(in_state(Screen::CraftingMenu)),
        );
        app.add_systems(
            Update,
            input::crafting_menu_input.run_if(in_state(Screen::CraftingMenu)),
        );

        // Registry extraction is isolated from bounded input/presentation systems.
        app.init_resource::<registry::RegistryCatalog>();
        app.init_resource::<registry::RegistryViewerState>();
        app.add_systems(
            Update,
            (
                registry::refresh_registry_catalog.run_if(registry::registry_sources_changed),
                registry::registry_input,
                registry::update_registry_viewer,
            )
                .chain()
                .run_if(in_state(Screen::RegistryViewer)),
        );

        // ── Debug spawn panel ─────────────────────────────────────────────
        app.add_systems(
            OnEnter(Screen::DevSpawnPanel),
            (
                dev_spawn::dev_spawn_populate,
                dev_spawn::spawn_dev_spawn_panel,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            (
                dev_spawn::dev_spawn_populate.run_if(dev_spawn::dev_spawn_catalog_changed),
                input::dev_spawn_input,
                dev_spawn::update_dev_spawn_panel,
            )
                .chain()
                .run_if(in_state(Screen::DevSpawnPanel)),
        );

        // ── Inventory screen — spawn and update handled by CddaScreen trait ──

        // ── Character sheet spawn via CddaScreen; Update still here ───
        app.add_systems(
            Update,
            (
                character::character_sheet_input,
                character::update_character_sheet_screen,
            )
                .chain()
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
    let font_handle: Handle<Font> = asset_server.load("fonts/ShareTechMono-Regular.ttf");
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

/// Resolve new and restyled UI text before measurement, without idle writes.
/// Presenters may replace TextFont on retained rows, so Changed (not only Added)
/// must be handled. Running in Update lets deferred text render with a fallback
/// for one frame and can miss replacements made later in that same schedule.
pub fn apply_ui_font(handle: Res<UiFontHandle>, mut text_fonts: Query<&mut TextFont, With<Text>>) {
    let Some(font) = &handle.0 else {
        return;
    };
    for mut text_font in &mut text_fonts {
        if (handle.is_changed() || text_font.is_changed()) && text_font.font != *font {
            text_font.font = font.clone();
        }
    }
}
