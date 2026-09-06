//! Production-schedule capacity/ownership and focused transaction regressions.
use bevy_app::App;
use bevy_ecs::prelude::*;
use cdda_components::{
    actor::{ActionPoints, Creature, HandCount, IsAlive},
    def::{DefStrId, ItemVolume, ItemWeight},
    events::{ItemMoveEvent, ItemMoveResult, MoveLocation},
    intent::*,
    item::*,
    sim::WorldPosition,
};
use cdda_core_types::{
    core::{
        coords::{WorldPos, ZLevel},
        units::{Length, Volume, Weight},
    },
    sim_id::SimId,
};
use cdda_sim::{
    inventory::{
        capacity::contents_load,
        systems::{assign_invlets_system, can_fit_in_container, merge_or_stack},
        transfer::{apply_inventory_action, InventoryAction, TransferError},
    },
    runtime::{step_simulation, SimulationPlugin},
};
fn pos(x: i32) -> WorldPosition {
    WorldPosition::new(WorldPos::new(x, 0, ZLevel::new(0)))
}
fn actor(w: &mut World) -> Entity {
    w.spawn((
        IsAlive,
        ActionPoints {
            current: 100,
            speed: 100,
        },
        HandCount(2),
        pos(0),
    ))
    .id()
}
fn pocket(w: &mut World, owner: Entity, volume: u64, weight: u64) -> Entity {
    w.spawn((
        IsPocket,
        MountedOn(owner),
        Pocket {
            max_volume: Volume(volume),
            max_weight: Weight(weight),
            max_item_length: Length(1000),
            min_item_volume: Volume(0),
            pocket_type: PocketType::Container,
        },
    ))
    .id()
}
fn put(w: &mut World, actor: Entity, item: Entity, container: Entity) -> Result<(), TransferError> {
    apply_inventory_action(w, actor, item, InventoryAction::Transfer { container })
}
fn app() -> App {
    let mut a = App::new();
    a.add_plugins(SimulationPlugin);
    a
}

#[test]
fn whole_stacks_and_existing_contents_count_towards_both_limits() {
    let mut w = World::new();
    let a = actor(&mut w);
    let p = pocket(&mut w, a, 100, 50);
    w.spawn((
        InsideContainer(p),
        ItemVolume(20),
        ItemWeight(10),
        StackCount::new(2).unwrap(),
    ));
    let item = w
        .spawn((
            pos(0),
            ItemVolume(20),
            ItemWeight(10),
            StackCount::new(4).unwrap(),
        ))
        .id();
    assert_eq!(put(&mut w, a, item, p), Err(TransferError::PocketFull));
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
    w.entity_mut(item).insert(StackCount::new(3).unwrap());
    assert!(put(&mut w, a, item, p).is_ok());
    assert_eq!(contents_load(&w, p).unwrap().volume_ml, 100);
    assert_eq!(contents_load(&w, p).unwrap().weight_g, 50);
    let overflow = w.spawn(ItemVolume(1)).id();
    assert!(!can_fit_in_container(&w, p, overflow));
}

#[test]
fn absent_volume_does_not_disable_weight_or_missing_destination_checks() {
    let mut w = World::new();
    let a = actor(&mut w);
    let p = pocket(&mut w, a, 100, 5);
    let heavy = w.spawn((pos(0), ItemWeight(6))).id();
    assert!(!can_fit_in_container(&w, p, heavy));
    assert_eq!(put(&mut w, a, heavy, p), Err(TransferError::TooHeavy));
    w.despawn(p);
    assert!(!can_fit_in_container(&w, p, heavy));
    assert!(put(&mut w, a, heavy, p).is_err());
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
}

