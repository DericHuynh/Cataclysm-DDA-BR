//! Focused inventory actions share live exclusive resolution and no-cost rejection.
use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, HandCount, IsAlive};
use cdda_components::def::ItemVolume;
use cdda_components::intent::*;
use cdda_components::item::*;
use cdda_components::schedule::ActingEntity;
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_core_types::sim_id::SimId;
use cdda_sim::intent::systems::{collect_intents, resolve_intents};
use cdda_sim::inventory::pocket::spawn_body_pocket;
use cdda_sim::runtime::test_utils::TestBed;

fn pos(x: i32, z: i8) -> WorldPosition {
    WorldPosition::new(WorldPos::new(x, 0, ZLevel::new(z)))
}
fn bed() -> TestBed {
    let mut bed = TestBed::new();
    bed.insert_resource(IntentQueue::default());
    bed.insert_resource(ActionRequestCounter::default());
    bed
}
fn actor(bed: &mut TestBed, ap: i32) -> Entity {
    bed.spawn((
        IsAlive,
        ActionPoints {
            current: ap,
            speed: 100,
        },
        HandCount(2),
        pos(0, 0),
    ))
}
fn run(bed: &mut TestBed, actor: Entity, action: ActionIntent) {
    bed.world_mut().entity_mut(actor).insert(action);
    bed.run_system(collect_intents);
    bed.run_system(resolve_intents);
}
fn outcome(bed: &TestBed, actor: Entity, state: ActionOutcomeState, ap: i32) {
    let outcome = bed.get::<ActionOutcome>(actor).unwrap();
    assert_eq!(outcome.state, state);
    assert_eq!(outcome.request, *bed.get::<ActionRequestId>(actor).unwrap());
    assert_eq!(bed.get::<ActionPoints>(actor).unwrap().current, ap);
}
fn enqueue(bed: &mut TestBed, actor: Entity, id: u64, intent: ActionIntent) {
    let request = ActionRequestId(id);
    bed.world_mut().entity_mut(actor).insert(request);
    bed.resource_mut::<IntentQueue>().queued.push(QueuedIntent {
        request,
        entity: actor,
        intent,
        ap: 1000,
    });
}

#[test]
fn nested_wield_stow_drop_update_real_ownership_and_reverse_links() {
    let mut bed = bed();
    let actor = actor(&mut bed, 500);
    let body = spawn_body_pocket(bed.world_mut(), actor);
    let coat = bed.spawn(WornOn {
        wearer: actor,
        slot: None,
    });
    let pocket = bed.spawn((IsPocket, MountedOn(coat)));
    let item = bed.spawn((InsideContainer(pocket), Invlet('a')));
    run(&mut bed, actor, ActionIntent::Wield { item });
    outcome(&bed, actor, ActionOutcomeState::Completed, 400);
    assert_eq!(bed.get::<WieldedBy>(item).unwrap().0, actor);
    assert!(bed.get::<InsideContainer>(item).is_none());
    assert!(!bed
        .get::<ContainerContents>(pocket)
        .is_some_and(|c| c.iter().any(|e| e == item)));
    run(&mut bed, actor, ActionIntent::Stow { item });
    outcome(&bed, actor, ActionOutcomeState::Completed, 300);
    assert_eq!(bed.get::<InsideContainer>(item).unwrap().0, body);
    assert!(bed.get::<WieldedBy>(item).is_none());
    bed.world_mut().entity_mut(actor).insert(pos(9, 1));
    run(&mut bed, actor, ActionIntent::Drop { item });
    outcome(&bed, actor, ActionOutcomeState::Completed, 200);
    assert_eq!(*bed.get::<WorldPosition>(item).unwrap(), pos(9, 1));
    assert!(bed.get::<InsideContainer>(item).is_none());
    assert!(bed.get::<Invlet>(item).is_none());
}

