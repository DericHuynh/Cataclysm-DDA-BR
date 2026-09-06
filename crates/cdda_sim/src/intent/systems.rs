//! # Intent resolution — turn ordering by action points, correlated results
//!
//! Handles the intent → action pipeline:
//!
//! 1. **Declare** (`SimSet::IntentDeclare`): AI + player input insert `ActionIntent` components.
//!    `collect_intents` gathers them into a global `IntentQueue`, sorted by AP descending,
//!    stamping each request with a correlated [`ActionRequestId`].
//! 2. **Resolve** (`SimSet::IntentResolve`): `resolve_intents` drains the queue, validates
//!    preconditions, and commits actions under exclusive world access. Later intents
//!    see earlier committed positions, ownership and AP before they validate.
//!
//! ## Correlated results (submission ≠ completion)
//!
//! After resolving each request the simulation writes an [`ActionOutcome`]
//! component onto the acting entity with the terminal verdict for **that
//! request id**:
//!
//! - `Completed` — Move / Wait / Pickup / Wield / Drop / Stow was committed.
//! - `Rejected` — refused before execution (dead actor, negative/missing AP,
//!   absent position, invalid/blocked move, missing/out-of-range/owned pickup
//!   target). No AP charged.
//! - `Failed` — accepted but not implemented on the intent path (UseItem,
//!   Reload, StartRead, Interact, MeleeAttack, StartCraft). **Nothing
//!   is performed and no AP is charged** — an unsupported action must never
//!   report success.
//!
//! Terminal outcomes persist until the actor's next declaration replaces
//! them; consumers must match on the request id. While no outcome (or an
//! older one) exists the request is pending/running.
//!
//! ## Relationship to activities
//!
//! Multi-turn activities (crafting, reading, …) are started through their
//! authoritative use-case paths, not through stub intent resolvers; until an
//! intent type has a real simulation operation behind it, resolving it is a
//! `Failed` outcome, never a silent AP burn.

use bevy_ecs::prelude::*;

use crate::inventory::transfer::{apply_inventory_action, InventoryAction};
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::intent::{
    ActionIntent, ActionOutcome, ActionOutcomeState, ActionRequestCounter, ActionRequestId,
    IntentQueue, QueuedIntent,
};
use cdda_components::schedule::ActingEntity;
use cdda_components::sim::{Solid, WorldPosition};
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::sim_id::SimId;

use crate::actor::turn::MOVE_COST_WALK;
use tracing::info;

// ---------------------------------------------------------------------------
// collect_intents — gather all ActionIntent components, sort by AP
// ---------------------------------------------------------------------------

/// Collects the selected `ActingEntity`'s intent when that resource exists;
/// otherwise collects all actors for isolated callers. Builds `IntentQueue`
/// by AP descending (highest AP acts first), then `SimId` ascending. Actors
/// with IDs precede entities without IDs at equal AP; test entities without a
/// `SimId` fall back to `Entity::to_bits()` (stable only within that world).
/// Duplicate IDs also fall back to Entity. IDs must be unique for replay order.
/// Each request is stamped in this order with a fresh [`ActionRequestId`].
/// Removes the `ActionIntent` component from each entity after collection.
pub fn collect_intents(
    mut commands: Commands,
    mut counter: ResMut<ActionRequestCounter>,
    q_intents: Query<(Entity, &ActionIntent, &ActionPoints, Option<&SimId>)>,
    mut queue: ResMut<IntentQueue>,
    acting: Option<Res<ActingEntity>>,
) {
    queue.queued.clear();
    queue.rejected = 0;

    let mut entries: Vec<_> = q_intents
        .iter()
        .filter(|(entity, _, _, _)| acting.as_ref().is_none_or(|acting| acting.0 == *entity))
        .collect();
    entries.sort_by_key(|(entity, _, ap, id)| {
        (
            std::cmp::Reverse(ap.current),
            id.is_none(),
            id.map(|id| id.0).unwrap_or_default(),
            entity.to_bits(),
        )
    });

    for (entity, intent, ap, _) in entries {
        let request: ActionRequestId = counter.next();
        commands
            .entity(entity)
            .insert(request)
            .remove::<ActionIntent>();
        queue.queued.push(QueuedIntent {
            request,
            entity,
            intent: intent.clone(),
            ap: ap.current,
        });
    }
}

