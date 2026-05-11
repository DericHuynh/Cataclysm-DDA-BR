// Re-export the pure types from cdda_core_types (coords, units, ids, damage, error, flags, raw_defs).
// Using `pub use cdda_core_types::core::*;` so that submodules like `crate::core::coords` resolve.
pub use cdda_core_types::core::*;

// ECS components — only in cdda_core (uses bevy_ecs).
pub mod components;
pub mod stats;
