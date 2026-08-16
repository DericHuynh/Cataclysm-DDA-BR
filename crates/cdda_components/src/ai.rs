//! AI planner components and shared AI vocabulary.
//!
//! Every AI-controlled entity carries a **planner marker** component that
//! selects which planning algorithm drives it.  Planners may range from a
//! trivial behaviour tree (dumb zombie) up to a full planner (GOAP for a
//! feral zombie, an HTN for a survivor or a "hunter" zombie).
//!
//! ## Dispatch model
//!
//! Bevy dispatches by component, so each planner type is a zero-sized marker
//! `Component` and the planner *systems* run `.run_if(With<PlannerX>)`.  A mob
//! is spawned with exactly one planner marker; which one it gets is a content
//! decision (derived from its def/`MonsterName`/variety), not a fixed global.
//!
//! All planners ultimately emit an [`ActionIntent`](crate::intent::ActionIntent)
//! into the shared AP-sorted `IntentQueue`, so monsters participate in the same
//! buffered, highest-AP-first resolution as the player — players have **no**
//! first-action guarantee, matching the fair turn ordering.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Component;
use cdda_core_types::core::coords::WorldPos;

// ---------------------------------------------------------------------------
// Planner markers
// ---------------------------------------------------------------------------

/// Behaviour tree planner — simplest, cheap, for predictable mobs (dumb
/// zombies, animals). Selection, sequencing, and utility decisions are data.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlannerBehaviourTree;

/// Goal-Oriented Action Planning motivator — picks a plan from world state +
/// available actions. Good for goal-driven mobs (feral zombies as an example)
/// where the best action depends on the current situation.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlannerGoap;

/// Hierarchical Task Network planner — decomposes high-level tasks into
/// ordered subtasks with preconditions/effects. Good for capable agents
/// (survivors, horde leaders) that must reason over multi-step plans.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlannerHtn;

/// Explicitly inert planner — the entity never plans an action. Used as a
/// fallback before a real planner is assigned, or for non-AI entities that we
/// want to be queryable as "has an AI slot" without acting.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlannerNone;

// ---------------------------------------------------------------------------
// Per-turn AI state
// ---------------------------------------------------------------------------

/// High-level objective an AI entity settled on this turn.
///
/// Produced by a planner (BT/GOAP/HTN) and translated by the AI systems into an
/// [`ActionIntent`] (or a path of intents) that enters the shared intent queue.
/// This decouples "what I want" (planner) from "how I act" (intent resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiGoal {
    /// Move one tile toward `target`; attack when adjacent.
    Attack { target: Entity },
    /// Move in a random open direction.
    Wander,
    /// Move away from a threat.
    Flee { from: Entity },
    /// Stay within `radius` tiles of `position`.
    Guard { position: WorldPos, radius: u32 },
    /// Pathfind toward `target`, attacking when in range.
    Hunt { target: Entity },
    /// Hold position and do nothing this turn.
    Idle,
    /// Take a recovery or utility action (reload, use item, interact).
    Interact,
}

// ---------------------------------------------------------------------------
// Planners registry (content-facing)
// ---------------------------------------------------------------------------

/// Stable string identity for a planner, so content (a monster def's
/// `"ai_planner"` field) can name a planner without each being a distinct code
/// path at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlannerKind {
    BehaviourTree,
    Goap,
    Htn,
    #[default]
    None_,
}

impl PlannerKind {
    /// Parse a planner kind from a string (a def's `"ai_planner"` value).
    /// Case-insensitive; unknown values yield `None_` (inert).
    pub fn parse(s: &str) -> PlannerKind {
        match s.trim().to_ascii_uppercase().as_str() {
            "BT" | "BEHAVIOUR" | "BEHAVIOR" | "BEHAVIOUR_TREE" | "BEHAVIOR_TREE" => {
                PlannerKind::BehaviourTree
            }
            "GOAP" | "PLANNER" => PlannerKind::Goap,
            "HTN" => PlannerKind::Htn,
            _ => PlannerKind::None_,
        }
    }

    /// The stable display name for this planner kind.
    pub fn name(self) -> &'static str {
        match self {
            PlannerKind::BehaviourTree => "behaviour_tree",
            PlannerKind::Goap => "goap",
            PlannerKind::Htn => "htn",
            PlannerKind::None_ => "none",
        }
    }
}
