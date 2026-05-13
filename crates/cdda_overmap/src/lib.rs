//! # cdda_overmap — Overmap terrain storage and spatial lookup
//!
//! ## Architecture
//!
//! - **`OvermapChunk`** — a 32×32 block of `TerrainHandle` values.
//! - **`TerrainRegistry`** — maps type indices to definition entities and properties.
//! - **`TerrainQuery`** — efficient spatial queries over chunk entities.
//! - **`OvermapCamera`** — viewport center for the overmap viewer.
//! - **`EntitySpatialIndex`** — 3D spatial partitioning for dynamic entities.

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
    ChunkOfOvermap, ChunkPosition, OvermapChunk, OvermapChunks, CHUNKS_PER_LAYER, CHUNK_DIM,
};
pub use connections::{
    closest_points_first, connect_closest_points, inbounds_omt, inbounds_omt_margin, line_between,
    point_flood_fill_4, square_dist, trig_dist, ConnectionType,
};
pub use direction::{CubeDirection, OmDirection, Rng, FOUR_ADJACENT_OFFSETS};
pub use index::ChunkIndex;
pub use pathfinding::{greedy_path, DirectedNode, NodeScore, TwoNodeScoringFn};
pub use query::{is_ot_match, OtMatchType, TerrainQuery};
pub use registry::{TerrainHandle, TerrainRegistry};
pub use rng::XorShiftRng;
pub use spatial::EntitySpatialIndex;
