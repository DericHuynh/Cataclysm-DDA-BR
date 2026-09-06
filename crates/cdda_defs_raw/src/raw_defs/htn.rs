//! HTN planner definitions — data-authored decomposition networks.
//!
//! These raw defs let mods author **compound tasks** for the HTN planner as
//! JSON (`"type": "htn_compound"`). Data decides which behavior to compose;
//! the *meaning* of every predicate and operator lives in a Rust kernel
//! registry (`cdda_sim::ai::htn`), and the simulation decides whether a
//! planned action actually happens. The planner never executes gameplay.
//!
//! Shape (illustrative):
//!
//! ```json
//! {
//!   "type": "htn_compound",
//!   "id": "core:meet_needs",
//!   "methods": [
//!     {
//!       "id": "drink",
//!       "when": [ { "predicate": "cdda:thirsty", "args": { "minimum": 40 } } ],
//!       "steps": [
//!         { "task": "core:consume_carried", "args": { "item_category": "food" } }
//!       ]
//!     }
//!   ]
//!   }
//! ```
//!
//! Every `"predicate"` / `"operator"` reference must resolve to a registered
//! native kernel at compile time — unknown symbols are load/compile errors,
//! not silent no-ops.

use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A data-authored HTN compound task (`"type": "htn_compound"`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HtnCompoundDef {
    /// Unique identifier, e.g. `"core:meet_needs"`. Namespaced ids are the
    /// modding convention; overrides replace by id after mod layering.
    pub id: DefId<HtnCompoundDef>,

    /// Parameter names this compound accepts at call sites. A step referencing
    /// this compound may pass `"args": { "<param>": <value> }`; inside the
    /// compound body, `{"param": "<name>"}` markers are substituted with the
    /// bound value at compile time. Each distinct (task, arguments)
    /// specialization compiles as its OWN graph node.
    #[serde(default)]
    pub parameters: Vec<String>,

    /// Ordered decomposition methods (tried in declaration order).
    #[serde(default)]
    pub methods: Vec<HtnMethodDef>,

    /// Display/debug name.
    #[serde(default)]
    pub name: Option<String>,
}

/// One decomposition alternative of an [`HtnCompoundDef`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HtnMethodDef {
    /// Method name — the override/diagnostic handle. Overrides target named
    /// methods; explicit declaration order is preserved.
    #[serde(default)]
    pub id: Option<String>,

    /// Preconditions — all must hold for this method to be chosen. Each is a
    /// native-predicate reference; unknown predicate names are compile errors.
    #[serde(default)]
    pub when: Vec<HtnCallDef>,

    /// Subtask steps in declaration order (a pure `then` chain).
    #[serde(default)]
    pub steps: Vec<HtnStepDef>,
}

/// A native-predicate reference: which kernel to compile, with its raw
/// arguments. The kernel owns its argument schema — this layer stores the
/// JSON verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HtnCallDef {
    /// Registered kernel name, e.g. `"cdda:hungry"`.
    pub predicate: String,

    /// Raw kernel arguments (schema-validated by the kernel at compile time).
    #[serde(default)]
    pub args: Value,
}

/// One step of a method body: a native operator, a sub-task reference, or a
/// `bind` step (a later extension — parsed but uninterpreted for now).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HtnStepDef {
    /// Native operator kernel name, e.g. `"cdda:consume"`.
    #[serde(default)]
    pub operator: Option<String>,

    /// Referenced compound task by definition id.
    #[serde(default)]
    pub task: Option<String>,

    /// Named operator/task arguments (raw; kernels validate their own).
    #[serde(default)]
    pub args: Value,
}

impl HtnStepDef {
    /// Whether this step is a native operator call.
    pub fn is_operator(&self) -> bool {
        self.operator.is_some()
    }

    /// Whether this step references another compound task.
    pub fn is_task_ref(&self) -> bool {
        self.task.is_some()
    }
}
