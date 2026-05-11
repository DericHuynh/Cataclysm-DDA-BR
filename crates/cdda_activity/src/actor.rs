//! Activity actors — concrete implementations of multi-turn activities.
//!
//! Mirrors the C++ `activity_actor` class hierarchy from `activity_actor.cpp`.
//! Each variant implements `start()`, `do_turn()`, and `finish()` logic.
//! Uses an enum instead of trait objects for ECS-friendly storage.
//!
//! ## Calling convention
//!
//! The actor methods receive individual mutable fields extracted from
//! `PlayerActivity` rather than `&mut PlayerActivity`. This lets callers hold
//! `&mut World` at the same time without aliasing the component.

use bevy_ecs::prelude::*;

use crate::tracker::{ActivityTracker, BRISK_EXERCISE, LIGHT_EXERCISE, NO_EXERCISE};
use crate::CRAFT_COMPLETE_HOOK;
use cdda_actor::turn::AP_COST_CRAFT_TICK;
use cdda_components::actor::ActionPoints;
use cdda_components::item::InProgressCraft;

// ---------------------------------------------------------------------------
// ActivityActor — enum-based dispatch
// ---------------------------------------------------------------------------

/// All concrete activity actor types.
#[derive(Debug, Clone)]
pub enum ActivityActor {
    Idle(IdleActor),
    Aim(AimActor),
    Read(ReadActor),
    Reload(ReloadActor),
    Craft(CraftActor),
    Wait(WaitActor),
    Interact(InteractActor),
}

impl ActivityActor {
    /// Returns the `activity_type` ID string for this actor.
    pub fn activity_type_id(&self) -> &str {
        match self {
            Self::Idle(_) => "ACT_IDLE",
            Self::Aim(_) => "ACT_AIM",
            Self::Read(_) => "ACT_READ",
            Self::Reload(_) => "ACT_RELOAD",
            Self::Craft(_) => "ACT_CRAFT",
            Self::Wait(_) => "ACT_WAIT",
            Self::Interact(_) => "ACT_INTERACT",
        }
    }

    /// Called once when the activity begins. Sets `moves_total` and `moves_left`.
    pub fn start(
        &mut self,
        moves_total: &mut i32,
        moves_left: &mut i32,
        entity: Entity,
        world: &mut World,
    ) {
        match self {
            Self::Idle(a) => a.start(moves_total, moves_left, entity, world),
            Self::Aim(a) => a.start(moves_total, moves_left, entity, world),
            Self::Read(a) => a.start(moves_total, moves_left, entity, world),
            Self::Reload(a) => a.start(moves_total, moves_left, entity, world),
            Self::Craft(a) => a.start(moves_total, moves_left, entity, world),
            Self::Wait(a) => a.start(moves_total, moves_left, entity, world),
            Self::Interact(a) => a.start(moves_total, moves_left, entity, world),
        }
    }

