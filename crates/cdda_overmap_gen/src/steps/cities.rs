//! Step 3: Place city centers using CDDA's coverage-ratio formula.
//!
//! Port of CDDA master's `overmap::place_cities()` (overmap_city.cpp L65-212).

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::forests::calculate_forestosity;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::HashSet;
use tracing::info;

/// A placed city center entity.
#[derive(Component)]
pub struct City {
    pub size: u8,
    pub omt_x: i32,
    pub omt_y: i32,
}

/// Tracks which OMT tiles belong to cities.
#[derive(Resource, Debug, Clone, Default)]
pub struct CityTiles {
    pub tiles: HashSet<(i32, i32)>,
}

// ---------------------------------------------------------------------------
// calculate_urbanity — port of overmap.cpp L2367-2426
// ---------------------------------------------------------------------------

/// Compute the directional urban density adjustment.
///
/// `urban_increase` is indexed by om_direction order:
/// `[North, East, South, West]`.  Positive values increase urban density
/// in that direction.
pub fn calculate_urbanity(om_x: i32, om_y: i32, settings: &OvermapRegionSettings) -> i32 {
    let op_city_size = settings.city_size;
    if op_city_size <= 0 {
        return 0;
    }

    let northern = settings.urban_increase[0]; // North
    let eastern = settings.urban_increase[1]; // East
    let southern = settings.urban_increase[2]; // South
    let western = settings.urban_increase[3]; // West

    if northern == 0 && eastern == 0 && western == 0 && southern == 0 {
        return 0;
    }

    let mut urbanity_adj: f32 = 0.0;

    if northern != 0 && om_y < 0 {
        urbanity_adj -= om_y as f32 * northern as f32 / 10.0;
        if om_x < 0 && western == 0 {
            urbanity_adj /= (om_x as f32 / -2.0).max(1.0);
        }
        if om_x > 0 && eastern == 0 {
            urbanity_adj /= (om_x as f32 / 2.0).max(1.0);
        }
    }
    if eastern != 0 && om_x > 0 {
        urbanity_adj += om_x as f32 * eastern as f32 / 10.0;
        if om_y < 0 && northern == 0 {
            urbanity_adj /= (om_y as f32 / -2.0).max(1.0);
        }
        if om_y > 0 && southern == 0 {
            urbanity_adj /= (om_y as f32 / 2.0).max(1.0);
        }
    }
    if western != 0 && om_x < 0 {
        urbanity_adj -= om_x as f32 * western as f32 / 10.0;
        if om_y < 0 && northern == 0 {
            urbanity_adj /= (om_y as f32 / -2.0).max(1.0);
        }
        if om_y > 0 && southern == 0 {
            urbanity_adj /= (om_y as f32 / 2.0).max(1.0);
        }
    }
    if southern != 0 && om_y > 0 {
        urbanity_adj += om_y as f32 * southern as f32 / 10.0;
        if om_x < 0 && western == 0 {
            urbanity_adj /= (om_x as f32 / -2.0).max(1.0);
        }
        if om_x > 0 && eastern == 0 {
            urbanity_adj /= (om_x as f32 / 2.0).max(1.0);
        }
    }

    urbanity_adj as i32
}

// ---------------------------------------------------------------------------
// place_cities — port of overmap::place_cities() (L65-212)
// ---------------------------------------------------------------------------

