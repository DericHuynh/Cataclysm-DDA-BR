//! Tests for the headless nested-menu (`SubStates`) types.
//!
//! Verifies that `SettingsTab` (a Bevy `SubStates` scoped under
//! `Ctx::SettingsMenu`) exposes the expected ordering helpers and that a minimal
//! `App` can register it and transition tabs through `NextState<SettingsTab>`.

use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_state::app::{AppExtStates, StatesPlugin};
use bevy_state::state::NextState;
use cdda_context::ctx::Ctx;
use cdda_context::substate::SettingsTab;

#[test]
fn settings_tab_all_is_in_display_order() {
    assert_eq!(
        SettingsTab::all(),
        &[
            SettingsTab::General,
            SettingsTab::Graphics,
            SettingsTab::Sound,
            SettingsTab::Interface,
            SettingsTab::Keybindings,
        ]
    );
}

#[test]
fn settings_tab_next_wraps() {
    assert_eq!(SettingsTab::General.next(), SettingsTab::Graphics);
    assert_eq!(SettingsTab::Keybindings.next(), SettingsTab::General);
}

#[test]
fn settings_tab_prev_wraps() {
    assert_eq!(SettingsTab::General.prev(), SettingsTab::Keybindings);
    assert_eq!(SettingsTab::Keybindings.prev(), SettingsTab::Interface);
}

#[test]
fn settings_tab_registers_as_substate_under_settings_menu() {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Ctx>();
    app.add_sub_state::<SettingsTab>();

    // SubStates only exist while their source state is active.
    app.world_mut()
        .resource_mut::<NextState<Ctx>>()
        .set(Ctx::SettingsMenu);
    app.update();
    app.update();

    // Default tab is General.
    let tab = app
        .world()
        .resource::<bevy_state::state::State<SettingsTab>>();
    assert_eq!(*tab.get(), SettingsTab::General);
}

#[test]
fn settings_tab_switches_via_next_state() {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Ctx>();
    app.add_sub_state::<SettingsTab>();

    // Activate the settings screen so the substate exists.
    app.world_mut()
        .resource_mut::<NextState<Ctx>>()
        .set(Ctx::SettingsMenu);
    app.update();
    app.update();

    app.world_mut()
        .resource_mut::<NextState<SettingsTab>>()
        .set(SettingsTab::Keybindings);
    app.update();

    let tab = app
        .world()
        .resource::<bevy_state::state::State<SettingsTab>>();
    assert_eq!(*tab.get(), SettingsTab::Keybindings);
}
