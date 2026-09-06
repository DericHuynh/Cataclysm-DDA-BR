//! The explicit-handle graph frontend — a data-driven construction API over
//! the same baked network the function-recording frontend produces.
//!
//! # Why a second frontend
//!
//! [`HtnDomain::from_root`](crate::domain::HtnDomain::from_root) uses task
//! functions as graph identity: `.then(f)` captures `TypeId::of::<F>()`. That
//! is perfect for hand-written Rust, but a **data-driven frontend** (a CDDA
//! JSON compiler, a scripting layer) cannot use it: two closures produced by
//! the same closure *expression* share one `TypeId` even when they capture
//! different values, so keying generated tasks by closure type would collapse
//! distinct data-defined tasks into one node. The fix is not "more closure
//! values" — it is **explicit graph handles**.
//!
//! [`GraphBuilder`] constructs exactly the same validated, baked, summarized
//! network through opaque [`TaskHandle`]s: handles identify graph nodes;
//! labels are authoring/diagnostics only. Nodes may be reserved before they
//! are defined (forward references and recursion fall out naturally), and two
//! specializations of the same native call get two distinct handles and bake
//! as two distinct tasks.
//!
//! ```
//! use cdda_htn::graph::GraphBuilder;
//! use cdda_htn::planner::HtnPlanner;
//! use cdda_htn::state::PlanState;
//! use bevy_ecs::prelude::*;
//!
//! #[derive(Component, Clone, Default, Debug)]
//! struct Hunger(u32);
//! #[derive(Component, Clone, Default, Debug)]
//! struct Food(pub u32);
//! #[derive(Component, Clone, Default, Debug)]
//! struct Steps(u32);
//!
//! let mut graph = GraphBuilder::new();
//!
//! let root = graph.reserve("survivor:survive");
//! let eat = graph.reserve("survivor:eat");
//! let wander = graph.reserve("survivor:wander");
//!
//! // Two primitives built by the SAME closure shape — distinct handles make
//! // them distinct tasks (a closure-type identity scheme would collapse
//! // everything generated from one expression into one node).
//! graph.define_primitive(eat, |p| {
//!     p.precondition(|h: &Hunger| h.0 >= 40)
//!         .effect(|f: &mut Food, h: &mut Hunger| {
//!             f.0 = f.0.saturating_sub(1);
//!             h.0 = 0;
//!         });
//! });
//! graph.define_primitive(wander, |p| {
//!     p.effect(|s: &mut Steps| s.0 += 1);
//! });
//!
//! graph.define_compound(root, |c| {
//!     // Reserve-then-define supports forward references and recursion.
//!     c.method()
//!         .precondition(|h: &Hunger| h.0 >= 40)
//!         .then(eat);
//!     c.method().then(wander);
//! });
//!
//! let baked = graph.build(root).unwrap();
//!
//! // The handle → baked-index mapping is the execution-table seam for
//! // data-driven executors.
//! let eat_idx = baked.index(eat).unwrap();
//!
//! let mut planner = HtnPlanner::new(baked.domain());
//! let state = PlanState::build(&baked.domain().components)
//!     .set(Hunger(50))
//!     .finish();
//! let plan = planner.plan(baked.root_index(), &state).unwrap();
//! assert_eq!(plan.steps(), &[eat_idx as u32]);
//! ```
//!
//! # Validation
//!
//! `build` rejects: reserved-but-never-defined nodes, duplicate definitions,
//! handles minted by a *different* builder, unknown subtask handles, and a
//! non-compound root. Everything else (mixed declarations, methodless
//! compounds, aliased effect slots, cyclic `before` constraints, ...) flows
//! through the same soft-collected `HtnError::Builder` reporting and the same
//! bake as the function frontend — there is one validation story, not two.
//!
//! Labels are interned ([`ustr::Ustr`]); callers may pass owned `String`s and
//! nothing is leaked on their behalf beyond the intern table every domain
//! already uses for display names.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use ustr::Ustr;

use crate::domain::{HtnDomain, SelectionPolicy};
use crate::error::{HtnError, HtnResult};
use crate::tasks::{
    Action, Effect, IntoEffect, IntoPrecondition, IntoUtility, MethodProto, Precondition, Recorder,
    ScoreFn, SubtaskHandle, SubtaskRef, TaskProto,
};

