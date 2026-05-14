//! Step: Place monster groups based on terrain and city proximity.
//!
//! Full port of CDDA's `overmap::place_mongroups()` (overmap.cpp L3448–3649).
//! Covers:
//! - City zombie hordes (road-aligned, density-scaled)
//! - Swamp monsters (7×7 scan for forest_water)
//! - River/lake monsters (7×7 scan for water tiles)
//! - Ocean monsters (deep vs shore, noise-gated)

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use crate::steps::oceans::calculate_ocean_gradient;
use bevy_ecs::prelude::*;
use cdda_noise;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

/// A placed monster group entity.
#[derive(Component)]
pub struct MonsterGroup {
    pub group_type: String, // "GROUP_ZOMBIE_HORDE", "GROUP_SWAMP", etc.
    pub population: u32,
    pub omt_x: i32,
    pub omt_y: i32,
    pub z: i32,
}

/// Place monster groups based on terrain and city proximity.
///
/// # Algorithm (port of `overmap::place_mongroups`)
///
/// 1. **City zombie hordes** — on road tiles near cities. Population scales
///    with city size. Uses submap-aligned quadrant distribution.
/// 2. **Swamp monsters** — 7×7 sliding window over forest_water tiles.
///    Places GROUP_SWAMP when ≥25 swamp tiles in window.
/// 3. **River/lake monsters** — 7×7 window over lake/river tiles.
///    Places GROUP_RIVER when ≥25 water tiles and center is water.
/// 4. **Ocean monsters** — 7×7 window over ocean tiles. Deep ocean
///    (noise+gated) gets GROUP_OCEAN_DEEP, shallow gets GROUP_OCEAN_SHORE.
pub fn place_mongroups(
    mut commands: Commands,
    chunks: Query<(&ChunkPosition, &OvermapChunk)>,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 13);

    // Build a dense grid of terrain type indices for z=0.
    let mut terrain_grid = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    terrain_grid[gx][gy] = chunk.get(lx, ly).type_index();
                }
            }
        }
    }

    let forest_water_index = registry
        .handle_by_id("forest_water")
        .map(|h| h.type_index())
        .unwrap_or(0);

    // ------------------------------------------------------------------
    // 1. City zombie hordes (CDDA L3452–3546)
    // ------------------------------------------------------------------
    place_city_hordes(&mut commands, &cities, &terrain_grid, &registry, &mut rng);

    // ------------------------------------------------------------------
    // 2. Swamp monsters (CDDA L3548–3575)
    // ------------------------------------------------------------------
    if settings.place_swamps {
        place_swamp_groups(&mut commands, &terrain_grid, forest_water_index, &mut rng);
    }

    // ------------------------------------------------------------------
    // 3. River / lake monsters (CDDA L3577–3601)
    // ------------------------------------------------------------------
    place_river_lake_groups(&mut commands, &terrain_grid, &registry, &mut rng);

    // ------------------------------------------------------------------
    // 4. Ocean monsters (CDDA L3602–3649)
    // ------------------------------------------------------------------
    if settings.ocean_start.iter().any(|s| s.is_some()) {
        place_ocean_groups(
            &mut commands,
            &terrain_grid,
            &registry,
            &settings,
            &config,
            &mut rng,
        );
    }

    info!(
        "Mongroups placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}

// ---------------------------------------------------------------------------
// City zombie hordes
// ---------------------------------------------------------------------------

/// Place zombie hordes on road tiles near cities.
///
/// Port of CDDA L3452–3546. Population = `city.size * scalar * density`.
/// Hordes are distributed across submaps on road tiles, with closer
/// submaps getting more hordes.
fn place_city_hordes(
    commands: &mut Commands,
    cities: &Query<&City>,
    terrain_grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
    rng: &mut XorShiftRng,
) {
    // CDDA default scalars.
    let city_spawn_scalar: f32 = 80.0;
    let spawn_density: f32 = 1.0;
    let city_spawn_spread: f32 = 1.5;

    // Identify the road type index for filtering.
    let road_index = registry
        .handle_by_id("road_nesw")
        .map(|h| h.type_index())
        .unwrap_or(0);

    for city in cities {
        let size = city.size as i32;
        let desired_zombies = (size as f32 * city_spawn_scalar * spawn_density) as i32;
        if desired_zombies <= 0 {
            continue;
        }

        let city_effective_radius = (size as f32 * city_spawn_spread) as i32;
        let city_distance_increment = (city_effective_radius as f32 / 4.0).ceil() as i32;

        // Collect submap positions on roads within the effective radius.
        let mut submap_list: Vec<(i32, i32)> = Vec::new();

        for dy in -city_effective_radius..=city_effective_radius {
            for dx in -city_effective_radius..=city_effective_radius {
                let wx = city.omt_x + dx;
                let wy = city.omt_y + dy;
                if wx < 2 || wx >= OMAP_DIM - 2 || wy < 2 || wy >= OMAP_DIM - 2 {
                    continue;
                }
                let ct = terrain_grid[wx as usize][wy as usize];
                // Only place on roads (matching CDDA's oter_type_road check).
                if ct != road_index {
                    continue;
                }

                let dist = dx.abs().max(dy.abs());
                let new_size = 4 - (dist / city_distance_increment.max(1));
                if new_size <= 0 {
                    continue;
                }

                // Submap-aligned quadrants for better distribution.
                // Each OMT maps to 4 submap positions.
                let sm_x = wx * 2;
                let sm_y = wy * 2;
                let mut local: Vec<(i32, i32)> = vec![
                    (sm_x, sm_y),
                    (sm_x + 1, sm_y),
                    (sm_x, sm_y + 1),
                    (sm_x + 1, sm_y + 1),
                ];

                // Shuffle and prune.
                shuffle_slice(&mut local, rng);
                local.truncate(new_size as usize);
                submap_list.extend(local);
            }
        }

        if submap_list.is_empty() {
            continue;
        }

        // Distribute zombies across submaps.
        let mut remaining = desired_zombies;
        while remaining > 0 {
            shuffle_slice(&mut submap_list, rng);
            for &(sm_x, sm_y) in &submap_list {
                if remaining <= 0 {
                    break;
                }
                let pop = remaining.min(10) as u32;
                commands.spawn(MonsterGroup {
                    group_type: "GROUP_ZOMBIE_HORDE".into(),
                    population: pop,
                    omt_x: sm_x,
                    omt_y: sm_y,
                    z: 0,
                });
                remaining -= 10;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Swamp monster groups
// ---------------------------------------------------------------------------

/// Place swamp monster groups using a 7×7 sliding window.
///
/// Port of CDDA L3548–3575.
fn place_swamp_groups(
    commands: &mut Commands,
    terrain_grid: &[[u32; 180]; 180],
    forest_water_index: u32,
    rng: &mut XorShiftRng,
) {
    // Stride by 7, scan 7×7 window centered on each sample point.
    for x in (3..OMAP_DIM as i32 - 3).step_by(7) {
        for y in (3..OMAP_DIM as i32 - 3).step_by(7) {
            let mut swamp_count = 0i32;
            for sx in x - 3..=x + 3 {
                for sy in y - 3..=y + 3 {
                    if terrain_grid[sx as usize][sy as usize] == forest_water_index {
                        swamp_count += 2;
                    }
                }
            }
            if swamp_count >= 25 {
                let pop = rng.range_i32(swamp_count * 8, swamp_count * 25) as u32;
                if pop > 0 {
                    commands.spawn(MonsterGroup {
                        group_type: "GROUP_SWAMP".into(),
                        population: pop,
                        omt_x: x,
                        omt_y: y,
                        z: 0,
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// River / lake monster groups
// ---------------------------------------------------------------------------

/// Place river/lake monster groups using a 7×7 sliding window.
///
/// Port of CDDA L3577–3601.
fn place_river_lake_groups(
    commands: &mut Commands,
    terrain_grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
    rng: &mut XorShiftRng,
) {
    for x in (3..OMAP_DIM as i32 - 3).step_by(7) {
        for y in (3..OMAP_DIM as i32 - 3).step_by(7) {
            let mut river_count = 0i32;
            for sx in x - 3..=x + 3 {
                for sy in y - 3..=y + 3 {
                    let handle = TerrainHandle::new(terrain_grid[sx as usize][sy as usize], 0);
                    let flags = registry.flags_for(handle);
                    if flags.contains(TerrainFlags::LAKE) || flags.contains(TerrainFlags::RIVER) {
                        river_count += 1;
                    }
                }
            }
            if river_count >= 25 {
                // CDDA: center must also be water.
                let center = TerrainHandle::new(terrain_grid[x as usize][y as usize], 0);
                let center_flags = registry.flags_for(center);
                if center_flags.contains(TerrainFlags::LAKE)
                    || center_flags.contains(TerrainFlags::RIVER)
                {
                    let pop = rng.range_i32(river_count * 8, river_count * 25) as u32;
                    if pop > 0 {
                        commands.spawn(MonsterGroup {
                            group_type: "GROUP_RIVER".into(),
                            population: pop,
                            omt_x: x,
                            omt_y: y,
                            z: 0,
                        });
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Ocean monster groups
// ---------------------------------------------------------------------------

/// Place ocean monster groups (deep vs shore) using 7×7 window + noise gate.
///
/// Port of CDDA L3602–3649.
fn place_ocean_groups(
    commands: &mut Commands,
    terrain_grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
    settings: &OvermapRegionSettings,
    config: &OvermapGenConfig,
    rng: &mut XorShiftRng,
) {
    let ocean_noise_threshold = settings.ocean_noise_threshold;
    let deep_threshold = ocean_noise_threshold * 1.25; // DEEP_OCEAN_THRESHOLD_ADJUST
    let seed = config.noise_seed;

    for x in (3..OMAP_DIM as i32 - 3).step_by(7) {
        for y in (3..OMAP_DIM as i32 - 3).step_by(7) {
            let mut ocean_count = 0i32;
            for sx in x - 3..=x + 3 {
                for sy in y - 3..=y + 3 {
                    let handle = TerrainHandle::new(terrain_grid[sx as usize][sy as usize], 0);
                    if registry.flags_for(handle).contains(TerrainFlags::OCEAN) {
                        ocean_count += 1;
                    }
                }
            }
            if ocean_count < 25 {
                continue;
            }

            // Determine deep vs shore using ocean noise + gradient.
            let grad = calculate_ocean_gradient((x, y), config.om_x, config.om_y, settings);
            if grad == 0.0 {
                continue;
            }
            let noise_val = cdda_noise::ocean_noise_at(x, y, seed);
            let is_deep = noise_val + grad > deep_threshold;

            let pop = rng.range_i32(ocean_count * 8, ocean_count * 25) as u32;
            if pop > 0 {
                commands.spawn(MonsterGroup {
                    group_type: if is_deep {
                        "GROUP_OCEAN_DEEP".into()
                    } else {
                        "GROUP_OCEAN_SHORE".into()
                    },
                    population: pop,
                    omt_x: x,
                    omt_y: y,
                    z: 0,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fisher-Yates shuffle on a mutable slice using an XorShiftRng.
fn shuffle_slice<T>(slice: &mut [T], rng: &mut XorShiftRng) {
    for i in (1..slice.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        slice.swap(i, j);
    }
}
