//! Step 7: Generate underground layers (sewers, subways).
//!
//! Port of CDDA master's `overmap::generate_sub()` (overmap.cpp L1060-1151).
//!
//! Called once; iterates z=-1 downward to z=-10 and places sewer/subway
//! tiles based on manholes and sub-stations at ground level.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{connect_closest_points, line_between, ConnectionType};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use tracing::info;

/// Generate underground layers (sewers, subways).
///
/// For each z-level from -1 down to -10:
/// 1. Scans ground-level (z=0) tiles for manholes and sub-stations.
/// 2. Places `sewer_isolated` below manholes, `sewer_sub_station` at z=-1
///    below sub-stations, and `subway_isolated` at z=-2 below sub-stations.
/// 3. Connects sewer and subway points into networks via MST pathfinding.
///
/// # Port notes
///
/// The C++ version is called once per z-level (`generate_sub(z)`) and
/// returns `true` to request the next level. This Rust port handles all
/// z-levels in a single system invocation since Bevy systems run once.
pub fn generate_sub(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    _cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
) {
    // Process each underground z-level sequentially.
    // Range: -1 down to -10 (inclusive).  Rust requires start <= end in
    // inclusive-range syntax, so we write (-10..=-1).rev().
    for z in (-10i8..=-1i8).rev() {
        let mut sewer_points: Vec<(i32, i32)> = Vec::new();
        let mut subway_points: Vec<(i32, i32)> = Vec::new();

        // ------------------------------------------------------------------
        // Build dense 180×180 grids for this z-level, the level above,
        // and ground level.  We pack raw u32 type indices for fast lookup.
        // ------------------------------------------------------------------
        let mut grid_z = [[0u32; 180]; 180];       // current z
        let mut grid_above = [[0u32; 180]; 180];   // z + 1
        let mut grid_ground = [[0u32; 180]; 180];  // z = 0

        for (chunk_pos, chunk) in &chunks {
            let (ox, oy) = chunk_pos.omt_origin();
            for ly in 0..CHUNK_DIM as u8 {
                for lx in 0..CHUNK_DIM as u8 {
                    let gx = (ox + lx as i32) as usize;
                    let gy = (oy + ly as i32) as usize;
                    if gx >= 180 || gy >= 180 {
                        continue;
                    }
                    let h = chunk.get(lx, ly);
                    if chunk_pos.z.0 == z {
                        grid_z[gx][gy] = h.0;
                    }
                    if chunk_pos.z.0 == z + 1 {
                        grid_above[gx][gy] = h.0;
                    }
                    if chunk_pos.z.0 == 0 {
                        grid_ground[gx][gy] = h.0;
                    }
                }
            }
        }

        // ------------------------------------------------------------------
        // Scan every OMT tile at ground level.
        //   • manhole → sewer_isolated below
        //   • sub_station → sewer_sub_station (z=-1) / subway_isolated (z=-2)
        // ------------------------------------------------------------------
        for x in 0..OMAP_DIM as usize {
            for y in 0..OMAP_DIM as usize {
                let ground = TerrainHandle(grid_ground[x][y]);
                let ground_flags = registry.flags_for(ground);

                // Manhole at z+1 → place sewer_isolated at our z-level.
                if ground_flags.contains(TerrainFlags::MANHOLE) {
                    if z == -1 {
                        // CDDA only places sewer_isolated directly below the
                        // manhole (i.e. at z=-1), not deeper.
                        if let Some(sewer) = registry.handle_by_id("sewer_isolated") {
                            place_in_chunk(&mut chunks, x as i32, y as i32, z, sewer);
                        }
                    }
                    sewer_points.push((x as i32, y as i32));
                }

                // Sub-station at ground level triggers specials below.
                if let Some(sub_station) = registry.handle_by_id("sub_station_north") {
                    let ground_type = ground.type_index();
                    if ground == sub_station || ground_type == sub_station.type_index() {
                        if z == -1 {
                            // Directly below sub-station: sewer sub-station room.
                            if let Some(sewer_sub) = registry.handle_by_id("sewer_sub_station") {
                                place_in_chunk(&mut chunks, x as i32, y as i32, z, sewer_sub);
                            }
                        } else if z == -2 {
                            // Two levels below: subway entrance.
                            if let Some(subway_iso) = registry.handle_by_id("subway_isolated") {
                                place_in_chunk(&mut chunks, x as i32, y as i32, z, subway_iso);
                            }
                            // Add three adjacent tiles for subway connectivity
                            // (CDDA adds the tile and its north/south neighbours).
                            subway_points.push((x as i32, y as i32 - 1));
                            subway_points.push((x as i32, y as i32));
                            subway_points.push((x as i32, y as i32 + 1));
                        }
                    }
                }
            }
        }

        // ------------------------------------------------------------------
        // Connect sewer points into a network (MST + optional loop edges).
        // ------------------------------------------------------------------
        if sewer_points.len() >= 2 {
            let mut rng = XorShiftRng::new(config.noise_seed as u64 + 100 + z as u64);
            connect_closest_points(
                &sewer_points,
                z as i32,
                ConnectionType::Sewer,
                &mut rng,
                |from, to, z_conn, _ct| {
                    if let Some(sewer) = registry.handle_by_id("sewer_ns") {
                        let path = line_between(from, to);
                        for &(px, py) in &path {
                            place_in_chunk(
                                &mut chunks, px, py, z_conn as i8, sewer,
                            );
                        }
                    }
                },
            );
        }

        // ------------------------------------------------------------------
        // Connect subway points into a network.
        // ------------------------------------------------------------------
        if subway_points.len() >= 2 {
            let mut rng = XorShiftRng::new(config.noise_seed as u64 + 200 + z as u64);
            connect_closest_points(
                &subway_points,
                z as i32,
                ConnectionType::Subway,
                &mut rng,
                |from, to, z_conn, _ct| {
                    if let Some(subway) = registry.handle_by_id("subway_ns") {
                        let path = line_between(from, to);
                        for &(px, py) in &path {
                            place_in_chunk(
                                &mut chunks, px, py, z_conn as i8, subway,
                            );
                        }
                    }
                },
            );
        }
    }

    info!(
        "Underground generated for overmap ({}, {})",
        config.om_x, config.om_y
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Place a terrain handle at world-absolute OMT coordinates in the chunk
/// that contains the point at the given z-level.
fn place_in_chunk(
    chunks: &mut Query<(&ChunkPosition, &mut OvermapChunk)>,
    omt_x: i32,
    omt_y: i32,
    z: i8,
    handle: TerrainHandle,
) {
    for (chunk_pos, mut chunk) in chunks {
        if chunk_pos.z.0 != z {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = omt_x - ox;
        let ly = omt_y - oy;
        if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
            chunk.set(lx as u8, ly as u8, handle);
            return;
        }
    }
}
