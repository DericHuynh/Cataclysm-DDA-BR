//! The HTN domain compiler: data-authored `htn_compound` definitions +
//! a native-kernel registry → a baked BHTN domain and an execution table.
//!
//! Compilation rules:
//!
//! - **Handles are identity.** Every compound def reserves a graph node; every
//!   (task reference, arguments) specialization reserves ANOTHER node, so
//!   differently parameterized native calls never collapse (the closure-type
//!   identity trap from function-based recording does not apply here).
//! - **Forward references and recursion** are supported: a node is reserved
//!   before its body is defined, so a compound may reference itself or a task
//!   defined later. Recursive *execution* is bounded the way the planner
//!   always bounds it — preconditions that eventually fail, search budgets,
//!   and termination summaries — not by prohibiting data-defined graphs.
//! - **Unknown symbols are compile errors** with def/method/step locations:
//!   unknown kernel names, unknown item/category references, unknown task ids,
//!   unknown parameter markers.
//! - **Compilation limit**: the node budget bounds runaway specialization
//!   (each unique argument set is its own node).

use std::collections::HashMap;
use std::sync::Arc;

use cdda_catalog::htn::HtnSource;
use cdda_catalog::htn::{Compound as HtnCompoundDef, Step as HtnStepDef};
use cdda_components::intent::ActionIntent;
use cdda_htn::graph::{BakedGraph, GraphBuilder, TaskHandle};
use cdda_htn::state::PlanState;
use serde_json::Value;

use super::kernel::{CompileCtx, CompileError, KernelRegistry, PredSink};

/// A compiled operator's execution entry: how the executor turns simulated
/// state into a simulation request for this exact baked step.
#[derive(Clone)]
pub struct OperatorExec {
    /// The kernel that produced this entry (diagnostics).
    pub kernel: String,
    /// Bind concrete targets from the planning state into a request.
    pub submit: Arc<dyn Fn(&PlanState) -> Option<ActionIntent> + Send + Sync>,
}

/// The compiled, planner-ready HTN domain.
pub struct CompiledHtnDomain {
    /// The baked graph (domain + handle → baked index mapping).
    pub graph: BakedGraph,
    /// Baked primitive index → execution entry. Only operators appear —
    /// plans are primitive step programs, compounds never execute.
    pub exec_table: HashMap<usize, OperatorExec>,
    /// Root def id → baked index (the agent brain's root seam).
    pub roots: HashMap<String, usize>,
}

impl CompiledHtnDomain {
    /// The baked domain.
    pub fn domain(&self) -> &cdda_htn::domain::HtnDomain {
        self.graph.domain()
    }

    /// The baked index of a root def id, if that def was compiled.
    pub fn root_index(&self, def_id: &str) -> Option<usize> {
        self.roots.get(def_id).copied()
    }
}

/// Upper bound on compiled graph nodes (specialization runaway guard).
pub const MAX_COMPILED_NODES: usize = 1024;

/// Compile every `htn_compound` in the registry into a planner-ready domain.
/// All defs are compiled (roots or not) so a mod's overrides are validated;
/// `roots` maps every def id to its baked index.
pub fn compile_domain(
    source: &impl HtnSource,
    kernels: &KernelRegistry,
) -> Result<CompiledHtnDomain, Vec<CompileError>> {
    let registry = source.htn_program();
    let item_exists = |id: &str| registry.items.contains(id);
    let category_exists = |id: &str| registry.item_categories.contains(id);
    let ctx = CompileCtx {
        item_exists: &item_exists,
        category_exists: &category_exists,
    };
    let mut c = Compiler {
        graph: GraphBuilder::new(),
        handles: HashMap::new(),
        exec: Vec::new(),
        errors: Vec::new(),
        kernels,
        ctx: &ctx,
        defs: &registry.htn_compounds,
        node_budget: MAX_COMPILED_NODES,
    };

    // Compile every definition (deterministic order by id for stable
    // graphs). Parameterized defs are templates: they compile only through
    // their (task, arguments) specializations — an unbound body has nothing
    // to substitute, like a CDDA abstract that never spawns.
    let mut ids: Vec<&str> = registry.htn_compounds.keys().map(|k| k.as_str()).collect();
    ids.sort();
    for id in ids {
        let is_template = registry
            .htn_compounds
            .get(&def_id_from(id))
            .map(|d| !d.parameters.is_empty())
            .unwrap_or(false);
        if !is_template {
            let _ = c.ensure_def(id, None, None);
        }
    }

    if !c.errors.is_empty() {
        return Err(c.errors);
    }

    // The nominal root is the first compiled def (alphabetically); agents
    // address roots through `roots`, never through the domain root field.
    let nominal_root = c.handles.values().copied().next().ok_or_else(|| {
        vec![CompileError::at_def(
            "<none>",
            "no htn_compound definitions",
        )]
    })?;

    let graph = c
        .graph
        .build(nominal_root)
        .map_err(|e| vec![CompileError::at_def("<bake>", e.to_string())])?;

    let mut exec_table = HashMap::new();
    let mut roots = HashMap::new();
    for (handle, exec) in c.exec {
        if let Some(idx) = graph.index(handle) {
            exec_table.insert(idx, exec);
        }
    }
    for (id, handle) in c.handles {
        if let Some(idx) = graph.index(handle) {
            roots.insert(id, idx);
        }
    }

    Ok(CompiledHtnDomain {
        graph,
        exec_table,
        roots,
    })
}

