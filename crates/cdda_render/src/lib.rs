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

use bevy::input_focus::{
    directional_navigation::DirectionalNavigationPlugin, InputDispatchPlugin, InputFocus,
    InputFocusVisible,
};
use bevy::prelude::*;
use bevy_state::state::OnEnter;
use cdda_screen::screen::Screen;

pub mod dev_spawn;
pub mod dev_worldgen;
pub mod inventory;
pub mod tiles;
pub mod main_menu;
pub mod settings;

/// Plugin that registers all CDDA render systems and components.
pub struct CddaRenderPlugin;

impl Plugin for CddaRenderPlugin {
    fn build(&self, app: &mut App) {
        // Bevy input focus + directional navigation
        app.add_plugins((InputDispatchPlugin, DirectionalNavigationPlugin));

        app.init_resource::<InputFocus>();
        app.insert_resource(InputFocusVisible(true));
        app.init_resource::<settings::SettingsState>();

        app.add_systems(Startup, (render_setup, tiles::load_tiles));

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

        // ── Debug spawn panel ─────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::DevSpawnPanel), dev_spawn::spawn_dev_spawn_panel);
        app.add_systems(
            Update,
            dev_spawn::update_dev_spawn_panel.run_if(in_state(Screen::DevSpawnPanel)),
        );

        // ── Inventory screen ──────────────────────────────────────────────
        app.add_systems(OnEnter(Screen::Inventory), inventory::spawn_inventory_screen);
        app.add_systems(
            Update,
            inventory::update_inventory_screen.run_if(in_state(Screen::Inventory)),
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
            (
                dev_worldgen::update_ascii_view,
                cdda_sim::systems::dev_move::dev_camera_move,
            )
                .run_if(in_state(Screen::Gameplay)),
        );
    }
}

fn render_setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 999.9),
    ));
}