/// Place city centers on the overmap.
pub fn place_cities(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let op_city_size = settings.city_size;
    if op_city_size <= 0 {
        commands.insert_resource(CityTiles::default());
        return;
    }

    let op_city_spacing = settings.city_spacing;
    let max_urbanity = settings.max_urban;

    // --- Adjust city size / spacing by forestosity & urbanity ---
    // C++ calculate_forestosity stores BOTH forest_size_adjust (raw, for
    // noise threshold) AND forestosity = forest_size_adjust * 25.0 (for
    // city sizing).  Our fn returns the raw adjust — scale here to match.
    let forest_adjust = calculate_forestosity(config.om_x, config.om_y, &settings);
    let forestosity = forest_adjust * 25.0;
    let urbanity = calculate_urbanity(config.om_x, config.om_y, &settings);

    let city_size_adjust = (urbanity - (forestosity / 2.0) as i32).min(-op_city_size + 2);
    let mut max_city_size = (op_city_size + city_size_adjust).min(op_city_size * max_urbanity);
    if max_city_size < op_city_size {
        max_city_size = op_city_size;
    }

    let mut city_space_adjust = urbanity / 2;
    let mut op_city_spacing = op_city_spacing;
    if op_city_spacing > 0 {
        city_space_adjust = city_space_adjust.min(op_city_spacing - 2);
        op_city_spacing = op_city_spacing - city_space_adjust + forestosity as i32;
    }
    op_city_spacing = op_city_spacing.min(10);

    let omts_per_overmap = (OMAP_DIM * OMAP_DIM) as f64;
    let city_map_coverage_ratio = 1.0 / (2.0_f64).powi(op_city_spacing);
    let omts_per_city = (op_city_size * 2 + 1) as f64 * (max_city_size * 2 + 1) as f64 * 3.0 / 4.0;

    let road_nesw = registry
        .handle_by_id("road_nesw")
        .unwrap_or(TerrainHandle::new(0, 0));

    let field_index = registry.field_index;

    let mut tile_writes: Vec<(i32, i32, i8, TerrainHandle)> = Vec::new();

    // --- Megacity mode (CDDA L152-167) ---
    if settings.is_megacity {
        let quarter_x = OMAP_DIM / 4;
        let quarter_y = OMAP_DIM / 4;
        let megacity_points: [(i32, i32); 5] = [
            (quarter_x, quarter_y),
            (quarter_x, quarter_y * 3),
            (quarter_x * 2, quarter_y * 2),
            (quarter_x * 3, quarter_y),
            (quarter_x * 3, quarter_y * 3),
        ];

        let mut city_tiles = CityTiles::default();

        for &(x, y) in &megacity_points {
            tile_writes.push((x, y, 0, road_nesw));
            city_tiles.tiles.insert((x, y));

            commands.spawn(City {
                size: 40,
                omt_x: x,
                omt_y: y,
            });
        }

        apply_tile_writes(&chunks, &par_commands, &tile_writes);

        commands.insert_resource(city_tiles);
        info!(
            "Megacity placed: 5 cities for overmap ({}, {})",
            config.om_x, config.om_y
        );
        return;
    }

    // --- Non-megacity: random city placement ---

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 1);
    let num_cities =
        rng.roll_remainder(omts_per_overmap * city_map_coverage_ratio / omts_per_city) as usize;
    if num_cities == 0 {
        commands.insert_resource(CityTiles::default());
        return;
    }

    // ---- Phase 1: build candidate list ----
    let half = OMAP_DIM / 2;
    let margin = max_city_size;
    let mut city_candidates: Vec<(i32, i32)> = Vec::new();

    for x in margin..(OMAP_DIM - margin) {
        for y in margin..(OMAP_DIM - margin) {
            let dist = (x - half).abs().max((y - half).abs());
            if dist > half - max_city_size {
                continue;
            }
            let mut is_field = false;
            for (_entity, chunk_pos, chunk) in &chunks {
                if chunk_pos.z.0 != 0 {
                    continue;
                }
                let (ox, oy) = chunk_pos.omt_origin();
                let lx = x - ox;
                let ly = y - oy;
                if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                    let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
                    is_field = chunk.terrain[idx].type_index() == field_index;
                    break;
                }
            }
            if is_field {
                city_candidates.push((x, y));
            }
        }
    }

    // ---- Phase 2: place cities ----
    let mut cities_placed_count: usize = 0;
    let mut city_tiles = CityTiles::default();

    while cities_placed_count < num_cities && !city_candidates.is_empty() {
        let idx = rng.range_i32(0, city_candidates.len() as i32 - 1) as usize;
        let selected = city_candidates.remove(idx);

        // City size distribution (CDDA L173-183):
        //   33% tiny  (1/3), 33% small (2/3), 17% large (3/2), 17% huge (2×)
        let base = rng.range_i32(op_city_size - 1, max_city_size);
        let roll = rng.next_u32() as f64 / ((u32::MAX as f64) + 1.0);
        let size = if roll < 0.33 {
            (base as f64 * 1.0 / 3.0) as i32
        } else if roll < 0.66 {
            (base as f64 * 2.0 / 3.0) as i32
        } else if roll < 0.83 {
            (base as f64 * 3.0 / 2.0) as i32
        } else {
            base * 2
        }
        .max(2)
        .min(55);

        tile_writes.push((selected.0, selected.1, 0, road_nesw));
        city_tiles.tiles.insert(selected);

        commands.spawn(City {
            size: size as u8,
            omt_x: selected.0,
            omt_y: selected.1,
        });

        cities_placed_count += 1;

        // Remove candidates within radius 2 (CDDA L192-198).
        let r2 = 2;
        city_candidates
            .retain(|&(cx, cy)| (cx - selected.0).abs() > r2 || (cy - selected.1).abs() > r2);
    }

    apply_tile_writes(&chunks, &par_commands, &tile_writes);

    commands.insert_resource(city_tiles);

    info!(
        "Cities placed: {} for overmap ({}, {})",
        cities_placed_count, config.om_x, config.om_y
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn apply_tile_writes(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: &ParallelCommands,
    tile_writes: &[(i32, i32, i8, TerrainHandle)],
) {
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z = chunk_pos.z.0;
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();
        let (ox, oy) = chunk_pos.omt_origin();

        for &(wx, wy, wz, handle) in tile_writes {
            if z != wz {
                continue;
            }
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
                if new_terrain[idx] != handle {
                    new_terrain[idx] = handle;
                    modified = true;
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
}
