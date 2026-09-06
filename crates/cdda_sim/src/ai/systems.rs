//! AI systems — per-planner behaviour dispatch feeding the shared intent queue.
//!
//! An AI entity wears exactly one planner marker (`PlannerBehaviourTree`,
//! `PlannerGoap`, `PlannerHtn`, or the inert `PlannerNone`).  A dedicated
//! system per planner type runs only for entities carrying that marker
//! (`.run_if(With<PlannerX>)`), decides an [`AiGoal`], and translates it into
//! an [`ActionIntent`] component on the entity.
//!
//! The intents **do not get resolved here** — they flow into the same buffered
//! `IntentQueue` that `collect_intents` sorts by action points and
//! `resolve_intents` drains during `SimSet::IntentResolve`.  Because the player
//! also declares an intent via `collect_intents` (never resolved inline), there
//! is **no player-first guarantee**: the highest-AP actor that turn acts first.
//!
//! ## Adding a planner
//!
//! 1. Add a marker component in `cdda_components::ai`.
//! 2. Add a `drive_<planner>` system here and register it in `AiPlugin`
//!    with `.run_if(With<PlannerX>)`.
//! 3. Have it produce an [`AiGoal`] and call [`declare_goal`].

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, IsAlive};
use cdda_components::ai::{AiGoal, PlannerBehaviourTree, PlannerGoap, PlannerHtn};
use cdda_components::def::IsDef;
use cdda_components::intent::ActionIntent;
use cdda_components::schedule::ActingEntity;
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::WorldPos;

// ---------------------------------------------------------------------------
// Intent declaration helper
// ---------------------------------------------------------------------------

/// Translate a chosen [`AiGoal`] into an [`ActionIntent`] and write it onto the
/// entity.  `Idle`/`Guard` emit nothing (inert) — the entity simply does not
/// act this turn.
///
/// This is the single seam between "what the planner wants" and "how the sim
/// acts"; adding a goal branch does not touch intent resolution.
pub fn declare_goal(commands: &mut Commands, entity: Entity, goal: AiGoal) {
    let intent = match goal {
        AiGoal::Attack { target } => ActionIntent::MeleeAttack { target },
        // A planner that settles on a movement goal emits a concrete single-tile
        // step. Real planners (BT/GOAP/HTN) pick the direction from pathing;
        // the stand-in wanders +x so the intent genuinely enters the queue.
        AiGoal::Hunt { .. } | AiGoal::Wander | AiGoal::Flee { .. } | AiGoal::Guard { .. } => {
            ActionIntent::Move { dx: 1, dy: 0 }
        }
        AiGoal::Interact => ActionIntent::Wait,
        AiGoal::Idle => return,
    };
    commands.entity(entity).insert(intent);
}

// ---------------------------------------------------------------------------
// Planner decision stubs
// ---------------------------------------------------------------------------

/// Dumb, predictable mobs: a behaviour tree is a cheap fixed-rule decision. The
/// stand-in picks `Wander`; a real BT would select attack/flee/move from data.
fn behaviour_tree_goal(
    _entity: Entity,
    _pos: Option<&WorldPosition>,
    _ap: Option<&ActionPoints>,
) -> AiGoal {
    AiGoal::Wander
}

/// Goal-driven mobs (feral zombies): appear out of world state. Stand-in:
/// wander.
fn goap_goal(_entity: Entity, _pos: Option<&WorldPosition>, _ap: Option<&ActionPoints>) -> AiGoal {
    AiGoal::Wander
}

// ---------------------------------------------------------------------------
// Per-marker systems (run_if guards in the plugin)
// ---------------------------------------------------------------------------

/// Behaviour tree planner system. Under the budget scheduler only the selected
/// `ActingEntity` declares; without that resource (direct test calls) all
/// agents declare as before.
pub fn drive_behaviour_tree(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    q: Query<
        (Entity, &WorldPosition, &ActionPoints),
        (With<PlannerBehaviourTree>, With<IsAlive>, Without<IsDef>),
    >,
) {
    for (e, p, a) in &q {
        if acting.as_ref().is_some_and(|a| a.0 != e) {
            continue;
        }
        declare_goal(&mut commands, e, behaviour_tree_goal(e, Some(p), Some(a)));
    }
}

/// GOAP planner system. Same selection gate as the behaviour-tree driver.
pub fn drive_goap(
    mut commands: Commands,
    acting: Option<Res<ActingEntity>>,
    q: Query<
        (Entity, &WorldPosition, &ActionPoints),
        (With<PlannerGoap>, With<IsAlive>, Without<IsDef>),
    >,
) {
    for (e, p, a) in &q {
        if acting.as_ref().is_some_and(|a| a.0 != e) {
            continue;
        }
        declare_goal(&mut commands, e, goap_goal(e, Some(p), Some(a)));
    }
}

/// Inert planner — exists so `PlannerNone` can be a valid (non-acting) marker.
pub fn drive_none() {}

// ---------------------------------------------------------------------------
// Run conditions (used by the plugin's `.run_if(...)`)
// ---------------------------------------------------------------------------

/// True when at least one `PlannerBehaviourTree` AI entity exists.
pub fn has_behaviour_tree_agents(q: Query<(), With<PlannerBehaviourTree>>) -> bool {
    !q.is_empty()
}

/// True when at least one `PlannerGoap` AI entity exists.
pub fn has_goap_agents(q: Query<(), With<PlannerGoap>>) -> bool {
    !q.is_empty()
}

/// True when at least one `PlannerHtn` AI entity exists.
pub fn has_htn_agents(q: Query<(), With<PlannerHtn>>) -> bool {
    !q.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::systems::collect_intents;
    use crate::runtime::test_utils::TestBed;
    use cdda_components::intent::{ActionRequestCounter, IntentQueue};

    /// A GOAP-planner mob's `drive_goap` system must emit an `ActionIntent` that
    /// reaches the shared AP-sorted buffer — i.e. monsters participate in turn
    /// ordering alongside the player.
    #[test]
    fn goap_mob_declares_intent_into_shared_queue() {
        let mut test = TestBed::new();
        test.insert_resource(IntentQueue::default());
        test.insert_resource(ActionRequestCounter::default());
        test.register::<ActionIntent>()
            .register::<ActionPoints>()
            .register::<IsAlive>()
            .register::<WorldPosition>()
            .register::<PlannerGoap>();

        let z = cdda_core_types::core::coords::ZLevel::new(0);
        let mob = test.spawn((
            PlannerGoap,
            ActionPoints::new(100),
            IsAlive,
            WorldPosition(WorldPos::new(0, 0, z)),
        ));

        // GOAP planner runs → writes an ActionIntent onto the mob.
        test.run_system(drive_goap);

        // Then the intent collector buffers + sorts it.
        test.run_system(collect_intents);

        let queue = test.resource::<IntentQueue>();
        assert_eq!(
            queue.queued.len(),
            1,
            "GOAP mob's intent must reach the queue"
        );
        assert_eq!(queue.queued[0].entity, mob);
        assert!(matches!(queue.queued[0].intent, ActionIntent::Move { .. }));
    }
}
