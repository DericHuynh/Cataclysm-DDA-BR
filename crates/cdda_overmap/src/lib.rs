//! # cdda_overmap — Overmap terrain storage and spatial lookup
//!
//! ## Coordinate model
//!
//! ```text
//! Map tile      1×1 tile (finest unit)
//! Submap        12×12 map tiles
//! OMT           2×2 submaps = 24×24 map tiles  (overmap tile)
//! Chunk         30×30 OMTs  (storage partition; 180 / 30 = 6 per overmap axis)
//! Overmap       180×180 OMTs = 6×6 chunks = 360×360 submaps
//! ```
//!
//! ## Architecture
//!
//! - **`OvermapChunk`** — a 30×30 block of `TerrainHandle` values (one per OMT).
//! - **`ChunkState`** — generation lifecycle: `Generating → Finalizing → Ready`.
//! - **`TerrainRegistry`** — SoA table: O(1) flag/cost/family lookup by handle.
//! - **`CoreTerrains`** — pre-resolved handles for game-specific terrain types.
//! - **`ChunkIndex`** — O(1) chunk entity lookup; maintained by `OnAdd`/`OnRemove` observers.
//! - **`EntitySpatialIndex`** — spatial grid for dynamic entities; synced via `Changed<WorldPos>`.
//! - **`TerrainQuery`** — `SystemParam` for world-coordinate terrain reads.
//! - **`OvermapCamera`** — viewport center for the overmap viewer.

pub mod camera;
pub mod chunk;
pub mod connections;
pub mod direction;
pub mod index;
pub mod pathfinding;
pub mod query;
pub mod registry;
pub mod rng;
pub mod serial;
pub mod spatial;

pub use camera::OvermapCamera;
pub use chunk::{
    ChunkOfOvermap, ChunkPosition, ChunkState, OvermapChunk, OvermapChunks,
    CHUNK_DIM, CHUNK_SIZE, CHUNKS_PER_LAYER, CHUNKS_PER_OVERMAP,
    OMAP_DIM, OMAP_DIM_SUBMAPS, OMT_DIM_TILES,
    SUBMAP_DIM, SUBMAP_SIZE, SUBMAPS_PER_OMT, SUBMAPS_PER_OMT_AXIS,
};
pub use connections::{
    closest_points_first, connect_closest_points, inbounds_omt, inbounds_omt_margin,
    line_between, point_flood_fill_4, square_dist, trig_dist, ConnectionType,
};
pub use direction::{CubeDirection, OmDirection, Rng, FOUR_ADJACENT_OFFSETS};
pub use index::ChunkIndex;
pub use pathfinding::{greedy_path, DirectedNode, NodeScore, TwoNodeScoringFn};
pub use query::{
    has_any_flag, has_family, has_flag, is_ot_match, omt_submaps, omt_to_submap_origin,
    submap_to_omt, OtMatchType, TerrainQuery,
};
pub use registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
pub use rng::XorShiftRng;
pub use spatial::{
    remove_from_spatial_index, remove_raw_pos_from_spatial_index, sync_spatial_index,
    EntitySpatialIndex,
};
