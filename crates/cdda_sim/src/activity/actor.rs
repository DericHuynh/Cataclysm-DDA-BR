//! Activity actor types — legacy compatibility shim.
//!
//! In the pre-refactor system, each activity variant had its own "actor" struct
//! implementing a common trait.  Those types have been migrated to dedicated
//! ECS components in [`super::components`].
//!
//! This module is kept for backward-compatible re-exports during the migration.
//! New code should use the `Crafting`, `Aiming`, etc. components directly.

// No public API — the component types live in `components.rs`.