// ---------------------------------------------------------------------------
// resolve_intents — drain IntentQueue, validate, execute, report
// ---------------------------------------------------------------------------

/// Resolve intents in AP-priority order against the live world, committing each
/// action and its AP cost BEFORE publishing its correlated terminal outcome.
/// Exclusive access is intentional: deferred Commands would let later requests
/// validate against stale positions/ownership and both claim the same item.
pub fn resolve_intents(world: &mut World) {
    let intents = std::mem::take(&mut world.resource_mut::<IntentQueue>().queued);
    for pending in intents {
        // Starting at zero AP may enter debt; negative AP cannot act. Recheck
        // the live balance, not the collection-time priority snapshot.
        let can_act = world.get::<IsAlive>(pending.entity).is_some()
            && world
                .get::<ActionPoints>(pending.entity)
                .is_some_and(|ap| ap.current >= 0);
        let state = if !can_act {
            ActionOutcomeState::Rejected
        } else {
            match &pending.intent {
                ActionIntent::Move { dx, dy } => resolve_move(world, pending.entity, *dx, *dy),
                ActionIntent::Pickup { item } => {
                    resolve_inventory(world, pending.entity, *item, InventoryAction::Pickup)
                }
                ActionIntent::Wield { item } => {
                    resolve_inventory(world, pending.entity, *item, InventoryAction::Wield)
                }
                ActionIntent::Drop { item } => {
                    resolve_inventory(world, pending.entity, *item, InventoryAction::Drop)
                }
                ActionIntent::Stow { item } => {
                    resolve_inventory(world, pending.entity, *item, InventoryAction::Stow)
                }
                ActionIntent::Wait => {
                    spend_ap(world, pending.entity, 100);
                    ActionOutcomeState::Completed
                }
                ActionIntent::MeleeAttack { .. }
                | ActionIntent::UseItem { .. }
                | ActionIntent::Reload { .. }
                | ActionIntent::StartRead { .. }
                | ActionIntent::Interact { .. }
                | ActionIntent::StartCraft { .. } => {
                    // Unsupported is not success, and never burns AP.
                    info!(
                        "action request {:?} for entity {:?} failed: {:?} is not implemented on the intent path",
                        pending.request, pending.entity, pending.intent
                    );
                    ActionOutcomeState::Failed
                }
            }
        };
        if state != ActionOutcomeState::Completed {
            world.resource_mut::<IntentQueue>().rejected += 1;
        }
        // A despawned actor cannot hold a component; count its rejection without
        // issuing a deferred command to a nonexistent entity (or resurrecting it).
        if let Ok(mut actor) = world.get_entity_mut(pending.entity) {
            actor.insert(ActionOutcome::new(pending.request, state));
        }
    }
}

fn spend_ap(world: &mut World, actor: Entity, cost: i32) {
    world
        .get_mut::<ActionPoints>(actor)
        .expect("actor AP validated before exclusive commit")
        .spend(cost);
}

/// Commit a legal, nonzero one-tile step without occupying a Solid entity's
/// current tile. Scan ECS positions rather than a potentially stale spatial
/// index; terrain collision without a Solid entity is not represented here.
fn resolve_move(world: &mut World, actor: Entity, dx: i32, dy: i32) -> ActionOutcomeState {
    if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dy) || (dx == 0 && dy == 0) {
        return ActionOutcomeState::Rejected;
    }
    let Some(pos) = world.get::<WorldPosition>(actor).map(WorldPosition::get) else {
        return ActionOutcomeState::Rejected;
    };
    let (Some(x), Some(y)) = (pos.x.checked_add(dx), pos.y.checked_add(dy)) else {
        return ActionOutcomeState::Rejected;
    };
    let destination = WorldPos::new(x, y, pos.z);
    let mut solids = world.query_filtered::<(Entity, &WorldPosition), With<Solid>>();
    if solids
        .iter(world)
        .any(|(entity, position)| entity != actor && position.get() == destination)
    {
        return ActionOutcomeState::Rejected;
    }

    world
        .entity_mut(actor)
        .insert(WorldPosition::new(destination));
    spend_ap(world, actor, MOVE_COST_WALK);
    ActionOutcomeState::Completed
}