    /// Called each turn. Decrements `moves_left`; set to 0 to signal completion.
    pub fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        match self {
            Self::Idle(a) => a.do_turn(moves_left, entity, world),
            Self::Aim(a) => a.do_turn(moves_left, entity, world),
            Self::Read(a) => a.do_turn(moves_left, entity, world),
            Self::Reload(a) => a.do_turn(moves_left, entity, world),
            Self::Craft(a) => a.do_turn(moves_left, entity, world),
            Self::Wait(a) => a.do_turn(moves_left, entity, world),
            Self::Interact(a) => a.do_turn(moves_left, entity, world),
        }
    }

    /// Called when `moves_left` reaches 0. Resolves the activity outcome.
    pub fn finish(&mut self, entity: Entity, world: &mut World) {
        match self {
            Self::Idle(a) => a.finish(entity, world),
            Self::Aim(a) => a.finish(entity, world),
            Self::Read(a) => a.finish(entity, world),
            Self::Reload(a) => a.finish(entity, world),
            Self::Craft(a) => a.finish(entity, world),
            Self::Wait(a) => a.finish(entity, world),
            Self::Interact(a) => a.finish(entity, world),
        }
    }

    /// Called just before the activity is cancelled (before the component is removed).
    pub fn canceled(&mut self, entity: Entity, world: &mut World) {
        match self {
            Self::Idle(a) => a.canceled(entity, world),
            Self::Aim(a) => a.canceled(entity, world),
            Self::Read(a) => a.canceled(entity, world),
            Self::Reload(a) => a.canceled(entity, world),
            Self::Craft(a) => a.canceled(entity, world),
            Self::Wait(a) => a.canceled(entity, world),
            Self::Interact(a) => a.canceled(entity, world),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper trait
// ---------------------------------------------------------------------------

pub trait ActorImpl: std::fmt::Debug + Clone {
    fn start(
        &mut self,
        moves_total: &mut i32,
        moves_left: &mut i32,
        entity: Entity,
        world: &mut World,
    );
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World);
    fn finish(&mut self, entity: Entity, world: &mut World);
    fn canceled(&mut self, _entity: Entity, _world: &mut World) {}
}

// ---------------------------------------------------------------------------
// IdleActor — ACT_IDLE
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct IdleActor {
    pub duration: i32,
}

impl ActorImpl for IdleActor {
    fn start(&mut self, moves_total: &mut i32, moves_left: &mut i32, _entity: Entity, _world: &mut World) {
        *moves_total = self.duration * 100;
        *moves_left = *moves_total;
    }
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(NO_EXERCISE);
        }
        *moves_left -= 100;
    }
    fn finish(&mut self, _entity: Entity, _world: &mut World) {}
}

// ---------------------------------------------------------------------------
// WaitActor — ACT_WAIT
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct WaitActor {
    pub turns: i32,
}

impl ActorImpl for WaitActor {
    fn start(&mut self, moves_total: &mut i32, moves_left: &mut i32, _entity: Entity, _world: &mut World) {
        *moves_total = self.turns * 100;
        *moves_left = *moves_total;
    }
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(NO_EXERCISE);
        }
        *moves_left -= 100;
    }
    fn finish(&mut self, _entity: Entity, _world: &mut World) {}
}

// ---------------------------------------------------------------------------
// AimActor — ACT_AIM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AimActor {
    pub target_aim_percent: u32,
    pub cur_aim: u32,
    pub fire_on_aim: bool,
    pub weapon_id: String,
}

impl ActorImpl for AimActor {
    fn start(&mut self, moves_total: &mut i32, moves_left: &mut i32, _entity: Entity, _world: &mut World) {
        // Aim is NEITHER-based; do_turn drives it.
        *moves_total = -1;
        *moves_left = 1;
    }
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(LIGHT_EXERCISE);
        }
        self.cur_aim = (self.cur_aim + 5).min(self.target_aim_percent);
        if self.cur_aim >= self.target_aim_percent {
            *moves_left = 0;
        }
    }
    fn finish(&mut self, _entity: Entity, _world: &mut World) {
        // Firing resolved by combat system on Done.
    }
}

// ---------------------------------------------------------------------------
// ReadActor — ACT_READ
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ReadActor {
    pub book_id: String,
    pub skill_id: String,
    pub turns_read: i32,
    pub turns_total: i32,
}

impl ActorImpl for ReadActor {
    fn start(&mut self, moves_total: &mut i32, moves_left: &mut i32, _entity: Entity, _world: &mut World) {
        *moves_total = self.turns_total * 100;
        *moves_left = *moves_total;
    }
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(NO_EXERCISE);
        }
        self.turns_read += 1;
        *moves_left -= 100;
    }
    fn finish(&mut self, _entity: Entity, _world: &mut World) {
        // Skill/morale gains applied by downstream systems on Done.
    }
}