#[test]
fn stow_without_body_pocket_uses_loose_inventory() {
    let mut bed = bed();
    let actor = actor(&mut bed, 100);
    let item = bed.spawn(WieldedBy(actor));
    run(&mut bed, actor, ActionIntent::Stow { item });
    outcome(&bed, actor, ActionOutcomeState::Completed, 0);
    assert_eq!(bed.get::<InsideContainer>(item).unwrap().0, actor);
}

#[test]
fn ground_wield_requires_live_same_z_reach() {
    for (x, z, success) in [
        (0, 0, true),
        (1, 0, true),
        (2, 0, false),
        (0, 1, false),
        (i32::MIN, 0, false),
    ] {
        let mut bed = bed();
        let actor = actor(&mut bed, 100);
        let item = bed.spawn(pos(x, z));
        run(&mut bed, actor, ActionIntent::Wield { item });
        outcome(
            &bed,
            actor,
            if success {
                ActionOutcomeState::Completed
            } else {
                ActionOutcomeState::Rejected
            },
            if success { 0 } else { 100 },
        );
        assert_eq!(bed.get::<WieldedBy>(item).is_some(), success);
        assert_eq!(bed.get::<WorldPosition>(item).is_some(), !success);
    }
}

#[test]
fn hands_limit_observes_prior_commits_and_missing_hands_rejects() {
    let mut bed = bed();
    let actor = actor(&mut bed, 500);
    bed.world_mut().entity_mut(actor).insert(HandCount(1));
    let first = bed.spawn(pos(0, 0));
    let second = bed.spawn(pos(0, 0));
    enqueue(&mut bed, actor, 1, ActionIntent::Wield { item: first });
    enqueue(&mut bed, actor, 2, ActionIntent::Wield { item: second });
    resolve_intents(bed.world_mut());
    outcome(&bed, actor, ActionOutcomeState::Rejected, 400);
    assert_eq!(bed.get::<WieldedBy>(first).unwrap().0, actor);
    assert!(bed.get::<WieldedBy>(second).is_none());
    bed.world_mut().entity_mut(actor).remove::<HandCount>();
    run(&mut bed, actor, ActionIntent::Wield { item: second });
    outcome(&bed, actor, ActionOutcomeState::Rejected, 400);
}

#[test]
fn other_actor_owned_items_cannot_be_wielded_dropped_or_stowed() {
    for action in 0..3 {
        let mut bed = bed();
        let actor = actor(&mut bed, 100);
        let owner = bed.spawn(pos(0, 0));
        let bag = bed.spawn(WornOn {
            wearer: owner,
            slot: None,
        });
        let pocket = bed.spawn((IsPocket, MountedOn(bag)));
        let item = bed.spawn(InsideContainer(pocket));
        let intent = match action {
            0 => ActionIntent::Wield { item },
            1 => ActionIntent::Drop { item },
            _ => ActionIntent::Stow { item },
        };
        run(&mut bed, actor, intent);
        outcome(&bed, actor, ActionOutcomeState::Rejected, 100);
        assert_eq!(bed.get::<InsideContainer>(item).unwrap().0, pocket);
    }
}

#[test]
fn ambiguous_locations_reject_without_repairing_or_charging() {
    for action in 0..4 {
        let mut bed = bed();
        let actor = actor(&mut bed, 100);
        let item = bed.spawn((InsideContainer(actor), WieldedBy(actor), pos(0, 0)));
        let intent = match action {
            0 => ActionIntent::Pickup { item },
            1 => ActionIntent::Wield { item },
            2 => ActionIntent::Drop { item },
            _ => ActionIntent::Stow { item },
        };
        run(&mut bed, actor, intent);
        outcome(&bed, actor, ActionOutcomeState::Rejected, 100);
        assert_eq!(bed.get::<InsideContainer>(item).unwrap().0, actor);
        assert_eq!(bed.get::<WieldedBy>(item).unwrap().0, actor);
        assert!(bed.get::<WorldPosition>(item).is_some());
    }
}