struct Compiler<'a> {
    graph: GraphBuilder,
    /// specialization key → graph handle (reserved before defined).
    handles: HashMap<String, TaskHandle>,
    /// (handle, exec entry) pairs, resolved to indices after bake.
    exec: Vec<(TaskHandle, OperatorExec)>,
    errors: Vec<CompileError>,
    kernels: &'a KernelRegistry,
    ctx: &'a CompileCtx<'a>,
    defs: &'a HashMap<String, Arc<HtnCompoundDef>>,
    node_budget: usize,
}

impl<'a> Compiler<'a> {
    /// Ensure a (possibly specialized) node exists for `def_id`; returns its
    /// handle. The node is reserved (and cached) BEFORE its body is defined,
    /// so forward references and recursion become plain graph edges.
    ///
    /// `spec_args` (the substituted call-site arguments) key the
    /// specialization; `binding` (parameter name → bound value) is what the
    /// body's `{"param": …}` markers resolve against.
    fn ensure_def(
        &mut self,
        def_id: &str,
        spec_args: Option<&Value>,
        binding: Option<Value>,
    ) -> Option<TaskHandle> {
        let spec_key = match spec_args {
            Some(v) if !v.is_null() => canonical_args(def_id, v),
            _ => def_id.to_string(),
        };
        if let Some(&h) = self.handles.get(&spec_key) {
            return Some(h);
        }

        let def = match self.defs.get(&def_id_from(def_id)) {
            Some(d) => d.clone(),
            None => {
                self.errors.push(CompileError::at_def(
                    def_id,
                    format!("unknown htn_compound task reference `{def_id}`"),
                ));
                return None;
            }
        };

        if self.node_budget == 0 {
            self.errors.push(CompileError::at_def(
                def_id,
                format!(
                    "compilation exceeded the node budget of {MAX_COMPILED_NODES} — \
                     too many distinct specializations"
                ),
            ));
            return None;
        }
        self.node_budget -= 1;

        // Reserve under the spec key FIRST (recursion-safe), then define.
        let label = match spec_args {
            Some(v) if !v.is_null() && !as_object(v).is_some_and(|o| o.is_empty()) => {
                format!("{def_id}[{v}]")
            }
            _ => def_id.to_string(),
        };
        let handle = self.graph.reserve(label);
        self.handles.insert(spec_key, handle);

        self.define_body(def_id, &def, handle, binding.as_ref());
        Some(handle)
    }