// ---------------------------------------------------------------------------
// ReloadActor — ACT_RELOAD
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ReloadActor {
    pub item_id: String,
    pub ammo_id: String,
    pub quantity: i32,
    pub speed_factor: f32,
}

impl ActorImpl for ReloadActor {
    fn start(&mut self, moves_total: &mut i32, moves_left: &mut i32, _entity: Entity, _world: &mut World) {
        let base = (self.quantity as f32 * 100.0 / self.speed_factor.max(0.01)) as i32;
        *moves_total = base;
        *moves_left = base;
    }
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(LIGHT_EXERCISE);
        }
        *moves_left -= 100;
    }
    fn finish(&mut self, _entity: Entity, _world: &mut World) {
        // Ammo transfer resolved by item systems on Done.
    }
}

// ---------------------------------------------------------------------------
// CraftActor — ACT_CRAFT
// ---------------------------------------------------------------------------

/// Craft activity: drives an `InProgressCraft` entity to completion.
///
/// The `craft_entity` holds the `InProgressCraft` component in the player's
/// inventory. Each `do_turn` tick spends `AP_COST_CRAFT_TICK` from the
/// character's `ActionPoints` and advances `InProgressCraft::ap_spent`.
/// When `ap_spent >= ap_total`, `finish` calls the registered craft completion
/// hook to spawn the result item.
#[derive(Debug, Clone)]
pub struct CraftActor {
    /// The `InProgressCraft` entity in the player's inventory.
    pub craft_entity: Entity,
}

impl ActorImpl for CraftActor {
    fn start(
        &mut self,
        moves_total: &mut i32,
        moves_left: &mut i32,
        _entity: Entity,
        world: &mut World,
    ) {
        let ap_total = world
            .get::<InProgressCraft>(self.craft_entity)
            .map(|c| c.ap_total)
            .unwrap_or(100);
        *moves_total = ap_total;
        *moves_left = ap_total;
    }

    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        // Guard: craft entity may have been despawned externally.
        if world.get::<InProgressCraft>(self.craft_entity).is_none() {
            *moves_left = 0;
            return;
        }

        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(BRISK_EXERCISE);
        }

        if let Some(mut ap) = world.get_mut::<ActionPoints>(entity) {
            ap.spend(AP_COST_CRAFT_TICK);
        }

        let is_done = {
            let Some(mut craft) = world.get_mut::<InProgressCraft>(self.craft_entity) else {
                *moves_left = 0;
                return;
            };
            craft.ap_spent += AP_COST_CRAFT_TICK;
            craft.is_complete()
        };

        *moves_left -= AP_COST_CRAFT_TICK;
        if is_done {
            *moves_left = 0;
        }
    }

    fn finish(&mut self, entity: Entity, world: &mut World) {
        // Craft completion is handled by registering a callback via CRAFT_COMPLETE_HOOK.
        // This avoids a circular dependency between cdda_activity and cdda_crafting.
        if let Some(complete_craft) = CRAFT_COMPLETE_HOOK.get() {
            complete_craft(world, entity, self.craft_entity);
        }
    }

    fn canceled(&mut self, _entity: Entity, _world: &mut World) {
        // The InProgressCraft entity stays in the inventory so the craft can be resumed.
    }
}

// ---------------------------------------------------------------------------
// InteractActor — ACT_INTERACT
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct InteractActor {
    pub description: String,
    pub duration: i32,
}

impl ActorImpl for InteractActor {
    fn start(&mut self, moves_total: &mut i32, moves_left: &mut i32, _entity: Entity, _world: &mut World) {
        *moves_total = self.duration * 100;
        *moves_left = *moves_total;
    }
    fn do_turn(&mut self, moves_left: &mut i32, entity: Entity, world: &mut World) {
        if let Some(mut tracker) = world.get_mut::<ActivityTracker>(entity) {
            tracker.log_activity(LIGHT_EXERCISE);
        }
        *moves_left -= 100;
    }
    fn finish(&mut self, _entity: Entity, _world: &mut World) {}
}