fn resolve_inventory(
    world: &mut World,
    actor: Entity,
    item: Entity,
    action: InventoryAction,
) -> ActionOutcomeState {
    match apply_inventory_action(world, actor, item, action) {
        Ok(()) => ActionOutcomeState::Completed,
        Err(_) => ActionOutcomeState::Rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::turn::AP_COST_PICKUP;
    use crate::runtime::test_utils::TestBed;
    use cdda_components::item::InsideContainer;

    fn bed() -> TestBed {
        let mut test = TestBed::new();
        test.insert_resource(IntentQueue::default());
        test.insert_resource(ActionRequestCounter::default());
        test.register::<ActionIntent>()
            .register::<ActionPoints>()
            .register::<IsAlive>()
            .register::<WorldPosition>()
            .register::<ActionRequestId>()
            .register::<ActionOutcome>();
        test
    }

    /// A monster with **more** AP than the player must be buffered and resolved
    /// first — there is no player-first guarantee. This is the fairness contract
    /// that distinguishes the rewrite from CDDA's blocking player loop.
    #[test]
    fn higher_ap_monster_goes_before_lower_ap_player() {
        let mut test = bed();

        // Monster at 150 AP, player at 50 AP.
        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let monster = test.spawn((
            ActionPoints {
                current: 150,
                speed: 100,
            },
            IsAlive,
            ActionIntent::Wait,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));
        let player = test.spawn((
            ActionPoints {
                current: 50,
                speed: 100,
            },
            IsAlive,
            ActionIntent::Wait,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));

        // Buffer + sort by AP: collect_intents writes the AP-sorted queue.
        test.run_system(collect_intents);

        let queue = test.resource::<IntentQueue>();
        assert_eq!(queue.queued.len(), 2, "both intents buffered");
        assert_eq!(
            queue.queued[0].entity, monster,
            "highest-AP (monster, 150) must act first — no player priority"
        );
        assert_eq!(queue.queued[1].entity, player, "player (50 AP) second");
    }

    /// Every collected request carries a correlated id that lands on the actor,
    /// and after resolution the actor holds the terminal verdict for *that* id.
    #[test]
    fn wait_intent_reports_a_correlated_completed_outcome() {
        let mut test = bed();
        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let actor = test.spawn((
            ActionPoints {
                current: 100,
                speed: 100,
            },
            IsAlive,
            ActionIntent::Wait,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));

        test.run_system(collect_intents);
        let request = test
            .world()
            .get::<ActionRequestId>(actor)
            .copied()
            .expect("stamped");
        assert_eq!(test.resource::<IntentQueue>().queued[0].request, request);

        test.run_system(resolve_intents);

        let outcome = test
            .world()
            .get::<ActionOutcome>(actor)
            .copied()
            .expect("outcome");
        assert!(
            outcome.matches(request),
            "verdict matches the stamped request"
        );
        assert_eq!(outcome.state, ActionOutcomeState::Completed);
        assert_eq!(
            test.world().get::<ActionPoints>(actor).unwrap().current,
            0,
            "wait burns 100 AP exactly once"
        );
    }

    /// A dead actor's request is `Rejected` with no AP charged; a moved actor's
    /// request is `Completed` and its position genuinely changed.
    #[test]
    fn move_completes_and_dead_actors_are_rejected_without_ap_charge() {
        let mut test = bed();
        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let mover = test.spawn((
            ActionPoints {
                current: 100,
                speed: 100,
            },
            IsAlive,
            ActionIntent::Move { dx: 1, dy: 0 },
            WorldPosition(WorldPos::new(5, 5, z)),
        ));
        let corpse = test.spawn((
            ActionPoints {
                current: 100,
                speed: 100,
            },
            ActionIntent::Wait,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));
        // `ActionPoints` requires `IsAlive`, so the spawn above auto-inserted
        // it — strip it to make this actor genuinely dead.
        test.world_mut().entity_mut(corpse).remove::<IsAlive>();

        test.run_system(collect_intents);
        test.run_system(resolve_intents);

        let mover_outcome = test.world().get::<ActionOutcome>(mover).copied().unwrap();
        assert_eq!(mover_outcome.state, ActionOutcomeState::Completed);
        assert_eq!(test.world().get::<WorldPosition>(mover).unwrap().get().x, 6);
        assert_eq!(
            test.world().get::<ActionPoints>(mover).unwrap().current,
            100 - MOVE_COST_WALK
        );

        let corpse_outcome = test.world().get::<ActionOutcome>(corpse).copied().unwrap();
        assert_eq!(corpse_outcome.state, ActionOutcomeState::Rejected);
        assert_eq!(
            test.world().get::<ActionPoints>(corpse).unwrap().current,
            100,
            "rejected requests charge no AP"
        );
    }

    /// Unsupported intent types must NOT report success: they resolve to
    /// `Failed`, perform nothing, and charge no AP.
    #[test]
    fn unsupported_intents_fail_without_performing_or_charging() {
        let mut test = bed();
        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let actor = test.spawn((
            ActionPoints {
                current: 100,
                speed: 100,
            },
            IsAlive,
            ActionIntent::UseItem {
                item: Entity::PLACEHOLDER,
            },
            WorldPosition(WorldPos::new(0, 0, z)),
        ));

        test.run_system(collect_intents);
        test.run_system(resolve_intents);

        let outcome = test.world().get::<ActionOutcome>(actor).copied().unwrap();
        assert_eq!(
            outcome.state,
            ActionOutcomeState::Failed,
            "an unimplemented operation must never report Completed"
        );
        assert_eq!(
            test.world().get::<ActionPoints>(actor).unwrap().current,
            100,
            "failed operations charge no AP"
        );
        assert!(test.resource::<IntentQueue>().rejected >= 1);
    }

    /// Pickup routes through the authoritative mutation: `InsideContainer`
    /// inserted AND the ground position removed, so the item stops being a
    /// world object the moment it is carried.
    #[test]
    fn pickup_inserts_container_and_removes_ground_position() {
        let mut test = bed();
        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let actor = test.spawn((
            ActionPoints {
                current: 100,
                speed: 100,
            },
            IsAlive,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));
        let item = test.spawn(WorldPosition(WorldPos::new(1, 0, z)));
        test.world_mut()
            .entity_mut(actor)
            .insert(ActionIntent::Pickup { item });

        test.run_system(collect_intents);
        test.run_system(resolve_intents);

        let outcome = test.world().get::<ActionOutcome>(actor).copied().unwrap();
        assert_eq!(outcome.state, ActionOutcomeState::Completed);
        assert!(test.world().get::<InsideContainer>(item).is_some());
        assert!(
            test.world().get::<WorldPosition>(item).is_none(),
            "carried items must not keep a ground position"
        );
        assert_eq!(
            test.world().get::<ActionPoints>(actor).unwrap().current,
            100 - AP_COST_PICKUP
        );
    }

    /// Request ids are monotonic across turns so a stale outcome can never be
    /// mistaken for the current request's verdict.
    #[test]
    fn request_ids_increase_monotonically() {
        let mut test = bed();
        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let actor = test.spawn((
            ActionPoints {
                current: 1000,
                speed: 100,
            },
            IsAlive,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));

        for turn in 0..3 {
            test.world_mut()
                .entity_mut(actor)
                .insert(ActionIntent::Wait);
            test.run_system(collect_intents);
            let id = test.world().get::<ActionRequestId>(actor).copied().unwrap();
            assert_eq!(id.0, turn + 1, "one fresh id per declared request");
        }
    }
}
