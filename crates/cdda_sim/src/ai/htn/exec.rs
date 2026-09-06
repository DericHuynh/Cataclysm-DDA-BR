//! The CDDA execution adapter: a planner-core harness with a CDDA-owned plan
//! cursor and the request/result simulation contract.
//!
//! The stock `htn_ai_system` dispatches commands, commits effects, and
//! advances immediately — right for its own execution contract, wrong for
//! CDDA's deferred intent/activity contract. Submitting an intent is NOT
//! completing a plan step: the resolver can reject it, refuse an unsupported
//! operation, or the request can sit pending. So this adapter:
//!
//! - submits at most one request per actor per round (inserting
//!   `ActionIntent` for the intent pipeline to collect and resolve);
//! - advances the cursor ONLY on a `Completed` outcome matching the request
//!   id the simulation stamped for that request;
//! - drops the plan on `Rejected`/`Failed`/`Cancelled` (replan from reality
//!   next round);
//! - NEVER writes predicted gameplay state to the world — it re-observes;
//! - treats planner costs as estimates only (no AP gating — the resolver
//!   permits starting at nonnegative AP and entering debt);
//! - replans from a fresh observation every time it plans.
//!
//! Coroutine-free by construction: the per-agent tick is a pure function of
//! (brain, agent state, correlated outcome, fresh observation).

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use cdda_components::actor::IsAlive;
use cdda_components::def::IsDef;
use cdda_components::intent::{ActionOutcome, ActionOutcomeState, ActionRequestId};
use cdda_htn::planner::{HtnPlanner, Plan};
use cdda_htn::state::PlanState;
use std::sync::Arc;

use super::compile::CompiledHtnDomain;
use super::model::{InventoryModel, NavigationModel, NearbyModel, NeedsModel};
use super::observe::{observe_actor, ActorObservation, ItemCatalog};

/// Which HTN root this agent runs and how far it can see. The planner
/// algorithm comes from the `PlannerHtn` marker; the brain is the *behavior*:
/// a root `htn_compound` def id (e.g. `"core:meet_needs"`).
#[derive(Component, Debug, Clone)]
pub struct HtnBrain {
    /// Root `htn_compound` definition id.
    pub root: String,
    /// Observation radius in Manhattan tiles.
    pub view_radius: i32,
}

impl Default for HtnBrain {
    fn default() -> Self {
        Self {
            root: String::new(),
            view_radius: 12,
        }
    }
}

/// Per-agent execution state: the compiled plan, the cursor into its step
/// program, and the request correlation bookkeeping.
#[derive(Component, Debug, Clone, Default)]
pub struct HtnAgentState {
    /// The compiled step program (domain task indices).
    pub plan: Option<Plan>,
    /// Cursor into `plan.steps()`.
    pub cursor: usize,
    /// The request id last observed on the actor (stamped by the collector).
    pub last_seen: Option<u64>,
    /// The request id whose outcome was last processed.
    pub processed: Option<u64>,
}

/// The shared, immutable runtime: one compiled domain + execution table
/// serving every HTN agent. Published as a validated generation on reload —
/// replacing this resource swaps domain, selector catalog, root mapping, and
/// execution table together.
#[derive(Resource, Clone)]
pub struct HtnRuntime {
    pub compiled: Arc<CompiledHtnDomain>,
    /// Definition facts (DefOrigin → id, id → category) captured at the same
    /// moment as the domain: one validated generation, domain + catalog
    /// together.
    pub catalog: Arc<ItemCatalog>,
}

impl HtnRuntime {
    /// Wrap a compiled domain and its catalog.
    pub fn new(compiled: CompiledHtnDomain, catalog: ItemCatalog) -> Self {
        Self {
            compiled: Arc::new(compiled),
            catalog: Arc::new(catalog),
        }
    }

    /// The baked domain.
    pub fn domain(&self) -> &cdda_htn::domain::HtnDomain {
        self.compiled.domain()
    }
}

