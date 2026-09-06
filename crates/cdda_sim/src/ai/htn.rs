//! # `cdda_sim::ai::htn` — mod-authored HTN behavior on the `cdda_htn` core
//!
//! The revised integration boundary (see the design review this implements):
//!
//! | Owner | Responsibility |
//! |---|---|
//! | `cdda_defs_raw` | Serializable `htn_compound` definitions. |
//! | `cdda_data` | Mod layering, inheritance, definition references. |
//! | **this module** | Rust kernel registry, JSON→graph compilation, actor-scoped observations, the CDDA execution adapter. |
//! | `cdda_htn` | Graph construction, scratch-state search, rollback, plans. |
//! | Simulation subsystems | Authoritative validation, AP charging, relationship mutations, outcomes. |
//!
//! **Data decides which behavior to compose. Rust defines what each operation
//! means. The simulation decides whether it actually happens.** The planner
//! owns none of the simulation's execution: it submits correlated
//! `ActionIntent`s through the request/result contract and advances its plan
//! only on matching `ActionOutcome`s ([`exec`](self::exec)).
//!
//! Quick shape:
//!
//! ```ignore
//! let mut kernels = KernelRegistry::new();
//! kernels.predicate("cdda:hungry", compile_hungry)
//!        .operator("cdda:consume", compile_consume);
//! let compiled = compile_domain(&def_registry, &kernels)?;
//! let runtime = HtnRuntime::new(compiled);           // one validated generation
//! // drive_htn_system(&mut world) ticks every PlannerHtn agent
//! ```

pub mod compile;
pub mod exec;
pub mod kernel;
pub mod kernels;
pub mod model;
pub mod observe;

pub use compile::{compile_domain, CompiledHtnDomain, OperatorExec, MAX_COMPILED_NODES};
pub use exec::{drive_htn_system, HtnAgentState, HtnBrain, HtnRuntime};
pub use kernel::{
    CompileCtx, CompileError, KernelRegistry, OperatorSpec, PredSink, PredicateSpec,
};
pub use model::{
    InventoryModel, NavigationModel, NearbyModel, Needs, NeedsModel, ObservedItem,
};
pub use observe::{observe_actor, ActorObservation, ItemCatalog};
