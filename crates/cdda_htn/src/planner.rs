//! Forward HTN planner with MTR (Method Traversal Record) backtracking.
//!
//! Starting from the domain's root task, the planner repeatedly decomposes the
//! top of a task stack. Compound tasks pick the first method whose preconditions
//! hold (at or after a per-decomposition `skip`), push their subtasks, and
//! record the method index into the MTR. Primitive tasks whose preconditions
//! hold are appended to the plan and their (expected) effects are applied to a
//! working copy of the state. When a task can't be satisfied, the planner
//! backtracks to the most recent decomposition, tries the next method, and
//! restores the plan/MTR.
//!
//! # Performance
//!
//! The hot loop works on **`usize` task indices**, never names. The working
//! stack and the backtracking frames hold `usize` indices into
//! [`HtnDomain::tasks`]. Domains intern task names as [`Ustr`] keys in a
//! precomputed `name -> index` map, so subtask resolution is O(1), not a linear
//! scan.
//!
//! `plan` and `mtr` are **append-only**, so backtracking frames store only the
//! two **lengths** (not cloned `Vec`s) — on backtrack a `truncate` restores the
//! exact prefix, which is provably identical to restoring a full clone but
//! avoids ~2 heap allocations + O(n) copies per recursion level. This is the
//! dominant win for domains that recursively decompose a root toward the sanity
//! limit (e.g. the miner benchmark). Task names are only materialized as
//! [`Ustr`]s when the final [`Plan`] is constructed.

use std::collections::VecDeque;

use bevy_reflect::TypeRegistry;
use ustr::Ustr;

use crate::domain::HtnDomain;
use crate::tasks::Task;
use crate::HtnState;

/// The method traversal record of a completed plan: the index of the chosen
/// method at each decomposition level. Used to compare plans by priority
/// (lower index = higher priority) and for debugging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mtr(pub Vec<usize>);

impl std::fmt::Display for Mtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

/// A completed forward plan: an ordered list of primitive task names (interned
/// [`Ustr`]s, so it's cheap to copy, compare, and hand around) plus the MTR
/// describing how each compound was decomposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Primitive task names, in execution order.
    pub tasks: Vec<Ustr>,
    /// Method indices chosen at each decomposition level.
    pub mtr: Mtr,
}

impl Plan {
    /// The ordered primitive task names (interned handles; deref to `&str`).
    pub fn task_names(&self) -> &[Ustr] {
        &self.tasks
    }

    /// The MTR for this plan.
    pub fn mtr(&self) -> &Mtr {
        &self.mtr
    }

    /// Order two plans by MTR priority (lower first).
    pub fn is_preferred_over(&self, other: &Self) -> bool {
        for (a, b) in self.mtr.0.iter().zip(other.mtr.0.iter()) {
            match a.cmp(b) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord == std::cmp::Ordering::Less,
            }
        }
        // Prefer the shorter MTR when they share a prefix.
        self.mtr.0.len() < other.mtr.0.len()
    }
}

/// Planner state used during one decomposition site for backtracking.
///
/// Stores **task indices** (not names) so frames stay tiny and copy-free.
///
/// # Backtracking via lengths
///
/// [`HtnPlanner`] builds `plan` and `mtr` **append-only** during a monotonic
/// recursive descent (primitives and method indices are only ever `push`ed).
/// So instead of deep-cloning both `Vec`s into every frame (which costs ~2n
/// allocations + O(n) copies per recursion level — catastrophic when a domain
/// recursively decomposes its root toward the sanity limit), we snapshot just
/// the two lengths. On backtrack we `truncate` back to those lengths, which is
/// provably identical to restoring a clone because the prefix of an append-only
/// Vec never changes. `skip_next` alone traces the search branch, so this stays
/// a fully correct DFS MTR backtrack.
#[derive(Debug)]
struct DecompositionFrame {
    /// The compound task index being decomposed.
    task: usize,
    /// `plan.len()` before this decomposition's subtasks were entered.
    plan_len: usize,
    /// The number of methods to skip (index+1 of the one just tried).
    skip_next: usize,
    /// `mtr.len()` before adding this decomposition's method index.
    mtr_len: usize,
}

