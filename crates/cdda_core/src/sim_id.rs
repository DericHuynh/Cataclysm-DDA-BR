//! Deterministic simulation entity ID.
//!
//! `SimId` is assigned at spawn time from `(world_seed, spawn_counter)`.
//! It is stable across replays because it encodes *why* an entity was
//! spawned, not *when* in Bevy's internal allocation order.

use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

/// Deterministic simulation entity identifier.
///
/// Smaller `SimId` = spawned earlier in the current session.
/// Sorting by `SimId` produces a stable, reproducible order.
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SimId(pub u64);

impl SimId {
    /// Generate a deterministic `SimId` from `(world_seed, spawn_counter)`.
    /// Uses a simple splitmix64 variant — fast, deterministic, no dependency.
    pub fn next(world_seed: u64, counter: u64) -> Self {
        let mut x = world_seed.wrapping_add(counter);
        x = x.wrapping_add(0x9e3779b97f4a7c15);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        SimId(x ^ (x >> 31))
    }
}

impl std::ops::Deref for SimId {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
