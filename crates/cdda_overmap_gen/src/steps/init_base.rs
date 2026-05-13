//! Step 1: Fill all z-level chunks (-10..=10) with the default terrain type.
//!
//! Reads `OvermapGenConfig` for the overmap position and spawns
//! chunk entities filled with the region's default terrain.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNKS_PER_OVERMAP};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_core_types::core::coords::ZLevel;
use crate::pipeline::OvermapGenConfig;
use tracing::info;

/// Fill all z-level chunks (-10..=10) with the default terrain for the region.
pub fn init_base_terrain(
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
) {
    assert!(registry.field_index > 0, "No field terrain registered!");
    let default_handle = TerrainHandle::new(registry.field_index, 0);

    info!(
        "InitBase: spawning {} chunks for overmap ({}, {})",
        CHUNKS_PER_OVERMAP, config.om_x, config.om_y
    );

    let om_x = config.om_x;
    let om_y = config.om_y;
    for z_val in -10i8..=10i8 {
        let z = ZLevel::new(z_val);
        for cy in 0u8..6 {
            for cx in 0u8..6 {
                commands.spawn((
                    ChunkPosition {
                        om_x,
                        om_y,
                        z,
                        chunk_x: cx,
                        chunk_y: cy,
                    },
                    OvermapChunk::new_filled(default_handle),
                ));
            }
        }
    }
}