/// Monotonic builder-instance ids: a [`TaskHandle`] minted by one
/// [`GraphBuilder`] is rejected by another (no cross-builder handle reuse).
static NEXT_GRAPH_ID: AtomicU64 = AtomicU64::new(1);

/// An opaque handle to one node of a [`GraphBuilder`] under construction.
///
/// Handles — not labels, not closure types — are the graph identity of the
/// explicit-handle frontend. They are cheap to copy and hash, and they are
/// only valid for the builder that minted them (enforced at `define`/`build`
/// time).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskHandle {
    pub(crate) graph: u64,
    pub(crate) node: u32,
}

/// The result of [`GraphBuilder::build`]: the baked [`HtnDomain`] plus the
/// handle → baked-task-index mapping the execution layer needs.
///
/// Runtime plans address tasks by baked index (the same compiled step program
/// the function frontend produces); this mapping is how a data-driven
/// executor binds a plan step back to "which native call, with which
/// compiled arguments" and to the agent's root.
#[derive(Debug)]
pub struct BakedGraph {
    domain: HtnDomain,
    index_of: HashMap<TaskHandle, usize>,
    root: TaskHandle,
}

impl BakedGraph {
    /// The baked domain (planner-ready, immutable).
    pub fn domain(&self) -> &HtnDomain {
        &self.domain
    }

    /// Consume the mapping and return just the domain.
    pub fn into_domain(self) -> HtnDomain {
        self.domain
    }

    /// Resolve a handle to its index into
    /// [`HtnDomain::tasks`](crate::domain::HtnDomain::tasks) — the execution-
    /// table seam. Returns `None` for handles from another builder.
    pub fn index(&self, handle: TaskHandle) -> Option<usize> {
        self.index_of.get(&handle).copied()
    }

    /// The baked index of the root the graph was built from.
    pub fn root_index(&self) -> usize {
        self.domain.root
    }

    /// The root handle.
    pub fn root_handle(&self) -> TaskHandle {
        self.root
    }
}

/// A builder for one primitive task node, handed to the closure passed to
/// [`GraphBuilder::define_primitive`]. Same closure vocabulary as
/// [`TaskBuilder`](crate::tasks::TaskBuilder)'s primitive side.
pub struct PrimitiveBuilder<'a> {
    rec: &'a mut Recorder,
    task_index: usize,
    preconditions: Vec<Precondition>,
    effects: Vec<Effect>,
    expected_effects: Vec<Effect>,
    action: Option<Action>,
    cost: Option<ScoreFn>,
    static_cost: Option<f32>,
}

impl<'a> PrimitiveBuilder<'a> {
    /// Add a precondition (all must hold for the primitive to be pickable).
    pub fn precondition<P, Args>(&mut self, p: P) -> &mut Self
    where
        P: IntoPrecondition<Args>,
    {
        self.preconditions.push(p.build(&mut self.rec.registry));
        self
    }

    /// Add an effect — applied to the planning scratchpad during search **and**
    /// committed to the real entity by the stock driver (data-driven executors
    /// that own their execution semantics usually want
    /// [`Self::expected`] instead).
    pub fn effect<E, Args>(&mut self, e: E) -> &mut Self
    where
        E: IntoEffect<Args>,
    {
        self.effects.push(e.build(&mut self.rec.registry));
        self
    }

    /// Add a planning-only (non-guaranteed) effect — the "predict" half of a
    /// simulation-owned operator: applied to the scratchpad during search but
    /// never committed to the real world.
    pub fn expected<E, Args>(&mut self, e: E) -> &mut Self
    where
        E: IntoEffect<Args>,
    {
        self.expected_effects.push(e.build(&mut self.rec.registry));
        self
    }