/// The exclusive driver for `PlannerHtn` agents. Registered in
/// `SimSet::IntentDeclare` BEFORE the intent collector: a submitted intent is
/// collected, stamped with its request id, and resolved later the same round.
pub fn drive_htn_system(world: &mut World) {
    let Some(runtime) = world.get_resource::<HtnRuntime>() else {
        return;
    };
    let runtime = runtime.clone();

    // Budget scheduler: only the selected acting agent runs this action pass.
    // Without the resource (direct test calls) every agent is processed.
    let acting = world
        .get_resource::<cdda_components::schedule::ActingEntity>()
        .map(|a| a.0);

    let mut agents: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<cdda_components::ai::PlannerHtn>,
            With<HtnBrain>,
            With<IsAlive>,
            Without<IsDef>,
        )>()
        .iter(world)
        .collect();
    if let Some(acting) = acting {
        agents.retain(|&entity| entity == acting);
    }

    for entity in agents.drain(..) {
        // An externally submitted command owns this pass. Discard the old plan
        // so its cursor cannot advance on that command's terminal outcome.
        if world
            .get::<cdda_components::intent::ActionIntent>(entity)
            .is_some()
        {
            world.entity_mut(entity).remove::<HtnAgentState>();
            continue;
        }
        let Some(brain) = world.get::<HtnBrain>(entity).cloned() else {
            continue;
        };
        let mut state = world
            .get::<HtnAgentState>(entity)
            .cloned()
            .unwrap_or_default();

        // ── Correlated outcome processing ────────────────────────────────
        // A fresh request id means the collector took our submission this
        // round; a terminal outcome for THAT id is the simulation's verdict.
        if let Some(req) = world.get::<ActionRequestId>(entity) {
            let req = req.0;
            if state.last_seen != Some(req) {
                state.last_seen = Some(req);
            }
        }
        if let Some(outcome) = world.get::<ActionOutcome>(entity) {
            if state.processed != Some(outcome.request.0)
                && state.last_seen == Some(outcome.request.0)
            {
                state.processed = Some(outcome.request.0);
                match outcome.state {
                    ActionOutcomeState::Completed => {
                        state.cursor += 1;
                        let done = state
                            .plan
                            .as_ref()
                            .map(|p| state.cursor >= p.len())
                            .unwrap_or(true);
                        if done {
                            state.plan = None; // plan complete
                        }
                    }
                    ActionOutcomeState::Rejected
                    | ActionOutcomeState::Failed
                    | ActionOutcomeState::Cancelled => {
                        // Rejection/failure → observation refresh + replanning.
                        state.plan = None;
                    }
                }
                world.entity_mut(entity).insert(state.clone());
                continue; // one state transition per round
            }
        }

        // ── Plan (when planless) ─────────────────────────────────────────
        if state.plan.is_none() {
            match plan_from_reality(world, entity, &brain, &runtime) {
                Some(plan) if plan.len() > 0 => {
                    state.cursor = 0;
                    state.plan = Some(plan);
                }
                // Empty decomposition (already satisfied) or NoPlan: stay
                // planless and retry next round — never wedge, never pretend.
                _ => {
                    world.entity_mut(entity).insert(state.clone());
                    continue;
                }
            }
        }

        // ── Submit the current step's request ────────────────────────────
        let Some(plan) = state.plan.clone() else {
            continue;
        };
        let steps = plan.steps();
        if state.cursor >= steps.len() {
            state.plan = None;
            world.entity_mut(entity).insert(state.clone());
            continue;
        }
        let step = steps[state.cursor] as usize;
        let Some(exec) = runtime.compiled.exec_table.get(&step) else {
            // Plan steps are primitives; a step without an execution entry
            // means the compiled domain changed underneath us — replan.
            state.plan = None;
            world.entity_mut(entity).insert(state.clone());
            continue;
        };

        // Bind from a FRESH observation (reality, not prediction).
        let obs = observe_actor(entity, world, brain.view_radius, &runtime.catalog);
        let scratch = plan_state_for(&obs, runtime.domain());
        match (exec.submit)(&scratch) {
            Some(intent) => {
                world.entity_mut(entity).insert(intent);
                // The collector stamps ActionRequestId + removes the intent;
                // the outcome comes back correlated. Nothing else to do.
            }
            None => {
                // Cannot bind this step from reality (target gone, already
                // adjacent, ...) — drop the plan and replan next round.
                state.plan = None;
                world.entity_mut(entity).insert(state.clone());
            }
        }
    }
}

/// Plan the brain's root against a fresh observation of the actor.
fn plan_from_reality(
    world: &mut World,
    entity: Entity,
    brain: &HtnBrain,
    runtime: &HtnRuntime,
) -> Option<Plan> {
    let Some(&root) = runtime.compiled.roots.get(&brain.root) else {
        return None; // unknown root def id — nothing this brain can run
    };
    let obs = observe_actor(entity, world, brain.view_radius, &runtime.catalog);
    let scratch = plan_state_for(&obs, runtime.domain());
    let mut planner = HtnPlanner::new(runtime.domain());
    planner.plan(root, &scratch).ok()
}

/// Build the planning scratchpad from an observation. Only models the
/// compiled domain actually registered are set (a domain whose kernels never
/// mention `NeedsModel` has no slot for it — setting it would be unsound).
pub fn plan_state_for(obs: &ActorObservation, domain: &cdda_htn::domain::HtnDomain) -> PlanState {
    let registry = &domain.components;
    let mut builder = PlanState::build(registry);
    if registry.slot_of::<NeedsModel>().is_some() {
        builder = builder.set(obs.needs.clone());
    }
    if registry.slot_of::<InventoryModel>().is_some() {
        builder = builder.set(obs.inventory.clone());
    }
    if registry.slot_of::<NearbyModel>().is_some() {
        builder = builder.set(obs.nearby.clone());
    }
    if registry.slot_of::<NavigationModel>().is_some() {
        builder = builder.set(obs.navigation.clone());
    }
    builder.finish()
}
