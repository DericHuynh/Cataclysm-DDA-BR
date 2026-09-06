mod support;

use bevy::prelude::*;
use bevy_state::prelude::State;
use cdda_context::substate::SettingsTab;
use cdda_render::render::{settings, theme, UiFontHandle};
use support::{add_text_pipeline, assert_shaped};

#[test]
fn deferred_text_and_restyled_retained_rows_use_shared_font_in_the_first_frame() {
    let mut app = App::new();
    add_text_pipeline(&mut app);
    // A late Update presenter reproduces Commands-based tab/detail replacement.
    app.add_systems(Update, |mut commands: Commands, mut once: Local<bool>| {
        if !*once {
            commands.spawn((
                Text::new("Fresh recipe details"),
                theme::TextPaint(theme::Role::Accent),
            ));
            *once = true;
        }
    });
    app.update();
    assert_shaped(app.world_mut());
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Text>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<TextColor>(entity).unwrap().0,
        app.world()
            .resource::<theme::UiTheme>()
            .color(theme::Role::Accent)
    );

    // Row reconciliation replaces the style while retaining the Text entity.
    app.add_systems(
        Update,
        move |mut commands: Commands, mut once: Local<bool>| {
            if !*once {
                commands.entity(entity).insert(TextFont {
                    font_size: 31.0,
                    ..default()
                });
                *once = true;
            }
        },
    );
    app.update();
    assert_shaped(app.world_mut());
    assert_eq!(app.world().get::<TextFont>(entity).unwrap().font_size, 31.0);
    let font_tick = app
        .world()
        .entity(entity)
        .get_ref::<TextFont>()
        .unwrap()
        .last_changed();
    let layout_tick = app
        .world()
        .entity(entity)
        .get_ref::<bevy::text::TextLayoutInfo>()
        .unwrap()
        .last_changed();
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        app.world()
            .entity(entity)
            .get_ref::<TextFont>()
            .unwrap()
            .last_changed(),
        font_tick
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get_ref::<bevy::text::TextLayoutInfo>()
            .unwrap()
            .last_changed(),
        layout_tick
    );
}

#[test]
fn every_settings_tab_shapes_immediately_and_preserves_static_heading() {
    let mut app = App::new();
    add_text_pipeline(&mut app);
    app.insert_resource(State::new(SettingsTab::Interface))
        .insert_resource(cdda_input::bindings::default_bindings())
        .init_resource::<settings::SettingsState>()
        .add_systems(Startup, settings::spawn)
        .add_systems(
            Update,
            (
                settings::rebuild_content_panel,
                settings::sync_tab_highlight,
            )
                .chain(),
        );
    app.update();
    assert_shaped(app.world_mut());
    let title = cdda_context::nav::ctx_def(cdda_context::Ctx::SettingsMenu).title;
    let heading = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find(|(_, t)| t.0 == title)
        .unwrap()
        .0;
    let initial_size = app.world().get::<ComputedNode>(heading).unwrap().size();
    for scale in [0.7, 1.0, 1.5] {
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        for tab in [
            SettingsTab::General,
            SettingsTab::Graphics,
            SettingsTab::Sound,
            SettingsTab::Keybindings,
            SettingsTab::Interface,
        ] {
            app.insert_resource(State::new(tab));
            app.update();
            assert_shaped(app.world_mut());
            assert_eq!(app.world().get::<Text>(heading).unwrap().0, title);
            assert!(
                app.world().get::<ComputedNode>(heading).unwrap().size().x >= initial_size.x * 0.65
            );
        }
    }
}

#[test]
fn late_font_initialization_updates_existing_text_before_measurement() {
    let mut app = App::new();
    add_text_pipeline(&mut app);
    let font = app.world_mut().resource_mut::<UiFontHandle>().0.take();
    app.world_mut().spawn(Text::new("Existing text"));
    app.update();
    app.world_mut().resource_mut::<UiFontHandle>().0 = font;
    app.update();
    assert_shaped(app.world_mut());
}
