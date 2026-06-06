//! Pipeline step 1: **InitBase** — fill all z-level chunks with default terrain.
//!
//! Verbatim port of C++ `overmap::init_layers()` (overmap.cpp L269-279).
//!
//! Spawns the [`OvermapEntity`] marker and 756 chunk entities (36 per z-level ×
//! 21 z-levels), all filled with the default field terrain from the
//! [`TerrainRegistry`].
//!
//! ## C++ reference
//!
//! ```cpp
//! void overmap::init_layers() {
//!     for( int k = 0; k < OVERMAP_LAYERS; ++k ) {
//!         const oter_id tid = get_default_terrain( k - OVERMAP_DEPTH );
//!         map_layer &l = layer[k];
//!         l.terrain.fill( tid );
//!     }
//! }
//! ```

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::ZLevel;
use cdda_overmap::chunk::{ChunkOfOvermap, ChunkPosition, OvermapChunk, OvermapChunks, CHUNK_DIM};
use cdda_overmap::registry::CoreTerrains;
use tracing::info;

use crate::pipeline::{OvermapEntity, OvermapGenConfig};

/// Number of chunks along one axis of the overmap (180 OMT ÷ 32 = 6 chunks).
const CHUNK_GRID: u8 = 6;

/// Spawn the overmap entity and all 756 chunk entities filled with default
/// field terrain (matching C++ `overmap::init_layers()`).
///
/// # Entity layout
///
/// - **Overmap entity**: carries [`OvermapEntity`] + [`OvermapChunks`]
///   (the latter is auto-maintained by relationship hooks).
/// - **Chunk entities**: each carries [`ChunkPosition`] + [`OvermapChunk`]
///   + [`ChunkOfOvermap`] linking it to the overmap entity.
pub fn init_base_terrain(
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    core_terrains: Res<CoreTerrains>,
) {
    let default_handle = core_terrains.field;

    // --- spawn the overmap entity -------------------------------------------------
    let overmap_entity = commands
        .spawn((
            OvermapEntity {
                om_x: config.om_x,
                om_y: config.om_y,
            },
            OvermapChunks::new(),
        ))
        .id();

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        default_terrain_id = core_terrains.field.type_index(),
        "InitBase: spawning overmap entity {:?} with default terrain id={}",
        overmap_entity,
        core_terrains.field.type_index(),
    );

    // --- spawn chunks for every z-level -------------------------------------------
    let mut chunk_count: usize = 0;

    for z_raw in -10i8..=10i8 {
        let z = ZLevel::new(z_raw);
        for chunk_y in 0u8..CHUNK_GRID {
            for chunk_x in 0u8..CHUNK_GRID {
                let pos = ChunkPosition {
                    om_x: config.om_x,
                    om_y: config.om_y,
                    z,
                    chunk_x,
                    chunk_y,
                };

                commands.spawn((
                    pos,
                    OvermapChunk::new_filled(default_handle),
                    ChunkOfOvermap(overmap_entity),
                ));

                chunk_count += 1;
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        chunks = chunk_count,
        z_range = "-10..=10",
        "InitBase: spawned {} chunk entities",
        chunk_count,
    );
}

// ---------------------------------------------------------------------------
// Helpers — converting between chunk grid and OMT coordinates
// ---------------------------------------------------------------------------

/// Convert an OMT coordinate (0..180) to a chunk index (0..6) and a local
/// offset within that chunk (0..32).
///
/// `div_euclid` / `rem_euclid` is used so this is correct for negative
/// coordinates as well (when working with world-relative positions).
#[allow(dead_code)]
#[inline]
pub fn omt_to_chunk_local(omt: i32) -> (u8, u8) {
    let chunk = omt.div_euclid(CHUNK_DIM as i32) as u8;
    let local = omt.rem_euclid(CHUNK_DIM as i32) as u8;
    (chunk, local)
}

/// Inverse of [`omt_to_chunk_local`]: given a chunk index and local offset,
/// return the absolute OMT coordinate.
#[allow(dead_code)]
#[inline]
pub fn chunk_local_to_omt(chunk: u8, local: u8) -> i32 {
    chunk as i32 * CHUNK_DIM as i32 + local as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omt_to_chunk_basics() {
        assert_eq!(omt_to_chunk_local(0), (0, 0));
        assert_eq!(omt_to_chunk_local(29), (0, 29));
        assert_eq!(omt_to_chunk_local(30), (1, 0));
        assert_eq!(omt_to_chunk_local(59), (1, 29));
        assert_eq!(omt_to_chunk_local(149), (4, 29));
        assert_eq!(omt_to_chunk_local(179), (5, 29)); // last tile in overmap
    }

    #[test]
    fn omt_to_chunk_negative() {
        // -1 should map to chunk -1, local 29 (euclid, CHUNK_DIM=30)
        assert_eq!(omt_to_chunk_local(-1), (255, 29));
        assert_eq!(omt_to_chunk_local(-30), (255, 0));
    }

    #[test]
    fn roundtrip_positive() {
        for omt in [0, 1, 31, 32, 63, 100, 179] {
            let (chunk, local) = omt_to_chunk_local(omt);
            let back = chunk_local_to_omt(chunk, local);
            assert_eq!(back, omt, "roundtrip failed for omt={omt}");
        }
    }
}