    /// Set the real-world action dispatched by the stock driver at execution.
    pub fn action<F: Fn(&mut bevy_ecs::system::EntityCommands) + Send + Sync + 'static>(
        &mut self,
        f: F,
    ) -> &mut Self {
        self.action = Some(std::sync::Arc::new(f));
        self
    }

    /// Constant cost estimate (search guidance, not permission to act).
    pub fn cost(&mut self, c: f32) -> &mut Self {
        self.static_cost = Some(c.max(0.0));
        self.cost = Some(Box::new(move |_| c));
        self
    }

    /// Dynamic cost sampled from the scratchpad at plan time.
    pub fn cost_fn<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&crate::state::PlanState) -> f32 + Send + Sync + 'static,
    {
        self.static_cost = None;
        self.cost = Some(Box::new(f));
        self
    }

    /// Write the recorded proto into the node's placeholder slot.
    fn commit(self) {
        self.rec.tasks[self.task_index].2 = TaskProto::Primitive {
            preconditions: self.preconditions,
            effects: self.effects,
            expected_effects: self.expected_effects,
            action: self.action,
            cost: self.cost,
            static_cost: self.static_cost,
        };
    }
}

/// A builder for one compound task node, handed to the closure passed to
/// [`GraphBuilder::define_compound`].
pub struct CompoundBuilder<'a> {
    graph: u64,
    nodes: &'a HashMap<u32, usize>,
    labels: &'a HashMap<u32, &'static str>,
    rec: &'a mut Recorder,
    errors: Vec<String>,
    task_index: usize,
    methods: Vec<MethodProto>,
    policy: Option<SelectionPolicy>,
}

impl<'a> CompoundBuilder<'a> {
    /// Set this compound task's branch-selection policy.
    pub fn select(&mut self, policy: SelectionPolicy) -> &mut Self {
        self.policy = Some(policy);
        self
    }

    /// Begin a new decomposition branch (method). Branches are tried in
    /// declaration order; the first whose preconditions hold is chosen.
    pub fn method(&mut self) -> GraphMethodBuilder<'_> {
        self.methods.push(MethodProto::default());
        // Disjoint field borrows: the registry lives in `rec`, the branch
        // under construction in `methods`.
        let (rec, methods) = (&mut self.rec, &mut self.methods);
        let proto = methods.last_mut().expect("just pushed");
        GraphMethodBuilder {
            graph: self.graph,
            nodes: self.nodes,
            labels: self.labels,
            registry: &mut rec.registry,
            proto,
            errors: &mut self.errors,
        }
    }

    /// Write the recorded proto into the node's placeholder slot, merging any
    /// method-level errors into the recorder's error sink.
    fn commit(mut self) {
        if !self.errors.is_empty() {
            self.rec.errors.append(&mut self.errors);
            return; // a method-level error means the proto is not trustworthy
        }
        self.rec.tasks[self.task_index].2 = TaskProto::Compound {
            methods: self.methods,
            policy: self.policy.unwrap_or_default(),
        };
    }
}

/// Configure one branch inside [`CompoundBuilder::method`]. Same surface as
/// [`MethodBuilder`](crate::tasks::MethodBuilder), with [`TaskHandle`]s in
/// place of task functions.
pub struct GraphMethodBuilder<'a> {
    graph: u64,
    nodes: &'a HashMap<u32, usize>,
    labels: &'a HashMap<u32, &'static str>,
    registry: &'a mut crate::state::RegistryBuilder,
    proto: &'a mut MethodProto,
    errors: &'a mut Vec<String>,
}

impl<'a> GraphMethodBuilder<'a> {
    /// Validate a subtask handle minted by this builder; collect an error and
    /// return `None` otherwise.
    fn resolve(&mut self, handle: TaskHandle, what: &str) -> Option<(SubtaskRef, &'static str)> {
        if handle.graph != self.graph {
            self.errors.push(format!(
                "{what} subtask handle belongs to a different GraphBuilder"
            ));
            return None;
        }
        match (self.nodes.get(&handle.node), self.labels.get(&handle.node)) {
            (Some(_), Some(label)) => Some((SubtaskRef::Synthetic(handle.node), *label)),
            _ => {
                self.errors
                    .push(format!("{what} subtask handle is unknown to this GraphBuilder"));
                None
            }
        }
    }

    /// Name this branch (debugging, tracing, rankers). Accepts owned strings.
    pub fn named(&mut self, name: impl Into<String>) -> &mut Self {
        let owned: String = name.into();
        self.proto.name = Some(Ustr::from(owned.as_str()).as_str());
        self
    }

    /// Static utility score for utility-based branch selection.
    pub fn utility(&mut self, u: f32) -> &mut Self {
        self.proto.utility = Some(Box::new(move |_| u));
        self
    }

