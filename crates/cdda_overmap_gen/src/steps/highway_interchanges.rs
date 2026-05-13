//! Step 6b: Highway interchange placement and finalization.
//!
//! Port of CDDA master's `overmap::place_highway_interchanges()` (overmap.cpp
//! L1130–1155) and `overmap::finalize_highways()` (overmap.cpp L931–971).
//!
//! # Algorithm — `place_highway_interchanges`
//!
//! 1. Scan all z=0 chunks for tiles with `TerrainFlags::HIGHWAY`.
//! 2. At regular spacing intervals (modulated by random variance), mark
//!    highway tiles as interchanges using hiway_4way / highway_4way terrain.
//!
//! # Algorithm — `finalize_highways`
//!
//! 1. Build a dense terrain grid from all z=0 chunks.
//! 2. Scan every highway tile.
//! 3. For each highway tile, check its 4 cardinal neighbours.
//! 4. If any neighbour is water (RIVER/LAKE/OCEAN), replace the highway tile
//!    with a bridge terrain so the tile set renders correctly.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use crate::pipeline::OvermapGenConfig;
use tracing::info;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Spacing between highway interchanges in OMT units.
const INTERCHANGE_SPACING: i32 = 20;
/// Random variance applied to interchange spacing.
const INTERCHANGE_VARIANCE: i32 = 5;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return `true` if a terrain handle represents water.
fn is_water(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

/// Build a dense 180x180 grid of TerrainHandle raw values from all z=0 chunks.
fn build_grid_from_chunks(
    chunks: &Query<(&ChunkPosition, &OvermapChunk)>,
) -> [[u32; 180]; 180] {
    let mut grid = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    grid[gx][gy] = chunk.get(lx, ly).0;
                }
            }
        }
    }
    grid
}

// ---------------------------------------------------------------------------
// place_highway_interchanges
// ---------------------------------------------------------------------------

/// Place highway interchange terrain at regular intervals on highway tiles.
///
/// # Algorithm (port of `overmap::place_highway_interchanges`)
///
/// Interchanges are placed at regular spacing intervals along highway tiles.
/// Each interchange is marked with the `hiway_4way` / `highway_4way` terrain
/// type which tells the tile renderer to draw an interchange junction.
///
/// Since we don't yet have the full highway-path vector data structure, we
/// walk highway tiles in spatial order and place interchanges at spacing
/// intervals with random variance.
pub fn place_highway_interchanges(
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    registry: Res<TerrainRegistry>,
    config: Res<OvermapGenConfig>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);

    // Look up interchange terrain handles.
    let interchange_handle = registry
        .handle_by_id("hiway_4way")
        .or_else(|| registry.handle_by_id("highway_4way"));

    let Some(interchange) = interchange_handle else {
        info!("Highway interchanges skipped: no hiway_4way/highway_4way in registry");
        return;
    };

    // Collect all highway tile positions in spatial order.
    let mut highway_tiles: Vec<(i32, i32)> = Vec::new();

    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let handle = chunk.get(lx, ly);
                let flags = registry.flags_for(handle);
                if flags.contains(TerrainFlags::HIGHWAY) {
                    highway_tiles.push((ox + lx as i32, oy + ly as i32));
                }
            }
        }
    }

    if highway_tiles.is_empty() {
        return;
    }

    // Sort for consistent spatial ordering.
    highway_tiles.sort_by_key(|&(x, y)| (y, x));

    // Determine spacing with variance.
    let spacing = INTERCHANGE_SPACING + rng.range_i32(-INTERCHANGE_VARIANCE, INTERCHANGE_VARIANCE);
    if spacing <= 0 {
        return;
    }

    let mut tiles_since_interchange = spacing / 2;
    let mut interchange_count = 0usize;

    for &(x, y) in &highway_tiles {
        tiles_since_interchange += 1;
        if tiles_since_interchange < spacing {
            continue;
        }
        tiles_since_interchange = 0;

        // Write interchange terrain into the appropriate chunk.
        for (chunk_pos, mut chunk) in &mut chunks {
            if chunk_pos.z.0 != 0 {
                continue;
            }
            let (ox, oy) = chunk_pos.omt_origin();
            let lx = x - ox;
            let ly = y - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let handle = chunk.get(lx as u8, ly as u8);
                if registry.flags_for(handle).contains(TerrainFlags::HIGHWAY) {
                    chunk.set(lx as u8, ly as u8, interchange);
                    interchange_count += 1;
                }
                break;
            }
        }
    }

    info!(
        "Highway interchanges placed: {} interchanges on {} highway tiles for overmap ({}, {})",
        interchange_count, highway_tiles.len(), config.om_x, config.om_y
    );
}

// ---------------------------------------------------------------------------
// finalize_highways
// ---------------------------------------------------------------------------