#[test]
fn ownership_cycles_and_self_targets_reject_without_hanging() {
    let mut bed = bed();
    let actor = actor(&mut bed, 100);
    let bag = bed.spawn(());
    let item = bed.spawn(InsideContainer(bag));
    bed.world_mut()
        .entity_mut(bag)
        .insert(InsideContainer(item));
    run(&mut bed, actor, ActionIntent::Wield { item });
    outcome(&bed, actor, ActionOutcomeState::Rejected, 100);
    run(&mut bed, actor, ActionIntent::Drop { item });
    outcome(&bed, actor, ActionOutcomeState::Rejected, 100);
    run(&mut bed, actor, ActionIntent::Wield { item: actor });
    outcome(&bed, actor, ActionOutcomeState::Rejected, 100);
}

#[test]
fn stow_cannot_move_a_bag_into_its_own_pocket() {
    let mut bed = bed();
    let actor = actor(&mut bed, 100);
    let bag = bed.spawn(WieldedBy(actor));
    let pocket = bed.spawn((IsPocket, MountedOn(bag)));
    run(&mut bed, actor, ActionIntent::Stow { item: bag });
    outcome(&bed, actor, ActionOutcomeState::Completed, 0);
    assert_eq!(bed.get::<InsideContainer>(bag).unwrap().0, actor);
    assert_eq!(bed.get::<MountedOn>(pocket).unwrap().0, bag);
}

#[test]
fn drop_clears_nested_invlets_but_preserves_the_subtree() {
    let mut bed = bed();
    let actor = actor(&mut bed, 100);
    let bag = bed.spawn((
        WornOn {
            wearer: actor,
            slot: None,
        },
        Invlet('b'),
    ));
    let pocket = bed.spawn((IsPocket, MountedOn(bag)));
    let item = bed.spawn((InsideContainer(pocket), Invlet('a')));
    run(&mut bed, actor, ActionIntent::Drop { item: bag });
    outcome(&bed, actor, ActionOutcomeState::Completed, 0);
    assert_eq!(bed.get::<InsideContainer>(item).unwrap().0, pocket);
    assert!(bed.get::<Invlet>(bag).is_none());
    assert!(bed.get::<Invlet>(item).is_none());
    assert!(bed.get::<WornOn>(bag).is_none());
    assert_eq!(*bed.get::<WorldPosition>(bag).unwrap(), pos(0, 0));
}

#[test]
fn drop_checks_live_floor_capacity_on_exact_tile() {
    let mut bed = bed();
    let actor = actor(&mut bed, 500);
    let first = bed.spawn((InsideContainer(actor), ItemVolume(FLOOR_CAP_ML)));
    let second = bed.spawn((InsideContainer(actor), ItemVolume(1)));
    enqueue(&mut bed, actor, 1, ActionIntent::Drop { item: first });
    enqueue(&mut bed, actor, 2, ActionIntent::Drop { item: second });
    resolve_intents(bed.world_mut());
    outcome(&bed, actor, ActionOutcomeState::Rejected, 400);
    assert!(bed.get::<WorldPosition>(first).is_some());
    assert_eq!(bed.get::<InsideContainer>(second).unwrap().0, actor);
    bed.world_mut().entity_mut(actor).insert(pos(1, 0));
    run(&mut bed, actor, ActionIntent::Drop { item: second });
    outcome(&bed, actor, ActionOutcomeState::Completed, 300);
}

#[test]
fn selected_actor_collection_leaves_other_requests_untouched() {
    let mut bed = bed();
    let selected = actor(&mut bed, 100);
    let other = actor(&mut bed, 200);
    bed.world_mut()
        .entity_mut(selected)
        .insert(ActionIntent::Wait);
    bed.world_mut().entity_mut(other).insert(ActionIntent::Wait);
    bed.insert_resource(ActingEntity(selected));
    bed.run_system(collect_intents);
    assert_eq!(bed.resource::<IntentQueue>().queued.len(), 1);
    assert_eq!(bed.resource::<IntentQueue>().queued[0].entity, selected);
    assert!(bed.get::<ActionIntent>(other).is_some());
    assert!(bed.get::<ActionRequestId>(other).is_none());
    bed.world_mut().remove_resource::<ActingEntity>();
    bed.run_system(collect_intents);
    assert_eq!(bed.resource::<IntentQueue>().queued[0].entity, other);
}