/// A forward planner over a parsed [`HtnDomain`].
///
/// Planning mutates no external state: it clones the initial state and works on
/// its own copy, so it can be called repeatedly and cheaply across turns.
pub struct HtnPlanner<'a> {
    domain: &'a HtnDomain,
    registry: &'a TypeRegistry,
}

impl<'a> HtnPlanner<'a> {
    /// Create a planner bound to a domain and a type registry (for reflection
    /// evaluation / effect application).
    pub fn new(domain: &'a HtnDomain, registry: &'a TypeRegistry) -> Self {
        Self { domain, registry }
    }

    /// The domain this planner reads.
    pub fn domain(&self) -> &'a HtnDomain {
        self.domain
    }

    /// Decompose `root` into a [`Plan`]. Even if no task satisfies, the search
    /// terminates after exhausting backtracking and returns the best partial
    /// plan found (with an empty task list if nothing was decomposable).
    pub fn plan<S: HtnState>(&mut self, root: &str, initial_state: &S) -> Plan {
        let sanity_limit = 100;
        let mut count = 0;
        let mut stack: VecDeque<usize> = VecDeque::with_capacity(16);
        let mut decomp_stack: Vec<DecompositionFrame> = Vec::with_capacity(8);
        let mut mtr: Vec<usize> = Vec::with_capacity(8);
        let mut plan: Vec<usize> = Vec::with_capacity(8);
        let mut skip = 0;
        let mut state = initial_state.clone();

        let tasks = &self.domain.tasks;

        let root = Ustr::from(root);
        let Some(&root_idx) = self.domain.index_of.get(&root) else {
            return Plan {
                tasks: Vec::new(),
                mtr: Mtr(Vec::new()),
            };
        };
        stack.push_back(root_idx);

        let registry = self.registry;

        while let Some(current) = stack.pop_front() {
            count += 1;
            if count > sanity_limit {
                return Plan {
                    tasks: materialize_names(tasks, &plan),
                    mtr: Mtr(mtr),
                };
            }

            let task = &tasks[current];

            match task {
                Task::Compound(compound) => {
                    let eligible = compound.find_method(state.as_reflect(), skip);
                    if let Some((method, idx)) = eligible {
                        mtr.push(idx);
                        let frame = DecompositionFrame {
                            task: current,
                            plan_len: plan.len(),
                            skip_next: idx + 1,
                            // Snapshot *after* the push so restoring truncates
                            // back to a world that includes this method choice.
                            mtr_len: mtr.len(),
                        };
                        decomp_stack.push(frame);
                        // Push subtask indices in reverse so the first pops first.
                        for sub in method.subtasks.iter().rev() {
                            if let Some(sub_idx) = self.domain.task_index(*sub) {
                                stack.push_front(sub_idx);
                            }
                        }
                        skip = 0;
                        continue;
                    } else if let Some(frame) = decomp_stack.pop() {
                        plan.truncate(frame.plan_len);
                        mtr.truncate(frame.mtr_len);
                        skip = frame.skip_next;
                        stack.push_front(frame.task);
                        continue;
                    } else {
                        break;
                    }
                }
                Task::Primitive(primitive) => {
                    if primitive.preconditions_met(state.as_reflect()) {
                        plan.push(current);
                        for e in primitive.effects.iter() {
                            e.apply_dyn(state.as_reflect_mut(), registry);
                        }
                        for e in primitive.expected_effects.iter() {
                            e.apply_dyn(state.as_reflect_mut(), registry);
                        }
                        skip = 0;
                        continue;
                    } else if let Some(frame) = decomp_stack.pop() {
                        plan.truncate(frame.plan_len);
                        mtr.truncate(frame.mtr_len);
                        skip = frame.skip_next;
                        stack.push_front(frame.task);
                        continue;
                    } else {
                        break;
                    }
                }
                Task::Goal(_) => {
                    break;
                }
            }
        }

        Plan {
            tasks: materialize_names(tasks, &plan),
            mtr: Mtr(mtr),
        }
    }
}

/// Convert a plan of task indices into interned task-name [`Ustr`]s in the same
/// order.
fn materialize_names(tasks: &[Task], plan: &[usize]) -> Vec<Ustr> {
    plan.iter().map(|&i| tasks[i].name().into()).collect()
}
