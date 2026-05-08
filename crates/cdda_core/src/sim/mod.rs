//! # cdda_sim — Simulation layer
//!
//! ECS components, systems, events, and the deterministic tick loop.
//! Depends on bevy_ecs and bevy_reflect (not full Bevy).

pub mod components;
pub mod def_components;
pub mod def_world;
pub mod dev_worldgen;
pub mod events;
pub mod flags;
pub mod populate_flags;
pub mod spatial;
pub mod state;
pub mod systems;
pub mod test_utils;
pub mod world_setup;
