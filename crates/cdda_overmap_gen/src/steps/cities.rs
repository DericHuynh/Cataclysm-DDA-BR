//! Step 4: Place city centers.
//!
//! Verbatim port of CDDA master's:
//! - `overmap::place_cities()` (overmap_city.cpp L65-212)
//! - `overmap::calculate_forestosity()` (overmap.cpp L2331-2369)
//! - `overmap::calculate_urbanity()` (overmap.cpp L2367-2426)

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::direction::Rng;
use cdda_overmap::registry::{CoreTerrains, TerrainHandle, TerrainRegistry};
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

/// Tracks which OMT tiles belong to cities and where buildings are placed.
#[derive(Resource, Debug, Clone, Default)]
pub struct CityTiles {
    pub tiles: HashSet<(i32, i32)>,
    pub buildings: HashSet<(i32, i32)>,
}

// ===========================================================================
// calculate_forestosity — port of overmap.cpp L2331-2369
// ===========================================================================

/// Computes the directional forest density adjustment based on how far
/// the overmap is from the origin in each cardinal direction.
///
/// `forest_increase` is indexed by om_direction order: [North, East, South, West].
/// Positive values increase forest density in that direction.
///
/// Returns the **raw noise adjust** (`forest_size_adjust` in C++), clamped to
/// `[0, forest_max - forest_noise_threshold]`. Callers needing the C++
/// `forestosity` value must multiply by 25.0.
pub fn calculate_forestosity(om_x: i32, om_y: i32, settings: &OvermapRegionSettings) -> f32 {
    if !settings.overmap_forest {
        return 0.0;
    }

    // C++ indices: west=3, north=0, east=1, south=2
    let western = settings.forest.forest_increase[3];
    let northern = settings.forest.forest_increase[0];
    let eastern = settings.forest.forest_increase[1];
    let southern = settings.forest.forest_increase[2];

    let mut forest_size_adjust: f32 = 0.0;

    // C++ checks in order: west, north, east, south
    if western != 0.0 && om_x < 0 {
        forest_size_adjust -= om_x as f32 * western;
    }
    if northern != 0.0 && om_y < 0 {
        forest_size_adjust -= om_y as f32 * northern;
    }
    if eastern != 0.0 && om_x > 0 {
        forest_size_adjust += om_x as f32 * eastern;
    }
    if southern != 0.0 && om_y > 0 {
        forest_size_adjust += om_y as f32 * southern;
    }

    forest_size_adjust
        .min(settings.forest.max_forest - settings.forest.noise_threshold_forest)
        .max(0.0)
}

// ===========================================================================
// calculate_urbanity — port of overmap.cpp L2367-2426
// ===========================================================================

/// Compute the directional urban density adjustment.
///
/// `urban_increase` is indexed by om_direction order: [North, East, South, West].
/// Positive values increase urban density in that direction.
pub fn calculate_urbanity(om_x: i32, om_y: i32, settings: &OvermapRegionSettings) -> i32 {
    let op_city_size = settings.city.city_size;
    if op_city_size <= 0 {
        return 0;
    }

    // C++ indices: north=0, east=1, south=2, west=3
    let northern = settings.urban_increase[0];
    let eastern = settings.urban_increase[1];
    let southern = settings.urban_increase[2];
    let western = settings.urban_increase[3];

    if northern == 0.0 && eastern == 0.0 && western == 0.0 && southern == 0.0 {
        return 0;
    }

    let mut urbanity_adj: f32 = 0.0;

    // C++ L2387-2423
    if northern != 0.0 && om_y < 0 {
        urbanity_adj -= om_y as f32 * northern / 10.0;
        if om_x < 0 && western == 0.0 {
            urbanity_adj /= (om_x as f32 / -2.0).max(1.0);
        }
        if om_x > 0 && eastern == 0.0 {
            urbanity_adj /= (om_x as f32 / 2.0).max(1.0);
        }
    }
    if eastern != 0.0 && om_x > 0 {
        urbanity_adj += om_x as f32 * eastern / 10.0;
        if om_y < 0 && northern == 0.0 {
            urbanity_adj /= (om_y as f32 / -2.0).max(1.0);
        }
        if om_y > 0 && southern == 0.0 {
            urbanity_adj /= (om_y as f32 / 2.0).max(1.0);
        }
    }
    if western != 0.0 && om_x < 0 {
        urbanity_adj -= om_x as f32 * western / 10.0;
        if om_y < 0 && northern == 0.0 {
            urbanity_adj /= (om_y as f32 / -2.0).max(1.0);
        }
        if om_y > 0 && southern == 0.0 {
            urbanity_adj /= (om_y as f32 / 2.0).max(1.0);
        }
    }
    if southern != 0.0 && om_y > 0 {
        urbanity_adj += om_y as f32 * southern / 10.0;
        if om_x < 0 && western == 0.0 {
            urbanity_adj /= (om_x as f32 / -2.0).max(1.0);
        }
        if om_x > 0 && eastern == 0.0 {
            urbanity_adj /= (om_x as f32 / 2.0).max(1.0);
        }
    }

    urbanity_adj as i32
}

