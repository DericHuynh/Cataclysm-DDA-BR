//! Step 4b: Connect cities with railroads via MST-based pathfinding.
//!
//! Port of CDDA master's `overmap::place_railroads()` (overmap.cpp L2227-2297).
//!
//! Similar to roads but:
//! - Railroad exit points are placed around city centers (random within `city.size * 4`).
//! - Uses `ConnectionType::Railroad`.
//! - Places railroad terrain instead of road terrain.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, line_between, ConnectionType,
};
use cdda_overmap::direction::OmDirection;
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_overmap::Rng;
use tracing::info;

// ---------------------------------------------------------------------------
// Helpers (shared with roads.rs logic)
// ---------------------------------------------------------------------------

/// Return all OMT points on the edge of the overmap for `dir`, excluding
/// `margin` tiles from each corner.
fn get_border(dir: OmDirection, margin: i32) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    let max = OMAP_DIM;
    match dir {
        OmDirection::North => {
            for x in margin..(max - margin) {
                pts.push((x, 0));
            }
        }
        OmDirection::South => {
            for x in margin..(max - margin) {
                pts.push((x, max - 1));
            }
        }
        OmDirection::East => {
            for y in margin..(max - margin) {
                pts.push((max - 1, y));
            }
        }
        OmDirection::West => {
            for y in margin..(max - margin) {
                pts.push((0, y));
            }
        }
        OmDirection::Invalid => {}
    }
    pts
}

/// Return all integer points within a Chebyshev radius of `center`.
fn points_in_radius(center: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let p = (center.0 + dx, center.1 + dy);
            if inbounds_omt(p) {
                pts.push(p);
            }
        }
    }
    pts
}