#[test]
fn nested_projection_checks_ancestors_without_double_counting_reparented_items() {
    let mut w = World::new();
    let a = actor(&mut w);
    let outer = pocket(&mut w, a, 100, 100);
    let bag = w
        .spawn((InsideContainer(outer), ItemVolume(10), ItemWeight(10)))
        .id();
    let inner = pocket(&mut w, bag, 1000, 1000);
    let content = w
        .spawn((InsideContainer(outer), ItemVolume(80), ItemWeight(80)))
        .id();
    assert!(
        put(&mut w, a, content, inner).is_ok(),
        "same ancestor load must not be counted twice"
    );
    let added = w.spawn((pos(0), ItemVolume(20), ItemWeight(1))).id();
    w.get_mut::<ActionPoints>(a).unwrap().current = 100;
    assert_eq!(put(&mut w, a, added, inner), Err(TransferError::PocketFull));
    assert_eq!(contents_load(&w, outer).unwrap().volume_ml, 90);
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
    w.entity_mut(bag).insert(Rigid);
    assert!(put(&mut w, a, added, inner).is_ok());
    assert_eq!(contents_load(&w, outer).unwrap().volume_ml, 10);
    assert_eq!(contents_load(&w, outer).unwrap().weight_g, 91);
    let heavy = w.spawn((pos(0), ItemVolume(0), ItemWeight(10))).id();
    w.get_mut::<ActionPoints>(a).unwrap().current = 100;
    assert_eq!(put(&mut w, a, heavy, inner), Err(TransferError::TooHeavy));
}

#[test]
fn automatic_storage_tries_fitting_pockets_and_never_falls_back_around_capacity() {
    let mut w = World::new();
    let a = actor(&mut w);
    let small = pocket(&mut w, a, 1, 100);
    w.entity_mut(small).insert(SimId(1));
    let large = pocket(&mut w, a, 10, 100);
    w.entity_mut(large).insert(SimId(2));
    let item = w.spawn((pos(0), ItemVolume(10))).id();
    assert!(apply_inventory_action(&mut w, a, item, InventoryAction::Pickup).is_ok());
    assert_eq!(w.get::<InsideContainer>(item).unwrap().0, large);
    let other = w.spawn((pos(0), ItemVolume(2))).id();
    w.get_mut::<ActionPoints>(a).unwrap().current = 100;
    assert_eq!(
        apply_inventory_action(&mut w, a, other, InventoryAction::Pickup),
        Err(TransferError::PocketFull)
    );
    assert!(
        put(&mut w, a, other, a).is_err(),
        "explicit loose inventory cannot bypass modeled storage"
    );
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
}

#[test]
fn sealed_and_unsupported_specialized_pockets_reject_without_mutation() {
    let mut w = World::new();
    let a = actor(&mut w);
    let p = pocket(&mut w, a, 100, 100);
    let item = w.spawn((pos(0), ItemVolume(1))).id();
    w.entity_mut(p).insert(Sealed);
    assert_eq!(
        put(&mut w, a, item, p),
        Err(TransferError::RestrictedPocket)
    );
    w.entity_mut(p).remove::<Sealed>();
    w.get_mut::<Pocket>(p).unwrap().pocket_type = PocketType::Magazine;
    assert_eq!(
        put(&mut w, a, item, p),
        Err(TransferError::RestrictedPocket)
    );
    w.get_mut::<Pocket>(p).unwrap().pocket_type = PocketType::Container;
    w.entity_mut(item)
        .remove::<WorldPosition>()
        .insert(InsideContainer(p));
    w.entity_mut(p).insert(Sealed);
    assert_eq!(
        apply_inventory_action(&mut w, a, item, InventoryAction::Wield),
        Err(TransferError::RestrictedPocket)
    );
    assert_eq!(w.get::<InsideContainer>(item).unwrap().0, p);
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
}

