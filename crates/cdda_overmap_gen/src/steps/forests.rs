//! Step 2a: Place forests using simplex noise.
//!
//! Port of CDDA master's `overmap::place_forests()` (C++ L2051-2077) and
//! `calculate_forestosity()` (C++ L2331-2369).
//!
//! Only overwrites the default FIELD terrain.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_noise;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::TerrainRegistry;
use tracing::info;

/// Port of overmap::calculate_forestosity() (C++ L2331-2369).
///
/// Computes the directional forest density adjustment based on how far
/// the overmap is from the origin in each cardinal direction.
///
/// `forest_increase` is indexed by om_direction order: [North, East, South, West].
/// Positive values increase forest density in that direction.
///
/// # Return value
///
/// Returns the **raw noise adjust** (`forest_size_adjust` in C++), clamped to
/// `[0, forest_max - forest_noise_threshold]`.  This is the value added to the
/// forest noise sample before threshold comparison.
///
/// Callers that need the C++ `forestosity` value (used for city sizing) must
/// multiply by 25.0: `forestosity = calculate_forestosity(...) * 25.0`.
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
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let field_index = registry.field_index;
    let forest_adjust = calculate_forestosity(config.om_x, config.om_y, &settings);
    let threshold = settings.forest_noise_threshold;
    let thick_threshold = settings.forest_noise_threshold_thick;
    let seed = config.noise_seed;

    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
                let handle = chunk.terrain[idx];
                if handle.type_index() != field_index {
                    continue;
                }
                let wx = ox + lx as i32;
                let wy = oy + ly as i32;
                let n = cdda_noise::forest_noise_at(wx, wy, seed);

                if n + forest_adjust > thick_threshold {
                    if let Some(h) = registry.handle_by_id("forest_thick") {
                        new_terrain[idx] = h;
                        modified = true;
                    }
                } else if n + forest_adjust > threshold {
                    if let Some(h) = registry.handle_by_id("forest") {
                        new_terrain[idx] = h;
                        modified = true;
                    }
                }
            }
        }
        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: new_terrain,
                });
            });
        }
    });

    info!(
        "Forests placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