    /// Dynamic utility scored from components at branch-evaluation time.
    pub fn utility_fn<F, Args>(&mut self, f: F) -> &mut Self
    where
        F: IntoUtility<Args>,
    {
        self.proto.utility = Some(f.build(self.registry));
        self
    }

    /// Add a precondition for this branch to be chosen.
    pub fn precondition<P, Args>(&mut self, p: P) -> &mut Self
    where
        P: IntoPrecondition<Args>,
    {
        let p = p.build(self.registry);
        self.proto.preconditions.push(p);
        self
    }

    /// Append a subtask at the end of the current total order.
    #[track_caller]
    pub fn then(&mut self, handle: TaskHandle) -> &mut Self {
        if let Some((rref, label)) = self.resolve(handle, "then") {
            self.proto.subtasks.push((rref, label, true));
        }
        self
    }

    /// Add a subtask with no ordering commitment relative to other unordered
    /// members. Returns a handle for [`Self::before`] constraints.
    #[track_caller]
    pub fn subtask(&mut self, handle: TaskHandle) -> SubtaskHandle {
        let pos = self.proto.subtasks.len() as u32;
        if let Some((rref, label)) = self.resolve(handle, "subtask") {
            self.proto.subtasks.push((rref, label, false));
            self.proto.unordered = true;
        }
        SubtaskHandle { pos }
    }

    /// Require that `before` completes before `after` starts.
    pub fn before(&mut self, before: SubtaskHandle, after: SubtaskHandle) -> &mut Self {
        self.proto.edges.push((before.pos, after.pos));
        self
    }

    /// Mark a plan boundary (PausePlan) at the current member position. See
    /// [`MethodBuilder::pause_plan`](crate::tasks::MethodBuilder::pause_plan).
    pub fn pause_plan(&mut self) -> &mut Self {
        let pos = self.proto.subtasks.len() as u32;
        if self.proto.pause_positions.last() != Some(&pos) {
            self.proto.pause_positions.push(pos);
        }
        self
    }
}

/// The explicit-handle domain builder: reserve nodes, define them as
/// primitives or compounds, then [`build`](Self::build) from a root handle.
///
/// See the [module docs](crate::graph) for the full contract.
pub struct GraphBuilder {
    id: u64,
    rec: Recorder,
    /// node id → index into `rec.tasks`.
    nodes: HashMap<u32, usize>,
    /// node id → interned label.
    labels: HashMap<u32, &'static str>,
    defined: HashSet<u32>,
    next_node: u32,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    /// A fresh, empty graph builder.
    pub fn new() -> Self {
        Self {
            id: NEXT_GRAPH_ID.fetch_add(1, Ordering::Relaxed),
            rec: Recorder {
                registry: crate::state::RegistryBuilder::default(),
                tasks: Vec::new(),
                index_of: HashMap::new(),
                queue: Default::default(),
                errors: Vec::new(),
                next_synthetic: 0,
                shares: Vec::new(),
                insertables: Vec::new(),
                insertion: false,
                extra_roots: Vec::new(),
            },
            nodes: HashMap::new(),
            labels: HashMap::new(),
            defined: HashSet::new(),
            next_node: 0,
        }
    }

    /// Reserve a node and return its handle. Labels are diagnostics only;
    /// duplicates are permitted (identity is the handle), but every reserved
    /// node must be defined before [`Self::build`].
    pub fn reserve(&mut self, label: impl Into<String>) -> TaskHandle {
        let node = self.next_node;
        self.next_node += 1;
        // Synthetic-id discipline: graph nodes mint from the recorder's
        // synthetic counter so the GTN compilation (which continues from it
        // at bake time) can never collide with a data-defined node.
        self.rec.next_synthetic = node + 1;
        let owned: String = label.into();
        let label = Ustr::from(owned.as_str()).as_str();
        let idx = self.rec.tasks.len();
        self.rec
            .tasks
            .push((SubtaskRef::Synthetic(node), label, placeholder_proto()));
        self.rec.index_of.insert(SubtaskRef::Synthetic(node), idx);
        self.nodes.insert(node, idx);
        self.labels.insert(node, label);
        TaskHandle { graph: self.id, node }
    }

