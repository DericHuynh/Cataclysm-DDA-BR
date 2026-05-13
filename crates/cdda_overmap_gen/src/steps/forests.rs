//! Step 2a: Place forests using simplex noise.
//!
//! Port of CDDA master's `overmap::place_forests()` (C++ L2216-2338).
//! Only overwrites the default FIELD terrain.
//!
//! Uses `OvermapRegionSettings` for thresholds and
//! `calculate_forestosity()` for the directional gradient.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_noise;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use tracing::info;

/// Port of overmap::calculate_forestosity() (C++ L2331-2365).
///
/// Computes the directional forest density adjustment based on how far
/// the overmap is from the origin in each cardinal direction.
///
/// `forest_increase` is indexed by om_direction order: [North, East, South, West].
/// Positive values increase forest density in that direction.
pub fn calculate_forestosity(om_x: i32, om_y: i32, settings: &OvermapRegionSettings) -> f32 {
    let northern = settings.forest_increase[0]; // North
    let eastern = settings.forest_increase[1]; // East
    let southern = settings.forest_increase[2]; // South
    let western = settings.forest_increase[3]; // West

    let mut adjust = 0.0f32;
    if northern != 0.0 && om_y < 0 {
        adjust -= om_y as f32 * northern;
    }
    if eastern != 0.0 && om_x > 0 {
        adjust += om_x as f32 * eastern;
    }
    if western != 0.0 && om_x < 0 {
        adjust -= om_x as f32 * western;
    }
    if southern != 0.0 && om_y > 0 {
        adjust += om_y as f32 * southern;
    }

    adjust.min(settings.forest_max - settings.forest_noise_threshold)
}

/// Place FOREST and FOREST_THICK on FIELD tiles using noise.
pub fn place_forests(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let field_index = registry.field_index;

    let forest_adjust = calculate_forestosity(config.om_x, config.om_y, &settings);

    let threshold = settings.forest_noise_threshold;
    let thick_threshold = settings.forest_noise_threshold_thick;
    let seed = config.noise_seed;

    for (chunk_pos, mut chunk) in &mut chunks {
        // Only process z=0
        if chunk_pos.z.0 != 0 {
            continue;
        }

        let (origin_x, origin_y) = chunk_pos.omt_origin();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let handle = chunk.get(lx, ly);
                if handle.type_index() != field_index {
                    continue;
                }

                let wx = origin_x + lx as i32;
                let wy = origin_y + ly as i32;
                let n = cdda_noise::forest_noise_at(wx, wy, seed);

                if n + forest_adjust > thick_threshold {
                    let thick = registry
                        .handle_by_id("forest_thick")
                        .unwrap_or(TerrainHandle::new(0, 0));
                    chunk.set(lx, ly, thick);
                } else if n + forest_adjust > threshold {
                    let forest = registry
                        .handle_by_id("forest")
                        .unwrap_or(TerrainHandle::new(0, 0));
                    chunk.set(lx, ly, forest);
                }
            }
        }
    }

    info!(
        "Forests placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
