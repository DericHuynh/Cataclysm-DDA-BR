//! Intent commits are sequential transactions, not deferred promises.
use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::intent::{
    ActionIntent, ActionOutcome, ActionOutcomeState, ActionRequestCounter, ActionRequestId,
    IntentQueue, QueuedIntent,
};
use cdda_components::item::{
    ContainerContents, InsideContainer, IsPocket, MountedOn, WieldedBy, WornOn,
};
use cdda_components::sim::{Solid, WorldPosition};
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_core_types::sim_id::SimId;
use cdda_sim::actor::turn::{AP_COST_PICKUP, MOVE_COST_WALK};
use cdda_sim::intent::systems::{collect_intents, resolve_intents};
use cdda_sim::runtime::test_utils::TestBed;

fn bed() -> TestBed {
    let mut test = TestBed::new();
    test.insert_resource(IntentQueue::default());
    test.insert_resource(ActionRequestCounter::default());
    test
}

fn position(x: i32, y: i32, z: i8) -> WorldPosition {
    WorldPosition::new(WorldPos::new(x, y, ZLevel::new(z)))
}

fn actor(test: &mut TestBed, x: i32, ap: i32) -> Entity {
    test.spawn((
        IsAlive,
        ActionPoints {
            current: ap,
            speed: 100,
        },
        position(x, 0, 0),
    ))
}

fn declare(test: &mut TestBed, actor: Entity, intent: ActionIntent) {
    test.world_mut().entity_mut(actor).insert(intent);
}

fn resolve(test: &mut TestBed) {
    test.run_system(collect_intents);
    test.run_system(resolve_intents);
}

fn assert_outcome(test: &TestBed, actor: Entity, expected: ActionOutcomeState, ap: i32) {
    let outcome = test.get::<ActionOutcome>(actor).expect("terminal outcome");
    assert_eq!(outcome.state, expected);
    assert_eq!(
        outcome.request,
        *test.get::<ActionRequestId>(actor).unwrap()
    );
    assert_eq!(test.get::<ActionPoints>(actor).unwrap().current, ap);
}

fn enqueue(test: &mut TestBed, entity: Entity, request: u64, intent: ActionIntent) {
    let request = ActionRequestId(request);
    test.world_mut().entity_mut(entity).insert(request);
    let ap = test.get::<ActionPoints>(entity).unwrap().current;
    test.resource_mut::<IntentQueue>()
        .queued
        .push(QueuedIntent {
            request,
            entity,
            intent,
            ap,
        });
}

#[test]
fn contested_pickup_commits_once_and_rejects_the_loser_without_ap() {
    let mut test = bed();
    let loser = actor(&mut test, 0, 100);
    let winner = actor(&mut test, 0, 150);
    let item = test.spawn(position(1, 0, 0));
    declare(&mut test, loser, ActionIntent::Pickup { item });
    declare(&mut test, winner, ActionIntent::Pickup { item });

    resolve(&mut test);

    assert_outcome(
        &test,
        winner,
        ActionOutcomeState::Completed,
        150 - AP_COST_PICKUP,
    );
    assert_outcome(&test, loser, ActionOutcomeState::Rejected, 100);
    assert_eq!(test.get::<InsideContainer>(item).unwrap().0, winner);
    assert!(test.get::<WorldPosition>(item).is_none());
    assert_eq!(
        test.get::<ContainerContents>(winner)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![item]
    );
    assert!(test.get::<ContainerContents>(loser).is_none());
    assert_eq!(test.resource::<IntentQueue>().rejected, 1);
    assert!(test.resource::<IntentQueue>().queued.is_empty());
}

#[test]
fn absent_actor_position_rejects_move_and_pickup_without_ap() {
    for pickup in [false, true] {
        let mut test = bed();
        let actor = actor(&mut test, 0, 100);
        test.world_mut().entity_mut(actor).remove::<WorldPosition>();
        let item = test.spawn(position(0, 0, 0));
        let intent = if pickup {
            ActionIntent::Pickup { item }
        } else {
            ActionIntent::Move { dx: 1, dy: 0 }
        };
        declare(&mut test, actor, intent);
        resolve(&mut test);
        assert_outcome(&test, actor, ActionOutcomeState::Rejected, 100);
        assert!(test.get::<InsideContainer>(item).is_none());
        assert!(test.get::<WorldPosition>(actor).is_none());
    }
}

