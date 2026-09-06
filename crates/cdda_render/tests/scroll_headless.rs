use bevy::prelude::*;
use cdda_render::render::scroll::{
    scroll_to_focused_row, update_virtual_windows, FocusedRow, KeyboardScroll, VirtualList,
};

fn app(row: f32, total: usize, scale: f32) -> (App, Entity) {
    let mut app = App::new();
    app.add_systems(
        Update,
        (scroll_to_focused_row, update_virtual_windows).chain(),
    );
    let entity = app
        .world_mut()
        .spawn((
            KeyboardScroll,
            FocusedRow(0),
            ScrollPosition::default(),
            ComputedNode {
                size: Vec2::new(400.0, 260.0) * scale,
                inverse_scale_factor: 1.0 / scale,
                ..default()
            },
            VirtualList {
                row_height: row,
                total_rows: total,
                ..default()
            },
        ))
        .id();
    (app, entity)
}

#[test]
fn large_catalog_navigation_keeps_selection_visible_at_both_scales() {
    for scale in [1.0, 2.0] {
        for row in [26.0, 30.0, 35.0, 48.0] {
            let (mut app, entity) = app(row, 40_000, scale);
            for index in [0, 30, 2000, 39_999, 0] {
                app.world_mut().get_mut::<FocusedRow>(entity).unwrap().0 = index;
                app.update();
                let world = app.world();
                let offset = world.get::<ScrollPosition>(entity).unwrap().y;
                let list = world.get::<VirtualList>(entity).unwrap();
                assert!(offset <= index as f32 * row);
                assert!(offset + 260.0 >= (index + 1) as f32 * row);
                assert!(list.window.0 <= index && index < list.window.1);
                assert!(list.window.1 - list.window.0 <= (260.0 / row).ceil() as usize + 9);
            }
        }
    }
}

#[test]
fn manual_scroll_is_not_pulled_back_to_unchanged_selection() {
    let (mut app, entity) = app(26.0, 40_000, 1.0);
    app.update();
    app.world_mut().get_mut::<ScrollPosition>(entity).unwrap().y = 10_000.0;
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(entity).unwrap().y,
        10_000.0
    );
    app.world_mut().get_mut::<FocusedRow>(entity).unwrap().0 = 1;
    app.update();
    assert_eq!(app.world().get::<ScrollPosition>(entity).unwrap().y, 26.0);
}

#[test]
fn filtering_and_fractional_scroll_always_produce_bounded_windows() {
    let list = VirtualList {
        row_height: 26.0,
        total_rows: 3,
        ..default()
    };
    for offset in [-100.0, 0.0, 25.9, 1_000_000.0] {
        let (start, end) = list.visible_window(offset, 260.0);
        assert_eq!((start, end), (0, 3));
    }
    assert_eq!(VirtualList::default().visible_window(1000.0, 260.0), (0, 0));
}

#[test]
fn idle_frames_do_not_invalidate_virtual_list() {
    #[derive(Resource, Default)]
    struct Changes(usize);
    let (mut app, _) = app(26.0, 40_000, 1.0);
    app.init_resource::<Changes>().add_systems(
        Update,
        (|q: Query<Entity, Changed<VirtualList>>, mut count: ResMut<Changes>| {
            count.0 += q.iter().count();
        })
        .after(update_virtual_windows),
    );
    app.update();
    let initial = app.world().resource::<Changes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Changes>().0, initial);
}

#[test]
fn wheel_over_child_scrolls_ancestor_in_lines_and_pixels() {
    use bevy::input::mouse::MouseScrollUnit;
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, Scroll},
        pointer::{Location, PointerId},
    };
    use cdda_render::render::scroll::scroll_with_wheel;
    let mut app = App::new();
    app.add_message::<Pointer<Scroll>>()
        .add_systems(Update, scroll_with_wheel);
    let pane = app
        .world_mut()
        .spawn((
            KeyboardScroll,
            ScrollPosition::default(),
            ComputedNode {
                size: Vec2::new(300.0, 200.0),
                content_size: Vec2::new(300.0, 2000.0),
                ..default()
            },
        ))
        .id();
    let child = app.world_mut().spawn(ChildOf(pane)).id();
    for (unit, expected) in [
        (MouseScrollUnit::Line, 34.0),
        (MouseScrollUnit::Pixel, 35.0),
    ] {
        app.world_mut().write_message(Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(pane))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::ZERO,
            },
            Scroll {
                unit,
                x: 0.0,
                y: -1.0,
                hit: HitData::new(pane, 0.0, None, None),
            },
            child,
        ));
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(pane).unwrap().y, expected);
    }
}
