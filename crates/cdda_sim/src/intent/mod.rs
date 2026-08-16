//! # Intent system — action ordering by priority
//!
//! Entities declare what they want to do each turn by inserting an
//! `ActionIntent` component.  Intents are collected, sorted by action
//! points, and resolved with precondition validation so that later
//! actions respect the results of earlier ones.
//!
//! Both player and AI use the same pipeline.
//!
//! ## Flow
//!
//! ```ignore
//! SimSet::IntentDeclare → collect_intents → IntentQueue (sorted by AP)
//! SimSet::IntentResolve → resolve_intents → execute + deduct AP
//! SimSet::Activity      → tick multi-turn activities
//! ```

pub mod plugin;
pub mod systems;
