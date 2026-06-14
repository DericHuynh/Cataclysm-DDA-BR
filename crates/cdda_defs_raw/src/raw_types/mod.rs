//! Shared types used across all definitions.
//!
//! These are the building blocks that def structs reference.

mod localized;
pub mod copy_from;

// DefId now lives in crate::core::id — re-export for backward compat in raw defs
pub use cdda_core_types::core::id::DefId;
pub use localized::LocalizedString;
pub use copy_from::{CopyFromChain, CopyFromOp, CopyFromTarget};