/// Check whether a terrain handle represents a water-type tile (river, lake, ocean).
fn is_water(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

// ---------------------------------------------------------------------------
// build_rail_segment — places rail tiles along the line between two points
// ---------------------------------------------------------------------------

/// Place railroad tiles along the straight-line path from `from` to `to`.
fn build_rail_segment(
    from: (i32, i32),
    to: (i32, i32),
    z: i32,
    _connection_type: ConnectionType,
    _registry: &TerrainRegistry,
    grid: &mut [[bool; 180]],
) {
    if z != 0 {
        return;
    }
    let path = line_between(from, to);
    for &(x, y) in &path {
        if !inbounds_omt((x, y)) {
            continue;
        }
        grid[x as usize][y as usize] = true;
    }
}

// ---------------------------------------------------------------------------
// place_railroads system
// ---------------------------------------------------------------------------

/// Connect cities with railroads.
///
/// # Algorithm (port of `overmap::place_railroads`)
///
/// 1. Generate 2–3 border exit points to ensure railroad continuity across overmaps.
/// 2. For each city, pick a random point within `city.size * 4` radius.
/// 3. Assemble railroad_points: exit points + city rail points.
/// 4. Call `connect_closest_points` with `ConnectionType::Railroad`.
/// 5. Write railroad tiles back to chunks, only overwriting FIELD and FOREST tiles.
pub fn place_railroads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    // no railroads if there are no cities (matching C++ behavior)
    // CDDA: no railroads if there are no cities
    let op_city_size = settings.city_size;
    if op_city_size <= 0 || !settings.place_railroads {
        return;
    }

    let city_count = cities.iter().count();
    if city_count == 0 {
        return;
    }

    // --- 1. Generate border exit points ---
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);
    let mut rails_out: Vec<(i32, i32)> = Vec::new();

    // At least 3 exit points for cross-overmap continuity.
    if rails_out.len() < 3 {
        let mut dirs: Vec<OmDirection> = vec![
            OmDirection::North,
            OmDirection::East,
            OmDirection::South,
            OmDirection::West,
        ];
        for i in (1..dirs.len()).rev() {
            let j = rng.random_usize(i + 1);
            dirs.swap(i, j);
        }

        for &dir in &dirs {
            let mut border = get_border(dir, 10);
            for i in (1..border.len()).rev() {
                let j = rng.random_usize(i + 1);
                border.swap(i, j);
            }

            for &p in &border {
                let (gx, gy) = p;
                let cx = (gx / 32) as u8;
                let cy = (gy / 32) as u8;
                let lx = (gx % 32) as u8;
                let ly = (gy % 32) as u8;

                let mut is_river_collision = false;
                for (_entity, chunk_pos, chunk) in &chunks {
                    if chunk_pos.z.0 != 0 {
                        continue;
                    }
                    if chunk_pos.chunk_x == cx && chunk_pos.chunk_y == cy {
                        let idx = ly as usize * CHUNK_DIM + lx as usize;
                        let handle = chunk.terrain[idx];
                        if is_water(handle, &registry) {
                            is_river_collision = true;
                        }
                        break;
                    }
                }
                if !is_river_collision {
                    rails_out.push(p);
                    break;
                }
            }
            if rails_out.len() >= 3 {
                break;
            }
        }
    }

    // --- 2. Assemble railroad_points ---
    let mut rail_points: Vec<(i32, i32)> = Vec::new();
    rail_points.reserve(rails_out.len() + city_count);

    for &p in &rails_out {
        rail_points.push(p);
    }

    // Place railroads at random points around the center of each city.
    for city in cities.iter() {
        let radius = (city.size as i32).saturating_mul(4).max(1);
        let candidates = points_in_radius((city.omt_x, city.omt_y), radius);
        if candidates.is_empty() {
            // Fallback: use the city center itself.
            rail_points.push((city.omt_x, city.omt_y));
        } else {
            let idx = rng.random_usize(candidates.len());
            rail_points.push(candidates[idx]);
        }
    }

    for &p in &rail_points {
        tracing::trace!("Rail point: ({}, {})", p.0, p.1);
    }

    if rail_points.len() < 2 {
        return;
    }

    // --- 3. Build railroad network via MST ---
    let rail_handle = registry
        .handle_by_id("railroad")
        .unwrap_or(TerrainHandle::NULL);
    let rail_ns_handle = registry.rotate(rail_handle, 0); // north-south
    let rail_ew_handle = registry.rotate(rail_handle, 1); // east-west
    let rail_nesw_handle = registry
        .handle_by_id("railroad_nesw")
        .unwrap_or_else(|| registry.rotate(rail_handle, 3)); // fallback: use rotation

    let rail_ew_idx = rail_ew_handle.type_index();
    let rail_ns_idx = rail_ns_handle.type_index();
    let rail_nesw_idx = rail_nesw_handle.type_index();

    let mut grid = [[false; 180]; 180];

    // Mark rail point tiles.
    for &(x, y) in &rail_points {
        if x >= 0 && x < 180 && y >= 0 && y < 180 {
            grid[x as usize][y as usize] = true;
        }
    }

    // Build connections.
    connect_closest_points(
        &rail_points,
        0,
        ConnectionType::Railroad,
        &mut rng,
        |from, to, z, ct| {
            build_rail_segment(from, to, z, ct, &registry, &mut grid);
        },
    );

    // --- 4. Write grid back to chunks ---
    // Only overwrite FIELD and FOREST-type tiles. Never overwrite water or roads.
    let field_index = registry.field_index;
    let forest_index = registry.forest_index;
    let forest_thick_index = registry.forest_thick_index;
    let forest_water_index = registry.forest_water_index;
    let reg = &*registry;

    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 { return; }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..32 {
            for lx in 0u8..32 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= 180 || gy >= 180 || !grid[gx][gy] {
                    continue;
                }

                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let current = chunk.terrain[idx];
                let ct = current.type_index();

                // Skip water tiles.
                if is_water(current, reg) {
                    continue;
                }

                // Only overwrite field and forest-type tiles.
                let is_field = ct == field_index;
                let is_forest = ct == forest_index
                    || ct == forest_thick_index
                    || ct == forest_water_index;
                let is_rail = ct == rail_ew_idx || ct == rail_ns_idx || ct == rail_nesw_idx;

                if !is_field && !is_forest && !is_rail {
                    continue;
                }

                // Determine railroad tile orientation based on neighbors.
                let north = gy > 0 && grid[gx][gy - 1];
                let south = gy + 1 < 180 && grid[gx][gy + 1];
                let east = gx + 1 < 180 && grid[gx + 1][gy];
                let west = gx > 0 && grid[gx - 1][gy];

                let has_ns = north || south;
                let has_ew = east || west;

                let handle = if has_ns && has_ew {
                    rail_nesw_handle
                } else if has_ew {
                    rail_ew_handle
                } else {
                    rail_ns_handle
                };
                new_terrain[idx] = handle;
                modified = true;
            }
        }
        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk { terrain: new_terrain });
            });
        }
    });

    info!(
        "Railroads placed: {} rail points, {} exit points for overmap ({}, {})",
        rail_points.len(),
        rails_out.len(),
        config.om_x,
        config.om_y
    );
}
