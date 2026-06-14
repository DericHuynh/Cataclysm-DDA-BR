//! Forest terrain generation — verbatim port of C++ `overmap::place_forests()`
//! (overmap.cpp L2051-2077) and `calculate_forestosity()` (L2331-2369).
//!
//! ## Algorithm
//!
//! 1. Read all z=0 chunk tiles into a dense `[[u32; 180]; 180]` grid.
//! 2. Compute `forest_size_adjust` via [`calculate_forestosity`] using the
//!    region settings and overmap position.
//! 3. For each tile that is still the default (field) terrain, sample
//!    forest noise and compare against the adjusted thresholds to decide
//!    whether to place `forest` or `forest_thick`.
//! 4. Write modified tiles back to chunks via `ParallelCommands`.

use bevy_ecs::prelude::*;
use cdda_sim::noise::forest_noise_at;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE, OMAP_DIM};
use cdda_overmap::registry::{CoreTerrains, TerrainHandle};
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::{OvermapRegionSettings, RegionSettingsForest};

// ---------------------------------------------------------------------------
// calculate_forestosity — directional forest density adjustment
// ---------------------------------------------------------------------------

/// Compute the forest-size adjustment factor for this overmap.
///
/// Verbatim port of C++ `overmap::calculate_forestosity()`
/// (overmap.cpp L2331-2369).
///
/// Uses the `forest_increase` directional array indexed in N-E-S-W order
/// (matching `om_direction::type` discriminants: North=0, East=1, South=2, West=3).
///
/// Returns `(forest_size_adjust, forestosity)`.
pub fn calculate_forestosity(
    om_x: i32,
    om_y: i32,
    settings_forest: &RegionSettingsForest,
) -> (f32, f32) {
    let northern_forest_increase = settings_forest.forest_increase[0]; // North
    let eastern_forest_increase = settings_forest.forest_increase[1]; // East
    let southern_forest_increase = settings_forest.forest_increase[2]; // South
    let western_forest_increase = settings_forest.forest_increase[3]; // West

    let mut forest_size_adjust: f32 = 0.0;

    // C++ L2338-2357 — directional adjustments keyed by overmap position relative to origin
    if western_forest_increase != 0.0 && om_x < 0 {
        forest_size_adjust -= om_x as f32 * western_forest_increase;
    }
    if northern_forest_increase != 0.0 && om_y < 0 {
        forest_size_adjust -= om_y as f32 * northern_forest_increase;
    }
    if eastern_forest_increase != 0.0 && om_x > 0 {
        forest_size_adjust += om_x as f32 * eastern_forest_increase;
    }
    if southern_forest_increase != 0.0 && om_y > 0 {
        forest_size_adjust += om_y as f32 * southern_forest_increase;
    }

    let forestosity = forest_size_adjust * 25.0;

    // Cap so forest never totally overwhelms the map
    forest_size_adjust =
        forest_size_adjust.min(settings_forest.max_forest - settings_forest.noise_threshold_forest);

    (forest_size_adjust, forestosity)
}

// ---------------------------------------------------------------------------
// Grid helpers
// ---------------------------------------------------------------------------

/// Build a dense `[[u32; 180]; 180]` grid from z=0 chunk entities.
fn build_omt_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
) -> ([[u32; 180]; 180], Vec<(Entity, ChunkPosition)>) {
    let mut grid = [[0u32; 180]; 180];
    let mut z0_chunks: Vec<(Entity, ChunkPosition)> = Vec::with_capacity(36);

    for (entity, pos, chunk) in chunks.iter() {
        if pos.z.0 != 0 {
            continue;
        }
        z0_chunks.push((entity, *pos));

        let (origin_x, origin_y) = pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let omt_x = origin_x + lx as i32;
                let omt_y = origin_y + ly as i32;
                if omt_x >= 0 && omt_x < OMAP_DIM && omt_y >= 0 && omt_y < OMAP_DIM {
                    grid[omt_y as usize][omt_x as usize] = chunk.get(lx, ly).0;
                }
            }
        }
    }

    (grid, z0_chunks)
}

// ---------------------------------------------------------------------------
// place_forests — system entry point
// ---------------------------------------------------------------------------

/// Place forest and forest-thick terrain on the overmap.
///
/// Verbatim port of C++ `overmap::place_forests()` (overmap.cpp L2051-2077).
pub fn place_forests(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    mut commands: Commands,
    config: Res<OvermapGenConfig>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.overmap_forest {
        info!("place_forests: skipped — overmap_forest is false");
        return;
    }

    let settings_forest = &settings.forest;

    // Compute global base-point coordinates for noise.
    let global_base_x = config.om_x * OMAP_DIM;
    let global_base_y = config.om_y * OMAP_DIM;

    // Compute forest-size adjustment from region settings.
    let (forest_size_adjust, forestosity) =
        calculate_forestosity(config.om_x, config.om_y, settings_forest);

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        forestosity,
        forest_size_adjust,
        "place_forests: adjustment computed"
    );

    // --- Build grid -----------------------------------------------------------
    let (mut grid, z0_chunks) = build_omt_grid(&chunks);

    let default_terrain_raw = core_terrains.field.0;
    let forest_raw = core_terrains.forest.0;
    let forest_thick_raw = core_terrains.forest_thick.0;

    let threshold_forest = settings_forest.noise_threshold_forest;
    let threshold_forest_thick = settings_forest.noise_threshold_forest_thick;

    let mut forest_count: usize = 0;
    let mut thick_count: usize = 0;

    // --- Compute terrain -------------------------------------------------------
    for y in 0..OMAP_DIM as usize {
        for x in 0..OMAP_DIM as usize {
            // Skip tiles that aren't the default (field) terrain — C++ L2060-2062
            if grid[y][x] != default_terrain_raw {
                continue;
            }

            let global_x = global_base_x + x as i32;
            let global_y = global_base_y + y as i32;
            let n = forest_noise_at(global_x, global_y, config.noise_seed);
            let adjusted = n + forest_size_adjust;

            if adjusted > threshold_forest_thick {
                grid[y][x] = forest_thick_raw;
                thick_count += 1;
            } else if adjusted > threshold_forest {
                grid[y][x] = forest_raw;
                forest_count += 1;
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        forest_tiles = forest_count,
        thick_tiles = thick_count,
        "place_forests: terrain computed"
    );

    // --- Write back to chunks --------------------------------------------------
    write_back_grid(&grid, &z0_chunks, &mut commands);
}

// ---------------------------------------------------------------------------
// Write-back
// ---------------------------------------------------------------------------

/// Write the modified grid back to z=0 chunk entities via `Commands`.
fn write_back_grid(
    grid: &[[u32; 180]; 180],
    z0_chunks: &[(Entity, ChunkPosition)],
    commands: &mut Commands,
) {
    for &(entity, pos) in z0_chunks {
        let (origin_x, origin_y) = pos.omt_origin();
        let mut new_terrain = [TerrainHandle::NULL; CHUNK_SIZE];
        let mut any_changed = false;

        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let omt_x = origin_x + lx as i32;
                let omt_y = origin_y + ly as i32;
                let idx = ly * CHUNK_DIM + lx;
                if omt_x >= 0 && omt_x < OMAP_DIM as i32 && omt_y >= 0 && omt_y < OMAP_DIM as i32 {
                    let new_handle = TerrainHandle(grid[omt_y as usize][omt_x as usize]);
                    new_terrain[idx] = new_handle;
                    any_changed = true;
                }
            }
        }

        if any_changed {
            commands.entity(entity).insert(OvermapChunk {
                terrain: Box::new(new_terrain),
            });
        }
    }
}
