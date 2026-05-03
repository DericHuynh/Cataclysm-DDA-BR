//! Shared types used across all definitions.
//!
//! These are the building blocks that def structs reference.

mod id;
mod localized;
pub mod copy_from;

pub use id::DefId;
pub use localized::LocalizedString;
pub use copy_from::{CopyFromChain, CopyFromOp, CopyFromTarget};
