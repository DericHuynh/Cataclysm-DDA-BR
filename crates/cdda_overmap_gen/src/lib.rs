//! # cdda_overmap_gen — Overmap generation pipeline
//!
//! Verbatim port of CDDA master's `overmap::generate()` (overmap.cpp L932-1060).
//!
//! Composable Bevy ECS systems that generate overmap terrain in ordered
//! phases matching the C++ generation order exactly.
//!
//! ## Pipeline order (1:1 with CDDA master)
//!
//! 1. **InitBase** — fill all z-level chunks with default terrain
//! 2. **NeighborConnections** — populate cross-overmap connection exits
//! 3. **NaturalTerrain** — rivers, lakes, oceans, forests, swamps, ravines
//! 4. **Highways** — highway path placement (before cities)
//! 5. **Cities** — city center placement
//! 6. **PostCities** — highway interchanges, then city street grids
//! 7. **Connections** — roads, railroads, forest trails
//! 8. **Structures** — overmap specials (fixed + mutable)
//! 9. **PreUnderground** — finalize highways, trailheads, polish rivers
//! 10. **Underground** — sewers, subways (z < 0)
//! 11. **Elevated** — bridges, railroad bridges (z > 0)
//! 12. **Population** — monster groups, radio towers
//! 13. **Finalize** — mark chunks immutable, log statistics

pub mod pipeline;
pub mod region_settings;
pub mod special_catalog;
pub mod steps;