#[test]
fn move_rejects_zero_large_and_overflowing_steps() {
    for (x, y, dx, dy) in [
        (0, 0, 0, 0),
        (0, 0, 24, 0),
        (0, 0, 0, -2),
        (0, 0, i32::MIN, 0),
        (i32::MAX, 0, 1, 0),
        (0, i32::MIN, 0, -1),
    ] {
        let mut test = bed();
        let actor = actor(&mut test, x, 100);
        test.world_mut().entity_mut(actor).insert(position(x, y, 0));
        declare(&mut test, actor, ActionIntent::Move { dx, dy });
        resolve(&mut test);
        assert_outcome(&test, actor, ActionOutcomeState::Rejected, 100);
        assert_eq!(
            *test.get::<WorldPosition>(actor).unwrap(),
            position(x, y, 0)
        );
    }
}

#[test]
fn pickup_checks_actual_range_z_and_ground_position() {
    for item_position in [
        Some(position(2, 0, 0)),
        Some(position(0, -2, 0)),
        Some(position(0, 0, 1)),
        Some(position(i32::MIN, 0, 0)),
        None,
    ] {
        let mut test = bed();
        let actor = actor(&mut test, 0, 100);
        let item = test.spawn(());
        if let Some(pos) = item_position {
            test.world_mut().entity_mut(item).insert(pos);
        }
        declare(&mut test, actor, ActionIntent::Pickup { item });
        resolve(&mut test);
        assert_outcome(&test, actor, ActionOutcomeState::Rejected, 100);
        assert!(test.get::<InsideContainer>(item).is_none());
        assert_eq!(test.get::<WorldPosition>(item).copied(), item_position);
    }
}

#[test]
fn pickup_allows_same_tile_and_diagonal_adjacency() {
    for (x, y) in [(0, 0), (1, 1), (-1, -1)] {
        let mut test = bed();
        let actor = actor(&mut test, 0, 100);
        let item = test.spawn(position(x, y, 0));
        declare(&mut test, actor, ActionIntent::Pickup { item });
        resolve(&mut test);
        assert_outcome(
            &test,
            actor,
            ActionOutcomeState::Completed,
            100 - AP_COST_PICKUP,
        );
        assert_eq!(test.get::<InsideContainer>(item).unwrap().0, actor);
    }
}

#[test]
fn stale_ground_position_never_overrides_inventory_or_equipment_ownership() {
    for ownership in 0..5 {
        let mut test = bed();
        let actor = actor(&mut test, 0, 100);
        let owner = test.spawn(());
        let item = test.spawn(position(0, 0, 0));
        match ownership {
            0 => {
                test.world_mut()
                    .entity_mut(item)
                    .insert(InsideContainer(owner));
            }
            1 => {
                test.world_mut().entity_mut(item).insert(WieldedBy(owner));
            }
            2 => {
                test.world_mut().entity_mut(item).insert(WornOn {
                    wearer: owner,
                    slot: None,
                });
            }
            3 => {
                test.world_mut().entity_mut(item).insert(MountedOn(owner));
            }
            _ => {
                test.world_mut().entity_mut(item).insert(IsPocket);
            }
        }
        declare(&mut test, actor, ActionIntent::Pickup { item });
        resolve(&mut test);
        assert_outcome(&test, actor, ActionOutcomeState::Rejected, 100);
        assert!(test.get::<WorldPosition>(item).is_some());
        assert_eq!(
            test.get::<InsideContainer>(item).map(|p| p.0),
            (ownership == 0).then_some(owner)
        );
        match ownership {
            1 => assert_eq!(test.get::<WieldedBy>(item).unwrap().0, owner),
            2 => assert_eq!(test.get::<WornOn>(item).unwrap().wearer, owner),
            3 => assert_eq!(test.get::<MountedOn>(item).unwrap().0, owner),
            _ => {}
        }
    }
}

#[test]
fn pickup_cannot_create_self_or_ancestor_ownership_cycles() {
    for self_pickup in [false, true] {
        let mut test = bed();
        let actor = actor(&mut test, 0, 100);
        let item = if self_pickup {
            actor
        } else {
            let item = test.spawn(position(0, 0, 0));
            let pocket = test.spawn(MountedOn(item));
            test.world_mut()
                .entity_mut(actor)
                .insert(InsideContainer(pocket));
            item
        };
        declare(&mut test, actor, ActionIntent::Pickup { item });
        resolve(&mut test);
        assert_outcome(&test, actor, ActionOutcomeState::Rejected, 100);
        assert!(test.get::<WorldPosition>(item).is_some());
    }
}

