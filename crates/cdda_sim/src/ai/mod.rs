//! # cdda_ai — AI module
//!
//! AI phase — entities decide actions.  Decision making is split into pluggable
//! **planners** selected by a marker component on each mob:
//!
//! - [`PlannerBehaviourTree`](cdda_components::ai::PlannerBehaviourTree) — cheap
//!   fixed-rule decisions (dumb zombies, animals).
//! - [`PlannerGoap`](cdda_components::ai::PlannerGoap) — goal-oriented action
//!   planning (feral zombies).
//! - [`PlannerHtn`](cdda_components::ai::PlannerHtn) — hierarchical task networks
//!   (survivors, high-level mobs).
//! - [`PlannerNone`](cdda_components::ai::PlannerNone) — inert, never acts.
//!
//! Every planner emits an [`ActionIntent`](cdda_components::intent::ActionIntent)
//! into the shared AP-sorted `IntentQueue`, so monsters and the player are
//! resolved together by highest action points — there is **no player-first
//! guarantee** (see the `intent` module and the `higher_ap_monster...` test).

pub mod plugin;
pub mod systems;
