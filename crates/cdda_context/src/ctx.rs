//! Screen state machine — which screen is currently active.
//!
//! Re-exported from `crate::state` so that all crates share
//! one canonical definition without circular dependencies.

pub use crate::state::{ContextStack, Ctx};
