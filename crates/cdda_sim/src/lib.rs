//! # cdda_sim — Simulation layer
//!
//! ECS components, systems, events, and the deterministic tick loop.
//! Depends on bevy_ecs and bevy_reflect (not full Bevy).

pub mod components;
pub mod events;
pub mod logic;
pub mod systems;
pub mod tick;
pub mod world_setup;