/// Finalize highway tiles: ensure bridge markers are present where highways
/// cross water.
///
/// # Algorithm (port of `overmap::finalize_highways`)
///
/// 1. Build a dense terrain grid from all z=0 chunks (immutable pass).
/// 2. Scan every z=0 highway tile with mutable access.
/// 3. For each highway tile, check its 4 cardinal neighbours in the grid.
/// 4. If any neighbour is water, replace the highway tile with a bridge
///    terrain handle. Also place the bridge at z=+1 for elevated rendering.
pub fn finalize_highways(
    chunks: Query<(&ChunkPosition, &OvermapChunk)>,
    mut chunks_mut: Query<(&ChunkPosition, &mut OvermapChunk)>,
    registry: Res<TerrainRegistry>,
    config: Res<OvermapGenConfig>,
) {
    // Build terrain grid from immutable query.
    let terrain_grid = build_grid_from_chunks(&chunks);

    // Look up bridge terrain handles.
    let bridge_ns = registry
        .handle_by_id("bridge_ns")
        .or_else(|| registry.handle_by_id("bridge"));
    let bridge_ew = registry
        .handle_by_id("bridge_ew")
        .or_else(|| registry.handle_by_id("bridge"));

    let hiway_bridge_ns = registry
        .handle_by_id("hiway_bridge_ns")
        .or_else(|| registry.handle_by_id("highway_bridge_ns"));
    let hiway_bridge_ew = registry
        .handle_by_id("hiway_bridge_ew")
        .or_else(|| registry.handle_by_id("highway_bridge_ew"));

    // Fall back to generic road bridge if highway-specific isn't available.
    let effective_bridge_ns = hiway_bridge_ns.or(bridge_ns);
    let effective_bridge_ew = hiway_bridge_ew.or(bridge_ew);

    let (Some(bridge_ns_h), Some(bridge_ew_h)) = (effective_bridge_ns, effective_bridge_ew) else {
        info!(
            "Highway finalize skipped: no bridge_ns/bridge_ew or hiway_bridge in registry"
        );
        return;
    };

    // Build a set of (x, y, bridge_handle) tuples to apply.
    struct BridgeCandidate {
        x: i32,
        y: i32,
        handle: TerrainHandle,
    }
    let mut candidates: Vec<BridgeCandidate> = Vec::new();

    // Scan using the immutable query to identify candidates.
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let handle = chunk.get(lx, ly);
                let flags = registry.flags_for(handle);
                if !flags.contains(TerrainFlags::HIGHWAY) {
                    continue;
                }

                let wx = ox + lx as i32;
                let wy = oy + ly as i32;

                // Check 4 cardinal neighbours for water.
                let neighbours: [(i32, i32); 4] = [
                    (wx, wy - 1),
                    (wx + 1, wy),
                    (wx, wy + 1),
                    (wx - 1, wy),
                ];

                let mut water_north = false;
                let mut water_south = false;
                let mut water_east = false;
                let mut water_west = false;

                for &(nx, ny) in &neighbours {
                    if nx < 0 || nx >= OMAP_DIM || ny < 0 || ny >= OMAP_DIM {
                        continue;
                    }
                    let nhandle = TerrainHandle(terrain_grid[nx as usize][ny as usize]);
                    if is_water(nhandle, &registry) {
                        match (nx - wx, ny - wy) {
                            (0, -1) => water_north = true,
                            (1, 0) => water_east = true,
                            (0, 1) => water_south = true,
                            (-1, 0) => water_west = true,
                            _ => {}
                        }
                    }
                }

                if !(water_north || water_south || water_east || water_west) {
                    continue;
                }

                let has_water_ns = water_north || water_south;
                let has_water_ew = water_east || water_west;

                let bridge_handle = if has_water_ns && has_water_ew {
                    // Water on both axes — prefer highway orientation.
                    let hwy_n = wy > 0 && {
                        let h = TerrainHandle(terrain_grid[wx as usize][wy as usize - 1]);
                        registry.flags_for(h).contains(TerrainFlags::HIGHWAY)
                    };
                    let hwy_s = wy + 1 < OMAP_DIM && {
                        let h = TerrainHandle(terrain_grid[wx as usize][wy as usize + 1]);
                        registry.flags_for(h).contains(TerrainFlags::HIGHWAY)
                    };
                    if hwy_n || hwy_s {
                        bridge_ns_h
                    } else {
                        bridge_ew_h
                    }
                } else if has_water_ew {
                    bridge_ew_h
                } else {
                    bridge_ns_h
                };

                candidates.push(BridgeCandidate { x: wx, y: wy, handle: bridge_handle });
            }
        }
    }

    // Apply bridge terrain to z=0 and z=+1 chunks.
    let mut bridge_count = 0usize;
    for candidate in &candidates {
        for (chunk_pos, mut chunk) in &mut chunks_mut {
            let z = chunk_pos.z.0;
            if z != 0 && z != 1 {
                continue;
            }
            let (ox, oy) = chunk_pos.omt_origin();
            let lx = candidate.x - ox;
            let ly = candidate.y - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                chunk.set(lx as u8, ly as u8, candidate.handle);
                if z == 0 {
                    bridge_count += 1;
                }
                break;
            }
        }
    }

    info!(
        "Highways finalized: {} bridges from {} candidates for overmap ({}, {})",
        bridge_count, candidates.len(), config.om_x, config.om_y
    );
}