// ===========================================================================
// place_cities — verbatim port of overmap::place_cities() (L65-212)
// ===========================================================================

pub fn place_cities(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    _registry: Res<TerrainRegistry>,
    core_terrains: Res<CoreTerrains>,
    settings: Res<OvermapRegionSettings>,
) {
    // L66-69: read settings
    let op_city_spacing = settings.city.city_spacing;
    let op_city_size = settings.city.city_size;
    let max_urbanity = settings.max_urban;

    // L70-72: early return
    if op_city_size <= 0 {
        commands.insert_resource(CityTiles::default());
        return;
    }

    // L74: calculate biome stats
    let forest_adjust = calculate_forestosity(config.om_x, config.om_y, &settings);
    let forestosity = forest_adjust * 25.0;
    let urbanity = calculate_urbanity(config.om_x, config.om_y, &settings);

    // L75-83: city size / spacing adjustment
    let city_size_adjust = (urbanity - (forestosity / 2.0) as i32).min(-op_city_size + 2);
    let mut city_space_adjust = urbanity / 2;
    let mut max_city_size = (op_city_size + city_size_adjust).min(op_city_size * max_urbanity);
    if max_city_size < op_city_size {
        max_city_size = op_city_size;
    }

    let mut op_city_spacing = op_city_spacing;
    if op_city_spacing > 0 {
        city_space_adjust = city_space_adjust.min(op_city_spacing - 2);
        op_city_spacing = op_city_spacing - city_space_adjust + forestosity as i32;
    }
    op_city_spacing = op_city_spacing.min(10);

    // L85-102: coverage ratio + number of cities
    let omts_per_overmap = (OMAP_DIM * OMAP_DIM) as f64;
    let city_map_coverage_ratio = 1.0 / (2.0_f64).powi(op_city_spacing);
    let omts_per_city = (op_city_size * 2 + 1) as f64 * (max_city_size * 2 + 1) as f64 * 3.0 / 4.0;

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 1);
    let num_cities_on_this_overmap =
        rng.roll_remainder(omts_per_overmap * city_map_coverage_ratio / omts_per_city);

    let road_nesw = core_terrains.road_nesw;
    let field_handle = core_terrains.field;

    // Build dense grid from chunks at z=0
    let omap_size = OMAP_DIM as usize;
    let mut grid = vec![TerrainHandle::NULL; omap_size * omap_size];
    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as i32 {
            for lx in 0..CHUNK_DIM as i32 {
                let gx = ox + lx;
                let gy = oy + ly;
                if gx >= 0 && gx < OMAP_DIM && gy >= 0 && gy < OMAP_DIM {
                    let src_idx = (ly as usize) * CHUNK_DIM + (lx as usize);
                    grid[(gy as usize) * omap_size + (gx as usize)] = chunk.terrain[src_idx];
                }
            }
        }
    }

    // Inline terrain access
    let ter = |x: i32, y: i32| -> TerrainHandle {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            grid[(y as usize) * omap_size + (x as usize)]
        } else {
            TerrainHandle::NULL
        }
    };

    let mut tile_writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();

    // L104-105: build city candidate list
    let center = OMAP_DIM / 2;
    let radius = OMAP_DIM / 2 - max_city_size;

    let mut city_candidates: Vec<(i32, i32)> = Vec::new();
    for y in (center - radius)..=(center + radius) {
        for x in (center - radius)..=(center + radius) {
            let dist = (x - center).abs().max((y - center).abs());
            if dist <= radius && ter(x, y) == field_handle {
                city_candidates.push((x, y));
            }
        }
    }

    // L108-167: megacity mode
    if settings.city.is_megacity {
        let quarter_x = OMAP_DIM / 4;
        let quarter_y = OMAP_DIM / 4;
        let megacity_points = [
            (quarter_x, quarter_y),
            (quarter_x, quarter_y * 3),
            (quarter_x * 2, quarter_y * 2),
            (quarter_x * 3, quarter_y),
            (quarter_x * 3, quarter_y * 3),
        ];

        let mut city_tiles = CityTiles::default();

        for &(x, y) in &megacity_points {
            tile_writes.push((x, y, road_nesw));
            city_tiles.tiles.insert((x, y));
            commands.spawn(City {
                size: 40,
                omt_x: x,
                omt_y: y,
            });
        }

        flush_tile_writes(&chunks, &par_commands, &tile_writes);
        commands.insert_resource(city_tiles);
        info!(
            "Megacity placed: 5 cities for overmap ({}, {})",
            config.om_x, config.om_y
        );
        return;
    }

    // L169-211: random city placement
    let num_cities_count = num_cities_on_this_overmap as usize;
    let mut cities_placed: Vec<(i32, i32, i32)> = Vec::new();
    let mut city_tiles = CityTiles::default();

    while cities_placed.len() < num_cities_count && !city_candidates.is_empty() {
        // L172-183: random size distribution
        let base_size = rng.range_i32(op_city_size - 1, max_city_size);
        let size = if rng.one_in(3) {
            base_size * 1 / 3
        } else if rng.one_in(2) {
            base_size * 2 / 3
        } else if rng.one_in(2) {
            base_size * 3 / 2
        } else {
            base_size * 2
        };
        let size = size.max(2).min(55);

        // L184-198: random_entry_removed + remove radius-2 neighbours
        let idx = rng.random_usize(city_candidates.len());
        let (sx, sy) = city_candidates.swap_remove(idx);
        city_candidates.retain(|&(cx, cy)| (cx - sx).abs() > 2 || (cy - sy).abs() > 2);

        // L199-208: place city
        tile_writes.push((sx, sy, road_nesw));
        city_tiles.tiles.insert((sx, sy));
        cities_placed.push((sx, sy, size));
    }

    for &(x, y, size) in &cities_placed {
        commands.spawn(City {
            size: size as u8,
            omt_x: x,
            omt_y: y,
        });
    }

    flush_tile_writes(&chunks, &par_commands, &tile_writes);
    commands.insert_resource(city_tiles);

    info!(
        "Cities placed: {} for overmap ({}, {})",
        cities_placed.len(),
        config.om_x,
        config.om_y
    );
}

// ---------------------------------------------------------------------------
// Helper: flush recorded tile writes back to chunk entities
// ---------------------------------------------------------------------------

fn flush_tile_writes(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: &ParallelCommands,
    tile_writes: &[(i32, i32, TerrainHandle)],
) {
    if tile_writes.is_empty() {
        return;
    }
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let local_ox = (chunk_pos.chunk_x as i32) * (CHUNK_DIM as i32);
        let local_oy = (chunk_pos.chunk_y as i32) * (CHUNK_DIM as i32);

        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, handle) in tile_writes {
            let lx = wx - local_ox;
            let ly = wy - local_oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
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
