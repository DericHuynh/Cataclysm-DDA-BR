//! # cdda_overmap_gen — Overmap generation pipeline
//!
//! Composable Bevy ECS systems that generate overmap terrain in ordered
//! phases. Each generation step is a regular system — Bevy's scheduler
//! runs them in order via `chain()` and parallelizes within phases when
//! systems access disjoint chunks.
//!
//! ## Pipeline order (matching CDDA master)
//!
//! 1. **InitBase** — fill all z=0 chunks with default terrain
//! 2. **NaturalTerrain** — forests, lakes, oceans, swamps (noise-driven)
//! 3. **Rivers** — river placement, meandering, shore building
//! 4. **Cities** — city center placement with coverage formula
//! 5. **Connections** — roads, railroads, forest trails (A* pathfinding)
//! 6. **Structures** — city buildings, overmap specials
//! 7. **Underground** — sewers, subways, ant/goo nests (z < 0)
//! 8. **Elevated** — bridges, railroad bridges (z > 0)
//! 9. **Population** — monster groups, NPCs, radios
//! 10. **Finalize** — mark chunks immutable, fire completion events

pub mod connection_catalog;
pub mod mongroup_catalog;
pub mod pipeline;
pub mod region_settings;
pub mod setup;
pub mod spawning;
pub mod spatial_systems;
pub mod special_catalog;
pub mod steps;

pub use pipeline::{OvermapGenPlugin, OvermapGenSet};
pub use region_settings::OvermapRegionSettings;