    /// The interned label of a handle minted by this builder.
    pub fn label(&self, handle: TaskHandle) -> Option<&'static str> {
        if handle.graph != self.id {
            return None;
        }
        self.labels.get(&handle.node).copied()
    }

    /// Define a reserved node as a primitive task.
    pub fn define_primitive(
        &mut self,
        handle: TaskHandle,
        f: impl FnOnce(&mut PrimitiveBuilder<'_>),
    ) {
        if !self.claim(handle, "primitive") {
            return;
        }
        let task_index = self.nodes[&handle.node];
        let mut builder = PrimitiveBuilder {
            rec: &mut self.rec,
            task_index,
            preconditions: Vec::new(),
            effects: Vec::new(),
            expected_effects: Vec::new(),
            action: None,
            cost: None,
            static_cost: None,
        };
        f(&mut builder);
        builder.commit();
    }

    /// Define a reserved node as a compound task.
    pub fn define_compound(
        &mut self,
        handle: TaskHandle,
        f: impl FnOnce(&mut CompoundBuilder<'_>),
    ) {
        if !self.claim(handle, "compound") {
            return;
        }
        let task_index = self.nodes[&handle.node];
        let mut builder = CompoundBuilder {
            graph: self.id,
            nodes: &self.nodes,
            labels: &self.labels,
            rec: &mut self.rec,
            errors: Vec::new(),
            task_index,
            methods: Vec::new(),
            policy: None,
        };
        f(&mut builder);
        builder.commit();
    }

    /// Claim a handle for definition: validates the builder and the
    /// defined-once rule; collects errors and returns `false` when the claim
    /// fails.
    fn claim(&mut self, handle: TaskHandle, kind: &str) -> bool {
        if handle.graph != self.id {
            self.rec.errors.push(format!(
                "`{kind}` definition handle belongs to a different GraphBuilder"
            ));
            return false;
        }
        if !self.nodes.contains_key(&handle.node) {
            self.rec.errors
                .push(format!("`{kind}` definition handle is unknown to this GraphBuilder"));
            return false;
        }
        if !self.defined.insert(handle.node) {
            let label = self.labels[&handle.node];
            self.rec.errors.push(format!(
                "task `{label}` is defined twice (each reserved node defines exactly once)"
            ));
            return false;
        }
        true
    }

    /// Validate the graph and bake it: returns the domain plus the
    /// handle → index mapping. The root must be a compound task defined in
    /// this builder; every reserved node must be defined.
    pub fn build(self, root: TaskHandle) -> HtnResult<BakedGraph> {
        if root.graph != self.id {
            return Err(HtnError::builder(
                "root handle belongs to a different GraphBuilder",
            ));
        }
        let Some(&root_idx) = self.nodes.get(&root.node) else {
            return Err(HtnError::builder(
                "root handle is unknown to this GraphBuilder",
            ));
        };

        // Every reserved node must be defined (the placeholder protos would
        // otherwise bake as methodless compounds).
        let undefined: Vec<String> = self
            .nodes
            .keys()
            .filter(|node| !self.defined.contains(node))
            .map(|node| {
                format!(
                    "task `{}` was reserved but never defined",
                    self.labels[node]
                )
            })
            .collect();
        if !undefined.is_empty() {
            return Err(HtnError::Builder { errors: undefined });
        }

        // The root check happens here (with the handle's label) as well as in
        // the shared bake (by position) — both report the same mistake.
        if matches!(self.rec.tasks[root_idx].2, TaskProto::Primitive { .. }) {
            return Err(HtnError::builder(format!(
                "root task `{}` must be a compound task",
                self.labels[&root.node]
            )));
        }

        let domain = crate::domain::finish_bake(self.rec, Vec::new(), root_idx)?;
        let index_of = self
            .nodes
            .iter()
            .map(|(&node, &idx)| (TaskHandle { graph: self.id, node }, idx))
            .collect();
        Ok(BakedGraph {
            domain,
            index_of,
            root,
        })
    }
}

/// The placeholder proto a reserve call registers; `define_*` overwrites it.
fn placeholder_proto() -> TaskProto {
    TaskProto::Compound {
        methods: Vec::new(),
        policy: SelectionPolicy::default(),
    }
}
