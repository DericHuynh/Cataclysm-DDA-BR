//! Monster group placement.
//!
//! Port of C++ `overmap::place_mongroups()` (overmap.cpp L3448-3649).
//!
//! Algorithm (matching C++ order):
//! 1. **City zombie hordes**: on road tiles within effective radius of each city,
//!    distribute zombie hordes with population scaled by city size.
//! 2. **Swamp monsters**: 7×7 sliding window over forest_water tiles,
//!    place GROUP_SWAMP when ≥25 swamp tiles in window.
//! 3. **River/lake monsters**: 7×7 window over lake/river tiles,
//!    place GROUP_RIVER when ≥25 water tiles AND center is water.
//! 4. **Ocean monsters**: 7×7 window, noise-gated deep/shore classification,
//!    GROUP_OCEAN_DEEP or GROUP_OCEAN_SHORE.

use bevy_ecs::prelude::*;
use cdda_noise::ocean_noise_at;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;

// ---------------------------------------------------------------------------
// MonsterGroup marker component
// ---------------------------------------------------------------------------

/// A spawned monster group on the overmap.
#[derive(Component)]
pub struct MonsterGroup {
    /// Group type identifier (e.g. "GROUP_ZOMBIE", "GROUP_SWAMP").
    pub group_type: String,
    /// Population count.
    pub population: u32,
    /// OMT x-coordinate.
    pub omt_x: i32,
    /// OMT y-coordinate.
    pub omt_y: i32,
    /// Z-level.
    pub z: i32,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Scalar for city zombie spawn density.
const CITY_SPAWN_SCALAR: f64 = 80.0;
/// Base spawn density.
const SPAWN_DENSITY: f64 = 1.0;
/// How far zombies spread from the city center relative to city size.
const CITY_SPAWN_SPREAD: f64 = 1.5;
/// Maximum horde population.
const MAX_HORDE_POP: u32 = 10;

/// 7×7 window size for swarm/river/ocean monster placement.
const WINDOW_SIZE: i32 = 7;
/// Threshold for swamp tiles in a 7×7 window.
const SWAMP_THRESHOLD: usize = 25;
/// Threshold for water tiles in a 7×7 window.
const WATER_THRESHOLD: usize = 25;

// ---------------------------------------------------------------------------
// place_mongroups — system entry point
// ---------------------------------------------------------------------------

/// Place monster groups on the overmap.
///
/// Port of C++ `overmap::place_mongroups()` (overmap.cpp L3448-3649).
pub fn place_mongroups(
    mut commands: Commands,
    chunks: Query<(&ChunkPosition, &OvermapChunk)>,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
) {
    // --- Build terrain grid from z=0 chunks ---------------------------------
    let omap_size = OMAP_DIM as usize;
    let mut grid = vec![TerrainHandle::NULL; omap_size * omap_size];

    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as i32 {
            for lx in 0..CHUNK_DIM as i32 {
                let gx = ox + lx;
                let gy = oy + ly;
                if gx >= 0 && gx < OMAP_DIM && gy >= 0 && gy < OMAP_DIM {
                    grid[(gy as usize) * omap_size + (gx as usize)] = chunk.get(lx as u8, ly as u8);
                }
            }
        }
    }

