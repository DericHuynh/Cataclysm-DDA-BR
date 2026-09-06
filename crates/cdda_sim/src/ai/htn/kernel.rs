//! Kernel plumbing shared by the CDDA HTN integration: the compile context,
//! the error shape, the kernel traits, the registry, and the sink that lets a
//! compiled predicate attach to either a method or a primitive.
//!
//! **Data decides which behavior to compose; Rust defines what each operation
//! means.** A kernel is the Rust meaning: a typed compiler from raw JSON
//! arguments to (a) BHTN preconditions / predicted effects and (b) an
//! execution hook that turns simulated state into a simulation request.

use std::collections::HashMap;
use std::sync::Arc;

use cdda_htn::graph::{GraphMethodBuilder, PrimitiveBuilder};
use cdda_htn::state::{PlanComponent, PlanState};
use cdda_components::intent::ActionIntent;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Compile diagnostics
// ---------------------------------------------------------------------------

/// One compile-time error, located at the definition / method / step that
/// produced it. Unknown kernel symbols, unknown definition references, and
/// kernel-schema violations are all compile errors — never silent no-ops.
#[derive(Debug, Clone)]
pub struct CompileError {
    /// The `htn_compound` id being compiled.
    pub def: String,
    /// The method id, when known.
    pub method: Option<String>,
    /// The step position, when known.
    pub step: Option<usize>,
    /// The specific failure.
    pub message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "in HTN def `{}`", self.def)?;
        if let Some(m) = &self.method {
            write!(f, " method `{m}`")?;
        }
        if let Some(s) = self.step {
            write!(f, " step {s}")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl CompileError {
    /// A def-level error.
    pub fn at_def(def: &str, message: impl Into<String>) -> Self {
        Self {
            def: def.to_string(),
            method: None,
            step: None,
            message: message.into(),
        }
    }

    /// A method-level error.
    pub fn at_method(def: &str, method: &str, message: impl Into<String>) -> Self {
        Self {
            def: def.to_string(),
            method: Some(method.to_string()),
            step: None,
            message: message.into(),
        }
    }

    /// A step-level error.
    pub fn at_step(
        def: &str,
        method: &str,
        step: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            def: def.to_string(),
            method: Some(method.to_string()),
            step: Some(step),
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Compile context — static reference validation
// ---------------------------------------------------------------------------

/// What a kernel can validate against at compile time: static definition
/// references (item ids, item categories). Resolution happens at content
/// compilation — a wrong-category or unknown-id reference is a load error.
#[derive(Clone, Copy)]
pub struct CompileCtx<'a> {
    /// Does an item definition with this id exist?
    pub item_exists: &'a dyn Fn(&str) -> bool,
    /// Does an item category with this id exist?
    pub category_exists: &'a dyn Fn(&str) -> bool,
}

impl<'a> CompileCtx<'a> {
    /// Validate an `"item"` argument reference.
    pub fn require_item(
        &self,
        id: &str,
        def: &str,
        method: &str,
        step: usize,
    ) -> Result<(), CompileError> {
        if (self.item_exists)(id) {
            Ok(())
        } else {
            Err(CompileError::at_step(
                def,
                method,
                step,
                format!("unknown item definition `{id}`"),
            ))
        }
    }

    /// Validate an `"item_category"` argument reference.
    pub fn require_category(
        &self,
        id: &str,
        def: &str,
        method: &str,
        step: usize,
    ) -> Result<(), CompileError> {
        if (self.category_exists)(id) {
            Ok(())
        } else {
            Err(CompileError::at_step(
                def,
                method,
                step,
                format!("unknown item category `{id}`"),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// PredSink — one compiled predicate, attachable to a method or a primitive
// ---------------------------------------------------------------------------

/// Where a compiled predicate is being attached: a compound's method (a
/// `when` clause) or a primitive task (an operator precondition). The kernel
/// supplies typed component closures either way; the sink routes them to the
/// right BHTN builder.
pub enum PredSink<'s, 'g> {
    /// A method's precondition list.
    Method(&'s mut GraphMethodBuilder<'g>),
    /// A primitive's precondition list.
    Primitive(&'s mut PrimitiveBuilder<'g>),
}

impl PredSink<'_, '_> {
    /// Add one typed precondition closure: `|model: &Model| -> bool`.
    pub fn add<T, F>(&mut self, f: F)
    where
        T: PlanComponent,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        match self {
            PredSink::Method(m) => {
                m.precondition(f);
            }
            PredSink::Primitive(p) => {
                p.precondition(f);
            }
        }
    }

    /// Add a two-model precondition closure:
    /// `|a: &ModelA, b: &ModelB| -> bool` (e.g. navigation + nearby).
    pub fn add2<A, B, F>(&mut self, f: F)
    where
        A: PlanComponent,
        B: PlanComponent,
        F: Fn(&A, &B) -> bool + Send + Sync + 'static,
    {
        match self {
            PredSink::Method(m) => {
                m.precondition(f);
            }
            PredSink::Primitive(p) => {
                p.precondition(f);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Kernel specs, traits, registry
// ---------------------------------------------------------------------------

/// The compiled form of one predicate call: a closure that attaches typed
/// precondition closures to whatever sink it is given.
pub struct PredicateSpec {
    pub attach: Box<dyn Fn(&mut PredSink<'_, '_>) + Send + Sync>,
}

/// The compiled form of one operator call: how the primitive node is defined
/// (preconditions, **predicted** effects, cost estimate) plus how the executor
/// prepares it (bind concrete targets from simulated state into a simulation
/// request). Prediction is planning-only — the executor never writes predicted
/// gameplay state back to the world; it observes what the simulation did.
pub struct OperatorSpec {
    /// Define the primitive node in the BHTN graph.
    pub define: Box<dyn Fn(&mut PrimitiveBuilder<'_>) + Send + Sync>,
    /// Bind concrete targets from the (freshly observed) planning state into
    /// a simulation request. `None` = this step cannot bind right now — the
    /// executor replans instead of forcing it.
    pub submit: Arc<dyn Fn(&PlanState) -> Option<ActionIntent> + Send + Sync>,
}

/// A typed predicate compiler: validates its own argument schema, resolves
/// static references through the compile context, and emits a
/// [`PredicateSpec`].
pub type PredicateCompiler = Arc<
    dyn Fn(&Value, &CompileCtx<'_>) -> Result<PredicateSpec, CompileError> + Send + Sync,
>;

/// A typed operator compiler (see [`OperatorSpec`]).
pub type OperatorCompiler =
    Arc<dyn Fn(&Value, &CompileCtx<'_>) -> Result<OperatorSpec, CompileError> + Send + Sync>;

/// The native-call registry: `kernels.predicate("cdda:hungry", compile_hungry)
/// .operator("cdda:consume", compile_consume)`. Names are kernel symbols —
/// JSON references them, unknown names are compile errors.
#[derive(Default, Clone)]
pub struct KernelRegistry {
    predicates: HashMap<String, PredicateCompiler>,
    operators: HashMap<String, OperatorCompiler>,
}

impl KernelRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a predicate kernel compiler.
    pub fn predicate<F>(&mut self, name: &str, compiler: F) -> &mut Self
    where
        F: Fn(&Value, &CompileCtx<'_>) -> Result<PredicateSpec, CompileError>
            + Send
            + Sync
            + 'static,
    {
        self.predicates.insert(name.to_string(), Arc::new(compiler));
        self
    }

    /// Register an operator kernel compiler.
    pub fn operator<F>(&mut self, name: &str, compiler: F) -> &mut Self
    where
        F: Fn(&Value, &CompileCtx<'_>) -> Result<OperatorSpec, CompileError>
            + Send
            + Sync
            + 'static,
    {
        self.operators.insert(name.to_string(), Arc::new(compiler));
        self
    }

    /// Resolve a predicate kernel by name.
    pub fn predicate_kernel(&self, name: &str) -> Option<&PredicateCompiler> {
        self.predicates.get(name)
    }

    /// Resolve an operator kernel by name.
    pub fn operator_kernel(&self, name: &str) -> Option<&OperatorCompiler> {
        self.operators.get(name)
    }

    /// Whether any kernel is registered under `name` (either kind).
    pub fn contains(&self, name: &str) -> bool {
        self.predicates.contains_key(name) || self.operators.contains_key(name)
    }
}

// ---------------------------------------------------------------------------
// Argument-schema helpers shared by the stock kernels
// ---------------------------------------------------------------------------

/// Read a required integer field from a kernel's args object.
pub fn require_i32(
    args: &Value,
    key: &str,
    def: &str,
    method: &str,
    step: usize,
) -> Result<i32, CompileError> {
    args.get(key)
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| {
            CompileError::at_step(def, method, step, format!("args need integer `{key}`"))
        })
}

/// Read an optional string field (present but not a string is an error).
pub fn opt_str<'v>(
    args: &'v Value,
    key: &str,
    def: &str,
    method: &str,
    step: usize,
) -> Result<Option<&'v str>, CompileError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(other) => Err(CompileError::at_step(
            def,
            method,
            step,
            format!("args field `{key}` must be a string, got {other}"),
        )),
    }
}
