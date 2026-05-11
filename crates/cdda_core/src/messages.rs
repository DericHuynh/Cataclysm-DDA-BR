//! Common messages shared across crate boundaries.
//!
//! These are globally broadcast `Message` types (not observer-based `Event`s)
//! that any system can subscribe to via `MessageReader<T>`.

/// TurnAdvanced message — re-exported from `cdda_components::sim`.
pub use cdda_components::sim::TurnAdvanced;