    let ter_at = |x: i32, y: i32| -> TerrainHandle {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            grid[(y as usize) * omap_size + (x as usize)]
        } else {
            TerrainHandle::NULL
        }
    };

    let ter_flags = |x: i32, y: i32| -> TerrainFlags { registry.flags_for(ter_at(x, y)) };

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 31);

    // === 1. City zombie hordes =============================================

    let mut horde_count = 0usize;

    for city in cities.iter() {
        let desired_zombies = (city.size as f64 * CITY_SPAWN_SCALAR * SPAWN_DENSITY) as u32;
        let effective_radius = (city.size as f64 * CITY_SPAWN_SPREAD) as i32;

        if desired_zombies == 0 {
            continue;
        }

        let mut hordes_remaining = desired_zombies;

        // Place hordes on road tiles within effective radius
        // Closer tiles get hordes first (spiral outward)
        for r in 0..=effective_radius {
            if hordes_remaining == 0 {
                break;
            }

            let tiles_at_radius = r * 8; // approximate perimeter
            let hordes_this_ring = if r == 0 {
                1
            } else {
                (tiles_at_radius as u32 * hordes_remaining / (effective_radius * 8) as u32)
                    .max(1)
                    .min(hordes_remaining)
            };

            let mut placed_this_ring = 0u32;

            for _attempt in 0..(tiles_at_radius * 4) {
                if placed_this_ring >= hordes_this_ring || hordes_remaining == 0 {
                    break;
                }

                // Pick a random point at this Chebyshev distance
                let dx = rng.range_i32(-r, r);
                let dy = if rng.one_in(2) {
                    r // top or bottom edge
                } else {
                    -r
                };
                // Randomly swap dx/dy
                let (gx, gy) = if rng.one_in(2) {
                    (city.omt_x + dx, city.omt_y + dy)
                } else {
                    (city.omt_x + dy, city.omt_y + dx)
                };

                if gx < 0 || gx >= OMAP_DIM || gy < 0 || gy >= OMAP_DIM {
                    continue;
                }

                let flags = ter_flags(gx, gy);
                if !flags.contains(TerrainFlags::ROAD) {
                    continue;
                }

                let pop = (hordes_remaining as u32).min(MAX_HORDE_POP).max(1);
                let pop = rng.range_i32(1, pop as i32) as u32;

                commands.spawn(MonsterGroup {
                    group_type: "GROUP_ZOMBIE".into(),
                    population: pop,
                    omt_x: gx,
                    omt_y: gy,
                    z: 0,
                });

                hordes_remaining = hordes_remaining.saturating_sub(pop);
                placed_this_ring += 1;
                horde_count += 1;
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        zombie_hordes = horde_count,
        "place_mongroups: city zombie hordes placed"
    );

    // === 2. Swamp monsters — 7×7 sliding window ===========================

    let mut swamp_count = 0usize;

    // Stride by 7
    for y in (0..OMAP_DIM).step_by(WINDOW_SIZE as usize) {
        for x in (0..OMAP_DIM).step_by(WINDOW_SIZE as usize) {
            let mut swamp_tiles = 0usize;
            let mut center_is_swamp = false;

            for wy in 0..WINDOW_SIZE {
                for wx in 0..WINDOW_SIZE {
                    let tx = x + wx;
                    let ty = y + wy;
                    let flags = ter_flags(tx, ty);
                    // Swamp = forest_water (FOREST | LAKE)
                    if flags.contains(TerrainFlags::FOREST) && flags.contains(TerrainFlags::LAKE) {
                        swamp_tiles += 1;
                        if wx == WINDOW_SIZE / 2 && wy == WINDOW_SIZE / 2 {
                            center_is_swamp = true;
                        }
                    }
                }
            }

            if swamp_tiles >= SWAMP_THRESHOLD && center_is_swamp {
                let cx = x + WINDOW_SIZE / 2;
                let cy = y + WINDOW_SIZE / 2;
                if cx < OMAP_DIM && cy < OMAP_DIM {
                    commands.spawn(MonsterGroup {
                        group_type: "GROUP_SWAMP".into(),
                        population: rng.range_i32(3, 8) as u32,
                        omt_x: cx,
                        omt_y: cy,
                        z: 0,
                    });
                    swamp_count += 1;
                }
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        swamp_groups = swamp_count,
        "place_mongroups: swamp monsters placed"
    );

    // === 3. River/lake monsters — 7×7 sliding window =======================

    let mut river_count = 0usize;

    for y in (0..OMAP_DIM).step_by(WINDOW_SIZE as usize) {
        for x in (0..OMAP_DIM).step_by(WINDOW_SIZE as usize) {
            let mut water_tiles = 0usize;
            let mut center_is_water = false;

            for wy in 0..WINDOW_SIZE {
                for wx in 0..WINDOW_SIZE {
                    let tx = x + wx;
                    let ty = y + wy;
                    let flags = ter_flags(tx, ty);
                    if flags.contains(TerrainFlags::LAKE) || flags.contains(TerrainFlags::RIVER) {
                        water_tiles += 1;
                        if wx == WINDOW_SIZE / 2 && wy == WINDOW_SIZE / 2 {
                            center_is_water = true;
                        }
                    }
                }
            }

            if water_tiles >= WATER_THRESHOLD && center_is_water {
                let cx = x + WINDOW_SIZE / 2;
                let cy = y + WINDOW_SIZE / 2;
                if cx < OMAP_DIM && cy < OMAP_DIM {
                    commands.spawn(MonsterGroup {
                        group_type: "GROUP_RIVER".into(),
                        population: rng.range_i32(3, 6) as u32,
                        omt_x: cx,
                        omt_y: cy,
                        z: 0,
                    });
                    river_count += 1;
                }
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        river_groups = river_count,
        "place_mongroups: river/lake monsters placed"
    );

    // === 4. Ocean monsters — 7×7 sliding window ============================

    let mut ocean_deep_count = 0usize;
    let mut ocean_shore_count = 0usize;

    let global_base_x = config.om_x * OMAP_DIM;
    let global_base_y = config.om_y * OMAP_DIM;

    for y in (0..OMAP_DIM).step_by(WINDOW_SIZE as usize) {
        for x in (0..OMAP_DIM).step_by(WINDOW_SIZE as usize) {
            let mut ocean_tiles = 0usize;
            let mut center_is_ocean = false;
            let mut is_deep = false;

            for wy in 0..WINDOW_SIZE {
                for wx in 0..WINDOW_SIZE {
                    let tx = x + wx;
                    let ty = y + wy;
                    let flags = ter_flags(tx, ty);
                    if flags.contains(TerrainFlags::OCEAN) {
                        ocean_tiles += 1;
                        if wx == WINDOW_SIZE / 2 && wy == WINDOW_SIZE / 2 {
                            center_is_ocean = true;
                            // Classify deep vs shore using noise
                            let noise = ocean_noise_at(
                                global_base_x + tx,
                                global_base_y + ty,
                                config.noise_seed,
                            );
                            is_deep = noise > 0.3;
                        }
                    }
                }
            }

            if ocean_tiles >= WATER_THRESHOLD && center_is_ocean {
                let cx = x + WINDOW_SIZE / 2;
                let cy = y + WINDOW_SIZE / 2;
                if cx < OMAP_DIM && cy < OMAP_DIM {
                    if is_deep {
                        commands.spawn(MonsterGroup {
                            group_type: "GROUP_OCEAN_DEEP".into(),
                            population: rng.range_i32(3, 8) as u32,
                            omt_x: cx,
                            omt_y: cy,
                            z: 0,
                        });
                        ocean_deep_count += 1;
                    } else {
                        commands.spawn(MonsterGroup {
                            group_type: "GROUP_OCEAN_SHORE".into(),
                            population: rng.range_i32(2, 5) as u32,
                            omt_x: cx,
                            omt_y: cy,
                            z: 0,
                        });
                        ocean_shore_count += 1;
                    }
                }
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        ocean_deep = ocean_deep_count,
        ocean_shore = ocean_shore_count,
        "place_mongroups: ocean monsters placed"
    );

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        total_hordes = horde_count,
        total_swamp = swamp_count,
        total_river = river_count,
        total_ocean_deep = ocean_deep_count,
        total_ocean_shore = ocean_shore_count,
        "place_mongroups: complete"
    );
}