#[test]
fn stow_pocket_selection_uses_stable_sim_id() {
    let mut bed = bed();
    let actor = actor(&mut bed, 100);
    let high = spawn_body_pocket(bed.world_mut(), actor);
    bed.world_mut().entity_mut(high).insert(SimId(20));
    let low = spawn_body_pocket(bed.world_mut(), actor);
    bed.world_mut().entity_mut(low).insert(SimId(10));
    let item = bed.spawn(WieldedBy(actor));
    run(&mut bed, actor, ActionIntent::Stow { item });
    assert_eq!(bed.get::<InsideContainer>(item).unwrap().0, low);
}

#[test]
fn live_eligibility_and_disappearing_targets_reject_without_cost() {
    for mode in 0..4 {
        let mut bed = bed();
        let actor = actor(&mut bed, 100);
        let item = bed.spawn(InsideContainer(actor));
        bed.world_mut()
            .entity_mut(actor)
            .insert(ActionIntent::Wield { item });
        bed.run_system(collect_intents);
        let expected_ap = if mode == 0 { -1 } else { 100 };
        match mode {
            0 => {
                bed.world_mut()
                    .get_mut::<ActionPoints>(actor)
                    .unwrap()
                    .current = -1
            }
            1 => {
                bed.world_mut().entity_mut(actor).remove::<IsAlive>();
            }
            2 => {
                bed.world_mut().despawn(item);
            }
            _ => {
                bed.world_mut().entity_mut(actor).remove::<WorldPosition>();
            }
        }
        bed.run_system(resolve_intents);
        outcome(&bed, actor, ActionOutcomeState::Rejected, expected_ap);
    }
}

#[test]
fn zero_ap_low_level_wield_may_enter_debt() {
    let mut bed = bed();
    let actor = actor(&mut bed, 0);
    let item = bed.spawn(InsideContainer(actor));
    run(&mut bed, actor, ActionIntent::Wield { item });
    outcome(&bed, actor, ActionOutcomeState::Completed, -100);
}

#[test]
fn pickup_and_wield_contend_in_the_same_live_resolver() {
    let mut bed = bed();
    let winner = actor(&mut bed, 200);
    let loser = actor(&mut bed, 100);
    let item = bed.spawn(pos(0, 0));
    bed.world_mut()
        .entity_mut(winner)
        .insert(ActionIntent::Wield { item });
    bed.world_mut()
        .entity_mut(loser)
        .insert(ActionIntent::Pickup { item });
    bed.run_system(collect_intents);
    bed.run_system(resolve_intents);
    outcome(&bed, winner, ActionOutcomeState::Completed, 100);
    outcome(&bed, loser, ActionOutcomeState::Rejected, 100);
    assert_eq!(bed.get::<WieldedBy>(item).unwrap().0, winner);
    assert!(bed.get::<InsideContainer>(item).is_none());
    assert!(bed.get::<WorldPosition>(item).is_none());
}

#[test]
fn ownership_and_range_changes_after_collection_are_revalidated() {
    for ground in [false, true] {
        let mut bed = bed();
        let actor = actor(&mut bed, 100);
        let other = bed.spawn(pos(0, 0));
        let item = bed.spawn(InsideContainer(actor));
        bed.world_mut()
            .entity_mut(actor)
            .insert(ActionIntent::Wield { item });
        bed.run_system(collect_intents);
        if ground {
            bed.world_mut()
                .entity_mut(item)
                .remove::<InsideContainer>()
                .insert(pos(3, 0));
        } else {
            bed.world_mut()
                .entity_mut(item)
                .insert(InsideContainer(other));
        }
        bed.run_system(resolve_intents);
        outcome(&bed, actor, ActionOutcomeState::Rejected, 100);
        assert!(bed.get::<WieldedBy>(item).is_none());
    }
}
