//! ECS systems for the activity system.
//!
//! Drives the multi-turn activity lifecycle each simulation tick.
//! The actor methods receive individual mutable fields extracted from
//! `PlayerActivity` so callers never hold a borrow of the component while
//! also needing `&mut World`.

use bevy_ecs::prelude::*;

use crate::activity::components::{ActivityPhase, PlayerActivity};
use crate::activity::tracker::ActivityTracker;

// ---------------------------------------------------------------------------
// start_pending_activities
// ---------------------------------------------------------------------------

/// Call `actor.start()` on every character whose `PlayerActivity` is `Pending`.
/// Must run before `tick_activities` each turn.
pub fn start_pending_activities(world: &mut World) {
    let pending: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerActivity>>()
        .iter(world)
        .filter(|&e| {
            world
                .get::<PlayerActivity>(e)
                .map(|a| a.phase == ActivityPhase::Pending)
                .unwrap_or(false)
        })
        .collect();

    for entity in pending {
        let mut actor = {
            let mut act = world.get_mut::<PlayerActivity>(entity).unwrap();
            act.phase = ActivityPhase::Active;
            act.actor.take()
        };

        if let Some(ref mut a) = actor {
            let (mut moves_total, mut moves_left) = (0i32, 0i32);
            a.start(&mut moves_total, &mut moves_left, entity, world);
            if let Some(mut act) = world.get_mut::<PlayerActivity>(entity) {
                act.moves_total = moves_total;
                act.moves_left = moves_left;
            }
        }

        if let Some(mut act) = world.get_mut::<PlayerActivity>(entity) {
            act.actor = actor;
        }
    }
}

// ---------------------------------------------------------------------------
// tick_activities
// ---------------------------------------------------------------------------

/// Advance every active `PlayerActivity` by one turn.
pub fn tick_activities(world: &mut World) {
    let active: Vec<Entity> = world
        .query_filtered::<Entity, With<PlayerActivity>>()
        .iter(world)
        .filter(|&e| {
            world
                .get::<PlayerActivity>(e)
                .map(|a| a.phase == ActivityPhase::Active)
                .unwrap_or(false)
        })
        .collect();

    for entity in active {
        tick_one(world, entity);

        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.new_turn(false);
        }
    }
}

/// Tick a single entity's activity: do_turn, then finish if complete.
pub fn tick_one(world: &mut World, entity: Entity) {
    // Take actor out so we can call do_turn with a free &mut World.
    let mut actor = {
        let Some(mut act) = world.get_mut::<PlayerActivity>(entity) else {
            return;
        };
        act.actor.take()
    };

    // do_turn — extract moves_left, pass it by &mut, write back.
    let mut moves_left = world
        .get::<PlayerActivity>(entity)
        .map(|a| a.moves_left)
        .unwrap_or(0);

    if let Some(ref mut a) = actor {
        a.do_turn(&mut moves_left, entity, world);
    }

    if let Some(mut act) = world.get_mut::<PlayerActivity>(entity) {
        act.moves_left = moves_left;
        act.actor = actor.take();
    }

    // Finish if complete.
    let is_complete = world
        .get::<PlayerActivity>(entity)
        .map(|a| a.is_complete())
        .unwrap_or(false);

    if is_complete {
        finish_activity(world, entity);
    }
}

/// Run `actor.finish()` and mark the activity as Done.
pub fn finish_activity(world: &mut World, entity: Entity) {
    let mut actor = {
        let Some(mut act) = world.get_mut::<PlayerActivity>(entity) else {
            return;
        };
        act.phase = ActivityPhase::Done;
        act.actor.take()
    };

    if let Some(ref mut a) = actor {
        a.finish(entity, world);
    }

    // Remove the component; Done activities are cleaned up immediately.
    world.entity_mut(entity).remove::<PlayerActivity>();
}

// ---------------------------------------------------------------------------
// cleanup_done_activities
// ---------------------------------------------------------------------------

/// Remove any `PlayerActivity` components stuck in phase `Done` (safety net).
pub fn cleanup_done_activities(
    mut commands: Commands,
    query: Query<Entity, With<PlayerActivity>>,
    world: &World,
) {
    for entity in &query {
        if let Some(act) = world.get::<PlayerActivity>(entity) {
            if act.phase == ActivityPhase::Done {
                commands.entity(entity).remove::<PlayerActivity>();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// cancel_activity
// ---------------------------------------------------------------------------

/// Cancel the active activity on `entity`, calling `actor.canceled()` first.
pub fn cancel_activity(entity: Entity, world: &mut World) {
    if world.get::<PlayerActivity>(entity).is_none() {
        return;
    }

    let mut actor = {
        let mut act = world.get_mut::<PlayerActivity>(entity).unwrap();
        act.phase = ActivityPhase::Done;
        act.actor.take()
    };

    if let Some(ref mut a) = actor {
        a.canceled(entity, world);
    }

    world.entity_mut(entity).remove::<PlayerActivity>();
}
