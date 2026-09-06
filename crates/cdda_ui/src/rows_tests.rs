use super::*;
use bevy_ecs::world::CommandQueue;

fn value(key: usize, selected: bool, cells: usize) -> TextRow {
    TextRow {
        node: Node::default(),
        background: if selected { Color::WHITE } else { Color::BLACK },
        border: Color::NONE,
        cells: (0..cells)
            .map(|i| RowCell::new(format!("{key}:{i}"), 16.0, Color::WHITE))
            .collect(),
    }
}
fn sync(
    world: &mut World,
    pane: Entity,
    pool: &mut RetainedRows<usize>,
    keys: std::ops::Range<usize>,
    selected: usize,
    cells: usize,
) {
    world.increment_change_tick();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let mut list = VirtualList {
        row_height: 36.0,
        ..Default::default()
    };
    list.total_rows = 3000;
    list.window = (keys.start, keys.end);
    pool.sync(
        &mut commands,
        pane,
        &list,
        keys.map(|k| (k, value(k, k == selected, cells))),
    );
    queue.apply(world);
}
fn descendants(world: &World, pane: Entity) -> std::collections::HashSet<Entity> {
    let mut result = std::collections::HashSet::new();
    if let Some(children) = world.get::<Children>(pane) {
        for child in children.iter() {
            result.insert(child);
            result.extend(descendants(world, child));
        }
    }
    result
}

#[test]
fn focus_updates_only_background_and_idle_sync_writes_nothing() {
    let mut world = World::new();
    let pane = world.spawn_empty().id();
    let mut pool = RetainedRows::default();
    sync(&mut world, pane, &mut pool, 0..12, 0, 2);
    let entities = descendants(&world, pane);
    let row = pool.entity(&1).unwrap();
    let cell = world.get::<Children>(row).unwrap()[0];
    let text_tick = world.entity(cell).get_ref::<Text>().unwrap().last_changed();
    let bg_tick = world
        .entity(row)
        .get_ref::<BackgroundColor>()
        .unwrap()
        .last_changed();
    let hierarchy_tick = world
        .entity(pane)
        .get_ref::<Children>()
        .unwrap()
        .last_changed();
    sync(&mut world, pane, &mut pool, 0..12, 1, 2);
    assert_eq!(descendants(&world, pane), entities);
    assert_eq!(
        world.entity(cell).get_ref::<Text>().unwrap().last_changed(),
        text_tick
    );
    assert_eq!(
        world
            .entity(pane)
            .get_ref::<Children>()
            .unwrap()
            .last_changed(),
        hierarchy_tick
    );
    let changed_bg = world
        .entity(row)
        .get_ref::<BackgroundColor>()
        .unwrap()
        .last_changed();
    assert_ne!(changed_bg, bg_tick);
    sync(&mut world, pane, &mut pool, 0..12, 1, 2);
    assert_eq!(
        world
            .entity(row)
            .get_ref::<BackgroundColor>()
            .unwrap()
            .last_changed(),
        changed_bg
    );
    assert_eq!(
        world.entity(cell).get_ref::<Text>().unwrap().last_changed(),
        text_tick
    );
}

#[test]
fn scroll_reuses_pool_preserves_overlap_and_cleans_up_variable_cells_and_pane() {
    let mut world = World::new();
    let pane = world.spawn_empty().id();
    let mut pool = RetainedRows::default();
    sync(&mut world, pane, &mut pool, 0..12, 0, 2);
    let entities = descendants(&world, pane);
    let overlap = pool.entity(&6).unwrap();
    sync(&mut world, pane, &mut pool, 5..17, 6, 2);
    assert_eq!(pool.entity(&6), Some(overlap));
    assert_eq!(descendants(&world, pane), entities);
    sync(&mut world, pane, &mut pool, 2000..2012, 2000, 2);
    assert_eq!(
        descendants(&world, pane),
        entities,
        "far jumps allocate no row or text entities"
    );
    for key in 2000..2012 {
        let row = pool.entity(&key).unwrap();
        assert_eq!(world.get::<RowKey<usize>>(row).unwrap().0, key);
        let cell = world.get::<Children>(row).unwrap()[0];
        assert_eq!(world.get::<Text>(cell).unwrap().0, format!("{key}:0"));
    }
    sync(&mut world, pane, &mut pool, 2000..2002, 2000, 1);
    assert_eq!(
        world.entities().count_spawned(),
        7,
        "pane, spacers, two roots and two cells"
    );
    sync(&mut world, pane, &mut pool, 0..0, 0, 0);
    assert_eq!(
        world.entities().count_spawned(),
        3,
        "only pane and spacers survive"
    );
    sync(&mut world, pane, &mut pool, 0..12, 0, 3);
    assert_eq!(world.entities().count_spawned(), 51);
    world.despawn(pane);
    assert_eq!(
        world.entities().count_spawned(),
        0,
        "pane hierarchy owns all widget entities"
    );
}