#[test]
fn production_transfer_intents_compete_for_live_ground_container_capacity() {
    let mut app = app();
    let w = app.world_mut();
    let a = actor(w);
    let b = actor(w);
    w.entity_mut(a).insert(SimId(1));
    w.entity_mut(b).insert(SimId(2));
    let chest = w
        .spawn((
            pos(0),
            Container {
                capacity: Volume(10),
            },
        ))
        .id();
    let first = w.spawn((InsideContainer(a), ItemVolume(10))).id();
    let second = w.spawn((InsideContainer(b), ItemVolume(1))).id();
    w.entity_mut(a).insert(ActionIntent::Transfer {
        item: first,
        container: chest,
    });
    w.entity_mut(b).insert(ActionIntent::Transfer {
        item: second,
        container: chest,
    });
    assert!(step_simulation(w));
    assert_eq!(
        w.get::<ActionOutcome>(a).unwrap().state,
        ActionOutcomeState::Completed
    );
    assert_eq!(
        w.get::<ActionOutcome>(b).unwrap().state,
        ActionOutcomeState::Rejected
    );
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
    assert_eq!(w.get::<ActionPoints>(b).unwrap().current, 200);
    assert_eq!(w.get::<InsideContainer>(second).unwrap().0, b);
}

#[test]
fn production_legacy_messages_validate_count_source_capacity_and_charge_once() {
    let mut app = app();
    let w = app.world_mut();
    let a = actor(w);
    let p = pocket(w, a, 10, 100);
    let item = w
        .spawn((pos(0), ItemVolume(5), StackCount::new(2).unwrap()))
        .id();
    let request = ItemMoveEvent {
        item,
        from: MoveLocation::Ground(pos(0).get()),
        to: MoveLocation::Container(p),
        count: 1,
    };
    w.write_message(request.clone());
    w.write_message(ItemMoveEvent {
        count: 2,
        ..request.clone()
    });
    w.write_message(ItemMoveEvent {
        count: 2,
        ..request
    });
    assert!(step_simulation(w));
    let mut cursor = bevy_ecs::message::MessageCursor::<ItemMoveResult>::default();
    let results: Vec<_> = cursor
        .read(w.resource::<Messages<ItemMoveResult>>())
        .collect();
    assert_eq!(
        results.iter().map(|r| r.accepted).collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(results[0].reason.as_deref(), Some("InvalidCount"));
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
    assert_eq!(w.get::<InsideContainer>(item).unwrap().0, p);
    assert!(step_simulation(w));
    assert_eq!(
        cursor
            .read(w.resource::<Messages<ItemMoveResult>>())
            .count(),
        0,
        "message cursor must not replay commits"
    );
}

#[test]
fn floor_capacity_includes_entire_counted_stacks() {
    let mut w = World::new();
    let a = actor(&mut w);
    let item = w
        .spawn((
            InsideContainer(a),
            ItemVolume(FLOOR_CAP_ML / 2 + 1),
            StackCount::new(2).unwrap(),
        ))
        .id();
    assert_eq!(
        apply_inventory_action(&mut w, a, item, InventoryAction::Drop),
        Err(TransferError::FloorFull)
    );
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
}

#[test]
fn explicit_merge_cannot_teleport_delete_children_or_overflow_counts() {
    let mut w = World::new();
    let a = w
        .spawn((
            pos(0),
            DefStrId("fiber".into()),
            StackCount::new(1).unwrap(),
        ))
        .id();
    let b = w
        .spawn((
            pos(1),
            DefStrId("fiber".into()),
            StackCount::new(1).unwrap(),
        ))
        .id();
    assert!(!merge_or_stack(&mut w, a, b));
    w.entity_mut(b).insert(pos(0));
    let child = w.spawn(InsideContainer(b)).id();
    assert!(!merge_or_stack(&mut w, a, b));
    assert!(w.get_entity(child).is_ok());
    w.despawn(child);
    w.entity_mut(b).remove::<ContainerContents>();
    w.entity_mut(a).insert(StackCount::new(u32::MAX).unwrap());
    assert!(!merge_or_stack(&mut w, a, b));
    assert!(!merge_or_stack(&mut w, a, a));
    assert!(w.get_entity(b).is_ok());
    w.entity_mut(a).insert(StackCount::new(2).unwrap());
    assert!(merge_or_stack(&mut w, a, b));
    assert_eq!(w.get::<StackCount>(a).unwrap().get(), 3);
}

#[test]
fn letter_assignment_is_unique_and_never_merges_or_despawns_items() {
    let mut w = World::new();
    let a = actor(&mut w);
    w.entity_mut(a).insert(Creature {
        def_id: "test".into(),
        name: "test".into(),
        species: cdda_components::SpeciesId::from(0u32),
        symbol: '@',
    });
    let items: Vec<_> = (0..3)
        .map(|_| {
            w.spawn((
                InsideContainer(a),
                DefOrigin(1),
                StackCount::new(1).unwrap(),
            ))
            .id()
        })
        .collect();
    use bevy_ecs::system::RunSystemOnce;
    w.run_system_once(assign_invlets_system).unwrap();
    let letters: std::collections::HashSet<_> = items
        .iter()
        .map(|&e| w.get::<Invlet>(e).unwrap().0)
        .collect();
    assert_eq!(letters.len(), 3);
    for item in items {
        assert_eq!(w.get::<StackCount>(item).unwrap().get(), 1);
    }
}

#[test]
fn reachable_ground_container_can_be_emptied_but_other_actors_and_cycles_are_rejected() {
    let mut w = World::new();
    let a = actor(&mut w);
    let owned = pocket(&mut w, a, 100, 100);
    let chest = w
        .spawn((
            pos(1),
            Container {
                capacity: Volume(100),
            },
        ))
        .id();
    let item = w.spawn((InsideContainer(chest), ItemVolume(1))).id();
    assert!(put(&mut w, a, item, owned).is_ok());
    w.get_mut::<ActionPoints>(a).unwrap().current = 100;
    let b = actor(&mut w);
    let other = pocket(&mut w, b, 100, 100);
    assert_eq!(put(&mut w, a, item, other), Err(TransferError::NotOwned));
    let bag = w
        .spawn((
            InsideContainer(owned),
            Container {
                capacity: Volume(100),
            },
        ))
        .id();
    let inner = pocket(&mut w, bag, 100, 100);
    assert!(put(&mut w, a, bag, inner).is_err());
    assert_eq!(w.get::<InsideContainer>(bag).unwrap().0, owned);
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
}

#[test]
fn capacity_overflow_and_charge_merge_overflow_fail_closed() {
    let mut w = World::new();
    let a = actor(&mut w);
    let p = pocket(&mut w, a, u64::MAX, u64::MAX);
    w.spawn((
        InsideContainer(p),
        ItemVolume(u32::MAX),
        StackCount::new(u32::MAX).unwrap(),
    ));
    let extra = w
        .spawn((pos(0), ItemVolume(u32::MAX), StackCount::new(3).unwrap()))
        .id();
    assert!(put(&mut w, a, extra, p).is_err());
    assert!(w.get::<WorldPosition>(extra).is_some());
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
    let first = w
        .spawn((pos(0), DefStrId("battery".into()), CurrentCharges(i32::MAX)))
        .id();
    let second = w
        .spawn((pos(0), DefStrId("battery".into()), CurrentCharges(1)))
        .id();
    assert!(!merge_or_stack(&mut w, first, second));
    assert_eq!(w.get::<CurrentCharges>(first).unwrap().0, i32::MAX);
    assert!(w.get_entity(second).is_ok());
}

#[test]
fn unimplemented_fluid_and_charge_dimensions_are_not_treated_as_counted_solids() {
    use cdda_components::def::{CountMode, ItemCountMode, ItemPhase, Phase};
    let mut w = World::new();
    let a = actor(&mut w);
    let p = pocket(&mut w, a, 1000, 1000);
    let item = w
        .spawn((pos(0), ItemVolume(1), ItemPhase(Phase::Liquid)))
        .id();
    let loose = actor(&mut w);
    assert_eq!(
        put(&mut w, loose, item, loose),
        Err(TransferError::UnsupportedItem)
    );
    assert_eq!(put(&mut w, a, item, p), Err(TransferError::UnsupportedItem));
    w.entity_mut(item)
        .remove::<ItemPhase>()
        .insert(ItemCountMode(CountMode::Charges {
            default: 10,
            max: 100,
        }));
    assert_eq!(put(&mut w, a, item, p), Err(TransferError::UnsupportedItem));
    assert_eq!(w.get::<ActionPoints>(a).unwrap().current, 100);
}
