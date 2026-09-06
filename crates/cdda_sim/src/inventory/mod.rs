//! # cdda_inventory — Inventory system
//!
//! Manages item stacks, inventory letters (invlets), binned lookups,
//! and item movement between containers and inventories.
//!
//! Extracted from `cdda_core::inventory` to avoid circular dependencies.

pub mod examine_resource;
pub mod pocket;
pub mod systems;
pub mod transfer;
