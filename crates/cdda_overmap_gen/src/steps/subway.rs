//! Subway generation for underground z-levels (z < 0).
//!
//! Port of C++ `overmap::generate_sub()` (overmap.cpp L1060-1151).
//!
//! Algorithm:
//! 1. For each z-level from -1 down to -10:
//!    a. Scan every OMT tile at ground (z=0):
//!       - MANHOLE flag → sewers at z=-1
//!       - sub_station terrain → subway at z=-1/-2
//!    b. Connect sewer points via MST.
//!    c. Connect subway points via MST.
//! 2. Write all underground terrain back to chunks.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, line_between, ConnectionType,
};
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;

// ---------------------------------------------------------------------------
// generate_sub — system entry point
// ---------------------------------------------------------------------------

/// Generate subway and sewer tunnels below the overmap.
///
/// Port of C++ `overmap::generate_sub()` (overmap.cpp L1060-1151).
pub fn generate_sub(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
) {
    // --- Resolve terrain handles -------------------------------------------
    let sewer_isolated = registry.handle_by_id("sewer_isolated").map(|h| h.0);
    let sewer_ns = registry.handle_by_id("sewer_ns").map(|h| h.0);
    let sewer_sub_station = registry.handle_by_id("sewer_sub_station").map(|h| h.0);
    let subway_isolated = registry.handle_by_id("subway_isolated").map(|h| h.0);
    let subway_ns = registry.handle_by_id("subway_ns").map(|h| h.0);
    let sub_station_north = registry.handle_by_id("sub_station_north").map(|h| h.0);
    let _road_nesw_manhole = registry.handle_by_id("road_nesw_manhole").map(|h| h.0);

    // --- Collect city centers for subway station placement -----------------
    let city_centers: Vec<(i32, i32)> = cities.iter().map(|c| (c.omt_x, c.omt_y)).collect();

    // --- Build z=0 terrain grid --------------------------------------------
    let omap_size = OMAP_DIM as usize;
    let mut ground_grid = vec![TerrainHandle::NULL; omap_size * omap_size];

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
                    ground_grid[(gy as usize) * omap_size + (gx as usize)] =
                        chunk.get(lx as u8, ly as u8);
                }
            }
        }
    }

    // --- Collect sewer and subway points from z=0 scan ---------------------
    let mut sewer_points: Vec<(i32, i32)> = Vec::new();
    let mut subway_points: Vec<(i32, i32)> = Vec::new();

    // All terrain writes: (z, x, y, handle)
    let mut tile_writes: Vec<(i32, i32, i32, TerrainHandle)> = Vec::new();

    for y in 0..OMAP_DIM {
        for x in 0..OMAP_DIM {
            let ground_handle = ground_grid[(y as usize) * omap_size + (x as usize)];
            let flags = registry.flags_for(ground_handle);

            // MANHOLE flag → sewer at z=-1
            if flags.contains(TerrainFlags::MANHOLE) {
                if let Some(sh) = sewer_isolated {
                    tile_writes.push((-1, x, y, TerrainHandle(sh)));
                    sewer_points.push((x, y));
                }
            }

            // sub_station terrain → subway infrastructure
            if sub_station_north.is_some() {
                if is_ot_match("sub_station", ground_handle, &registry, OtMatchType::Prefix) {
                    // z=-1: sewer_sub_station
                    if let Some(sss) = sewer_sub_station {
                        tile_writes.push((-1, x, y, TerrainHandle(sss)));
                        sewer_points.push((x, y));
                    }

                    // z=-2: subway_isolated + 3 adjacent tiles
                    if subway_isolated.is_some() {
                        tile_writes.push((-2, x, y, TerrainHandle(subway_isolated.unwrap())));
                        subway_points.push((x, y));
                        // Place 3 adjacent subway tiles for the station footprint
                        for &(dx, dy) in &[(1, 0), (0, 1), (1, 1)] {
                            let sx = x + dx;
                            let sy = y + dy;
                            if sx >= 0 && sx < OMAP_DIM && sy >= 0 && sy < OMAP_DIM {
                                tile_writes.push((
                                    -2,
                                    sx,
                                    sy,
                                    TerrainHandle(subway_isolated.unwrap()),
                                ));
                                subway_points.push((sx, sy));
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Also add city-adjacent subway stations -----------------------------
    for &(cx, cy) in &city_centers {
        // Check a 3×3 area around the city center for subway placement
        for dy in -1..=1 {
            for dx in -1..=1 {
                let tx = cx + dx;
                let ty = cy + dy;
                if tx < 4 || tx >= OMAP_DIM - 4 || ty < 4 || ty >= OMAP_DIM - 4 {
                    continue;
                }
                // Only add if not already a subway point
                if !subway_points.contains(&(tx, ty)) {
                    if subway_isolated.is_some() {
                        subway_points.push((tx, ty));
                    }
                }
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        sewer_points = sewer_points.len(),
        subway_points = subway_points.len(),
        "generate_sub: points collected"
    );

    // --- Connect sewer points via MST (z=-1) --------------------------------
    if sewer_points.len() >= 2 {
        let mut rng = XorShiftRng::new(config.noise_seed as u64 + 23);
        let sewer_ns_handle = sewer_ns.map(TerrainHandle);

        connect_closest_points(
            &sewer_points,
            -1,
            ConnectionType::Sewer,
            &mut rng,
            |from, to, z, _ct| {
                let line = line_between(from, to);
                for &(lx, ly) in &line {
                    if inbounds_omt((lx, ly)) {
                        if let Some(ns) = sewer_ns_handle {
                            tile_writes.push((z, lx, ly, ns));
                        }
                    }
                }
            },
        );
    }

    // --- Connect subway points via MST (z=-2) -------------------------------
    if subway_points.len() >= 2 {
        let mut rng = XorShiftRng::new(config.noise_seed as u64 + 29);
        let subway_ns_handle = subway_ns.map(TerrainHandle);

        connect_closest_points(
            &subway_points,
            -2,
            ConnectionType::Subway,
            &mut rng,
            |from, to, z, _ct| {
                let line = line_between(from, to);
                for &(lx, ly) in &line {
                    if inbounds_omt((lx, ly)) {
                        if let Some(ns) = subway_ns_handle {
                            tile_writes.push((z, lx, ly, ns));
                        }
                    }
                }
            },
        );
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        total_writes = tile_writes.len(),
        "generate_sub: connections complete, writing chunks"
    );

    // --- Write underground terrain to chunks -------------------------------
    flush_underground_tile_writes(&chunks, &par_commands, &tile_writes);
}

// ---------------------------------------------------------------------------
// Helper: flush underground tile writes to chunks via par_iter
// ---------------------------------------------------------------------------

fn flush_underground_tile_writes(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: &ParallelCommands,
    tile_writes: &[(i32, i32, i32, TerrainHandle)], // (z, x, y, handle)
) {
    if tile_writes.is_empty() {
        return;
    }
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        let z = chunk_pos.z.0 as i32;

        let local_ox = (chunk_pos.chunk_x as i32) * (CHUNK_DIM as i32);
        let local_oy = (chunk_pos.chunk_y as i32) * (CHUNK_DIM as i32);

        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wz, wx, wy, handle) in tile_writes {
            if wz != z {
                continue;
            }
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