    /// Define a reserved node's methods from the definition body.
    fn define_body(
        &mut self,
        def_id: &str,
        def: &Arc<HtnCompoundDef>,
        handle: TaskHandle,
        binding: Option<&Value>,
    ) {
        // Phase A — reserve every step's child node (may recurse into
        // `ensure_def`; the parent node is already cached, so cycles are
        // edges, not loops). Errors here are recorded; the step is skipped.
        let mut reserved: Vec<Vec<Option<TaskHandle>>> = Vec::with_capacity(def.methods.len());
        for method in &def.methods {
            let method_name = method.id.clone().unwrap_or_default();
            let mut nodes = Vec::with_capacity(method.steps.len());
            for (step_idx, step) in method.steps.iter().enumerate() {
                nodes.push(self.materialize_step(def_id, &method_name, step_idx, step, binding));
            }
            reserved.push(nodes);
        }

        // Phase B — define the compound body over the reserved handles. The
        // compiler's fields are split-borrowed: the graph mutably, the rest
        // immutably.
        let Compiler {
            graph,
            kernels,
            ctx,
            errors,
            ..
        } = self;
        let kernels = *kernels;
        let ctx = **ctx;
        graph.define_compound(handle, |c| {
            for (method_idx, method) in def.methods.iter().enumerate() {
                let method_name = method.id.clone().unwrap_or_else(|| format!("{method_idx}"));
                let mut mb = c.method();
                mb.named(method_name.clone());

                // `when` clauses: compile each predicate call and attach.
                for call in &method.when {
                    let args = substitute(&call.args, binding);
                    match kernels.predicate_kernel(&call.predicate) {
                        None => errors.push(CompileError::at_step(
                            def_id,
                            &method_name,
                            method_idx,
                            format!("unknown predicate kernel `{}`", call.predicate),
                        )),
                        Some(compile) => match compile(&args, &ctx) {
                            Ok(spec) => {
                                let mut sink = PredSink::Method(&mut mb);
                                (spec.attach)(&mut sink);
                            }
                            Err(e) => errors.push(relocate(e, def_id, &method_name, method_idx)),
                        },
                    }
                }

                // Steps: reference the nodes reserved in phase A.
                for (step_idx, node) in reserved[method_idx].iter().enumerate() {
                    match node {
                        Some(node) => {
                            mb.then(*node);
                        }
                        None => {
                            let _ = step_idx; // error already recorded
                        }
                    }
                }
            }
        });
    }

    /// Compile one step into a node handle (operator) or a task-ref handle.
    fn materialize_step(
        &mut self,
        def_id: &str,
        method_name: &str,
        step_idx: usize,
        step: &HtnStepDef,
        binding: Option<&Value>,
    ) -> Option<TaskHandle> {
        match (&step.operator, &step.task) {
            (Some(op_name), _) => {
                let args = substitute(&step.args, binding);
                let Some(compile) = self.kernels.operator_kernel(op_name) else {
                    self.errors.push(CompileError::at_step(
                        def_id,
                        method_name,
                        step_idx,
                        format!("unknown operator kernel `{op_name}`"),
                    ));
                    return None;
                };
                let spec = match compile(&args, self.ctx) {
                    Ok(s) => s,
                    Err(e) => {
                        self.errors.push(relocate(e, def_id, method_name, step_idx));
                        return None;
                    }
                };
                let handle = self
                    .graph
                    .reserve(format!("{op_name}@{def_id}/{method_name}/{step_idx}"));
                let exec = OperatorExec {
                    kernel: op_name.clone(),
                    submit: spec.submit.clone(),
                };
                self.exec.push((handle, exec));
                let define = spec.define;
                self.graph.define_primitive(handle, |p| {
                    define(p);
                });
                Some(handle)
            }
            (None, Some(task_id)) => {
                let raw = if step.args.is_null() {
                    None
                } else {
                    Some(&step.args)
                };
                let Some(target) = self.defs.get(&def_id_from(task_id)).cloned() else {
                    self.errors.push(CompileError::at_def(
                        def_id,
                        format!("unknown htn_compound task reference `{task_id}`"),
                    ));
                    return None;
                };
                // Validate the RAW call-site args against the target's
                // declared parameters. A `{"param": "name"}` marker passes the
                // caller's binding through (lexical scope), enabling recursive
                // references to the same specialization.
                let in_scope: Vec<String> = binding
                    .as_ref()
                    .and_then(|b| b.as_object())
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default();
                if let Some(raw) = raw {
                    validate_call_site(
                        &target,
                        raw,
                        task_id,
                        def_id,
                        method_name,
                        step_idx,
                        &in_scope,
                        &mut self.errors,
                    );
                }
                // The child's parameter binding: explicit keys substitute
                // against the caller's binding; a pass-through marker binds
                // the named in-scope value.
                let mut child_binding = serde_json::Map::new();
                for param in &target.parameters {
                    if let Some(raw_args) = raw {
                        if let Some(v) = raw_args.get(param.as_str()) {
                            child_binding.insert(param.clone(), substitute(v, binding));
                            continue;
                        }
                    }
                    if let Some(Value::String(n)) = raw.and_then(|r| r.get("param")) {
                        if let Some(b) = binding.as_ref().and_then(|b| b.get(n.as_str())) {
                            child_binding.insert(param.clone(), b.clone());
                        }
                    }
                }
                let child_binding = if child_binding.is_empty() {
                    None
                } else {
                    Some(Value::Object(child_binding))
                };
                let spec_args = raw.map(|r| substitute(r, binding)).filter(|v| !v.is_null());
                self.ensure_def(task_id, spec_args.as_ref(), child_binding)
            }
            (None, None) => {
                self.errors.push(CompileError::at_step(
                    def_id,
                    method_name,
                    step_idx,
                    "step declares neither `operator` nor `task`",
                ));
                None
            }
        }
    }
}