#[test]
fn later_move_sees_both_vacated_and_newly_occupied_solid_tiles() {
    for vacated in [false, true] {
        let mut test = bed();
        let first = actor(&mut test, 0, 200);
        let second = actor(&mut test, if vacated { -1 } else { 2 }, 100);
        test.world_mut().entity_mut(first).insert(Solid);
        test.world_mut().entity_mut(second).insert(Solid);
        declare(&mut test, first, ActionIntent::Move { dx: 1, dy: 0 });
        declare(
            &mut test,
            second,
            ActionIntent::Move {
                dx: if vacated { 1 } else { -1 },
                dy: 0,
            },
        );
        resolve(&mut test);
        assert_outcome(
            &test,
            first,
            ActionOutcomeState::Completed,
            200 - MOVE_COST_WALK,
        );
        assert_eq!(
            *test.get::<WorldPosition>(first).unwrap(),
            position(1, 0, 0)
        );
        if vacated {
            assert_outcome(
                &test,
                second,
                ActionOutcomeState::Completed,
                100 - MOVE_COST_WALK,
            );
            assert_eq!(
                *test.get::<WorldPosition>(second).unwrap(),
                position(0, 0, 0)
            );
        } else {
            assert_outcome(&test, second, ActionOutcomeState::Rejected, 100);
            assert_eq!(
                *test.get::<WorldPosition>(second).unwrap(),
                position(2, 0, 0)
            );
        }
    }
}

#[test]
fn later_request_uses_committed_actor_position_and_ap() {
    let mut test = bed();
    let actor = actor(&mut test, 0, 1000);
    let item = test.spawn(position(2, 0, 0));
    enqueue(&mut test, actor, 1, ActionIntent::Move { dx: 1, dy: 0 });
    enqueue(&mut test, actor, 2, ActionIntent::Pickup { item });
    // Direct call deliberately has no apply_deferred step: completion is real.
    resolve_intents(test.world_mut());
    assert_outcome(
        &test,
        actor,
        ActionOutcomeState::Completed,
        1000 - MOVE_COST_WALK - AP_COST_PICKUP,
    );
    assert_eq!(test.get::<InsideContainer>(item).unwrap().0, actor);
    assert_eq!(
        *test.get::<WorldPosition>(actor).unwrap(),
        position(1, 0, 0)
    );
    assert!(test.get::<WorldPosition>(item).is_none());

    test.world_mut()
        .get_mut::<ActionPoints>(actor)
        .unwrap()
        .current = 0;
    enqueue(&mut test, actor, 3, ActionIntent::Wait);
    enqueue(&mut test, actor, 4, ActionIntent::Move { dx: 1, dy: 0 });
    resolve_intents(test.world_mut());
    assert_outcome(&test, actor, ActionOutcomeState::Rejected, -100);
    assert_eq!(
        *test.get::<WorldPosition>(actor).unwrap(),
        position(1, 0, 0)
    );
}

#[test]
fn equal_ap_uses_sim_id_before_entity_fallback_and_stamps_in_order() {
    let mut test = bed();
    let high_id = actor(&mut test, 0, 100);
    let fallback_a = actor(&mut test, 0, 100);
    let low_id = actor(&mut test, 0, 100);
    let fallback_b = actor(&mut test, 0, 100);
    test.world_mut().entity_mut(high_id).insert(SimId(9));
    test.world_mut().entity_mut(low_id).insert(SimId(2));
    for actor in [high_id, fallback_a, low_id, fallback_b] {
        declare(&mut test, actor, ActionIntent::Wait);
    }
    test.run_system(collect_intents);
    let mut fallback = [fallback_a, fallback_b];
    fallback.sort_by_key(|entity| entity.to_bits());
    let order: Vec<_> = test
        .resource::<IntentQueue>()
        .queued
        .iter()
        .map(|p| p.entity)
        .collect();
    assert_eq!(order, vec![low_id, high_id, fallback[0], fallback[1]]);
    for (i, entity) in order.into_iter().enumerate() {
        assert_eq!(test.get::<ActionRequestId>(entity).unwrap().0, i as u64 + 1);
    }
}

#[test]
fn vanished_actor_and_item_reject_without_panicking_or_charging() {
    let mut test = bed();
    let vanished = actor(&mut test, 0, 200);
    let survivor = actor(&mut test, 0, 100);
    let item = test.spawn(position(0, 0, 0));
    declare(&mut test, vanished, ActionIntent::Wait);
    declare(&mut test, survivor, ActionIntent::Pickup { item });
    test.run_system(collect_intents);
    test.world_mut().despawn(vanished);
    test.world_mut().despawn(item);
    test.run_system(resolve_intents);
    assert_outcome(&test, survivor, ActionOutcomeState::Rejected, 100);
    assert_eq!(test.resource::<IntentQueue>().rejected, 2);
}