/// Pin a kernel error to the referencing def/method/step if the kernel did
/// not already set a location.
fn relocate(e: CompileError, def_id: &str, method_name: &str, step_idx: usize) -> CompileError {
    if e.def == "<kernel>" || e.method.is_none() {
        CompileError {
            def: def_id.to_string(),
            method: Some(method_name.to_string()),
            step: Some(step_idx),
            message: e.message,
        }
    } else {
        e
    }
}

/// Canonical specialization key for a (task, args) call site.
fn canonical_args(def_id: &str, args: &Value) -> String {
    // Sorted-key canonical JSON so {a:1,b:2} == {b:2,a:1}.
    let canon = canonicalize(args);
    format!("{def_id}#{canon}")
}

fn canonicalize(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|k| (k.clone(), canonicalize(&map[k])))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

fn as_object(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    v.as_object()
}

/// Validate RAW call-site arguments against the target def's declared
/// parameters. Every key must be a declared parameter name, except the
/// `{"param": "<name>"}` marker whose value must name a parameter in the
/// CALLER's scope (the recursion pass-through).
fn validate_call_site(
    target: &HtnCompoundDef,
    raw: &Value,
    task_id: &str,
    def_id: &str,
    method_name: &str,
    step_idx: usize,
    in_scope: &[String],
    errors: &mut Vec<CompileError>,
) {
    let Some(map) = raw.as_object() else {
        errors.push(CompileError::at_step(
            def_id,
            method_name,
            step_idx,
            format!("call-site args for `{task_id}` must be an object, got {raw}"),
        ));
        return;
    };
    for (key, value) in map {
        if key == "param" {
            match value.as_str() {
                Some(name) if in_scope.iter().any(|p| p == name) => {}
                Some(name) => errors.push(CompileError::at_step(
                    def_id,
                    method_name,
                    step_idx,
                    format!(
                        "`param` marker `{name}` is not a parameter in scope (in scope: {in_scope:?})"
                    ),
                )),
                None => errors.push(CompileError::at_step(
                    def_id,
                    method_name,
                    step_idx,
                    "`param` marker must be a string parameter name",
                )),
            }
        } else if !target.parameters.iter().any(|p| p == key) {
            errors.push(CompileError::at_step(
                def_id,
                method_name,
                step_idx,
                format!(
                    "call-site arg `{key}` is not declared by `{task_id}` (declared: {:?})",
                    target.parameters
                ),
            ));
        }
    }
}

/// Recursively substitute `{"param": "name"}` markers with the bound value.
/// Unbound markers stay verbatim and surface as kernel-schema errors with
/// the referencing site's location.
fn substitute(args: &Value, binding: Option<&Value>) -> Value {
    let binding = binding.cloned().unwrap_or(Value::Null);
    sub(args, &binding)
}

fn sub(v: &Value, binding: &Value) -> Value {
    match v {
        Value::Object(map) => {
            // A bare marker object is replaced by the bound value wholesale.
            if map.len() == 1 {
                if let Some(Value::String(name)) = map.get("param") {
                    if let Some(bound) = binding.get(name) {
                        return bound.clone();
                    }
                }
            }
            // A mixed object merges the bound value into the remaining keys
            // (`{"scope": "nearby", "param": "target"}` + bound
            // `{"item_category": "tools"}` → `{"scope": "nearby",
            // "item_category": "tools"}`; bound keys win on conflict).
            if let Some(Value::String(name)) = map.get("param") {
                if let Some(Value::Object(bound)) = binding.get(name) {
                    let mut out: serde_json::Map<String, Value> = map
                        .iter()
                        .filter(|(k, _)| k.as_str() != "param")
                        .map(|(k, val)| (k.clone(), sub(val, binding)))
                        .collect();
                    for (k, val) in bound {
                        out.insert(k.clone(), val.clone());
                    }
                    return Value::Object(out);
                }
            }
            Value::Object(
                map.iter()
                    .map(|(k, val)| (k.clone(), sub(val, binding)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(|i| sub(i, binding)).collect()),
        other => other.clone(),
    }
}

/// Parse a def id string into the registry key type.
fn def_id_from(id: &str) -> String {
    id.to_string()
}
