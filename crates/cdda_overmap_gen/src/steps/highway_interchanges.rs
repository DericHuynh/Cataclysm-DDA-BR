//! Highway interchange placement and highway finalization.
//!
//! Port of C++ `place_highway_interchanges()` (overmap_highway.cpp L1130-1155)
//! and `finalize_highways()` (overmap_highway.cpp L931-971).

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Spacing between interchanges along a highway, in OMT tiles.
const INTERCHANGE_SPACING: i32 = OMAP_DIM / 4; // 45
const INTERCHANGE_VARIANCE: i32 = INTERCHANGE_SPACING / 10; // 4

// ---------------------------------------------------------------------------
// place_highway_interchanges
// ---------------------------------------------------------------------------

/// Place 4-way interchanges along highway paths at randomized intervals.
///
/// Port of C++ `place_highway_interchanges()` (overmap_highway.cpp L1130-1155).
///
/// Algorithm:
/// 1. Build terrain grid + highway presence grid from z=0 chunks.
/// 2. Walk highway tiles in row-major order with a spacing counter.
/// 3. When the counter hits 0, place an interchange and reset with random variance.
pub fn place_highway_interchanges(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
    config: Res<OvermapGenConfig>,
) {
    // --- Resolve terrain handles -------------------------------------------
    let interchange_handle = registry
        .handle_by_id("hiway_4way")
        .or_else(|| registry.handle_by_id("highway_4way"))
        .map(|h| h.0);
    let Some(interchange_raw) = interchange_handle else {
        info!("place_highway_interchanges: no hiway_4way/highway_4way in registry, skipping");
        return;
    };

    // --- Build highway presence grid from z=0 chunks -----------------------
    let omap_size = OMAP_DIM as usize;
    let mut is_highway = vec![false; omap_size * omap_size];

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
                    let handle = chunk.get(lx as u8, ly as u8);
                    if registry.flags_for(handle).contains(TerrainFlags::HIGHWAY) {
                        is_highway[(gy as usize) * omap_size + (gx as usize)] = true;
                    }
                }
            }
        }
    }

    // --- Interchange placement via row-major walk with spacing counter ------
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 13);
    let mut node_count =
        INTERCHANGE_SPACING + rng.range_i32(-INTERCHANGE_VARIANCE, INTERCHANGE_VARIANCE);
    let mut tile_writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();

    // Row-major scan
    for y in 0..OMAP_DIM {
        for x in 0..OMAP_DIM {
            if !is_highway[y as usize * omap_size + x as usize] {
                continue;
            }

            if node_count == 0 {
                tile_writes.push((x, y, TerrainHandle(interchange_raw)));
                node_count = INTERCHANGE_SPACING
                    + rng.range_i32(-INTERCHANGE_VARIANCE, INTERCHANGE_VARIANCE);
            }

            if node_count > 0 {
                node_count -= 1;
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        interchanges = tile_writes.len(),
        "place_highway_interchanges: placed"
    );

    // --- Write back to chunks -----------------------------------------------
    flush_tile_writes(&chunks, &par_commands, &tile_writes);
}

// ---------------------------------------------------------------------------
// finalize_highways
// ---------------------------------------------------------------------------

/// Replace highway tiles adjacent to water with bridge terrain.
///
/// Port of C++ `finalize_highways()` (overmap_highway.cpp L931-971).
///
/// Algorithm:
/// 1. Build terrain grid and highway presence grid from z=0 chunks.
/// 2. For each highway tile, check 4 cardinal neighbours for water.
/// 3. Replace highway-with-water-neighbour tiles with bridge terrain.
///
/// Bridge orientation:
/// - Water on east or west sides → bridge_ns (crosses N-S)
/// - Water on north or south sides → bridge_ew (crosses E-W)
/// - Water on both axes → check highway neighbour connectivity
pub fn finalize_highways(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
    config: Res<OvermapGenConfig>,
) {
    // --- Resolve terrain handles -------------------------------------------
    let bridge_ns_raw = registry
        .handle_by_id("hiway_bridge_ns")
        .or_else(|| registry.handle_by_id("highway_bridge_ns"))
        .or_else(|| registry.handle_by_id("bridge_ns"))
        .map(|h| h.0);
    let bridge_ew_raw = registry
        .handle_by_id("hiway_bridge_ew")
        .or_else(|| registry.handle_by_id("highway_bridge_ew"))
        .or_else(|| registry.handle_by_id("bridge_ew"))
        .map(|h| h.0);

    let (Some(bridge_ns), Some(bridge_ew)) = (bridge_ns_raw, bridge_ew_raw) else {
        info!("finalize_highways: bridge terrain not in registry, skipping");
        return;
    };

    // --- Build terrain + highway presence grid -----------------------------
    let omap_size = OMAP_DIM as usize;
    let mut grid = vec![TerrainHandle::NULL; omap_size * omap_size];
    let mut is_highway = vec![false; omap_size * omap_size];

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
                    let idx = (gy as usize) * omap_size + (gx as usize);
                    let handle = chunk.get(lx as u8, ly as u8);
                    grid[idx] = handle;
                    if registry.flags_for(handle).contains(TerrainFlags::HIGHWAY) {
                        is_highway[idx] = true;
                    }
                }
            }
        }
    }

    let ter = |x: i32, y: i32| -> TerrainHandle {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            grid[(y as usize) * omap_size + (x as usize)]
        } else {
            TerrainHandle::NULL
        }
    };

    let water_flags =
        TerrainFlags::from_bits(TerrainFlags::RIVER | TerrainFlags::LAKE | TerrainFlags::OCEAN);

    // --- Detect highway tiles adjacent to water ----------------------------
    let mut tile_writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();

    for y in 0..OMAP_DIM {
        for x in 0..OMAP_DIM {
            if !is_highway[y as usize * omap_size + x as usize] {
                continue;
            }

            // Check cardinal neighbours for water
            let n_water = registry.flags_for(ter(x, y - 1)).intersects(water_flags);
            let s_water = registry.flags_for(ter(x, y + 1)).intersects(water_flags);
            let e_water = registry.flags_for(ter(x + 1, y)).intersects(water_flags);
            let w_water = registry.flags_for(ter(x - 1, y)).intersects(water_flags);

            let ns_water = n_water || s_water;
            let ew_water = e_water || w_water;

            if !ns_water && !ew_water {
                continue; // No water neighbour → not a bridge
            }

            let bridge_terrain = if ns_water && !ew_water {
                // Water on north or south → bridge crosses E-W
                TerrainHandle(bridge_ew)
            } else if ew_water && !ns_water {
                // Water on east or west → bridge crosses N-S
                TerrainHandle(bridge_ns)
            } else {
                // Water on both axes — use highway neighbour connectivity
                let has_n_highway = *is_highway
                    .get((y - 1) as usize * omap_size + x as usize)
                    .unwrap_or(&false);
                let has_s_highway = *is_highway
                    .get((y + 1) as usize * omap_size + x as usize)
                    .unwrap_or(&false);
                let has_e_highway = *is_highway
                    .get(y as usize * omap_size + (x + 1) as usize)
                    .unwrap_or(&false);
                let has_w_highway = *is_highway
                    .get(y as usize * omap_size + (x - 1) as usize)
                    .unwrap_or(&false);

                if (has_n_highway || has_s_highway) && !(has_e_highway || has_w_highway) {
                    TerrainHandle(bridge_ns)
                } else {
                    TerrainHandle(bridge_ew)
                }
            };

            tile_writes.push((x, y, bridge_terrain));
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        bridges = tile_writes.len(),
        "finalize_highways: bridges placed"
    );

    flush_tile_writes(&chunks, &par_commands, &tile_writes);
}

// ---------------------------------------------------------------------------
// Helper: flush recorded tile writes back to chunk entities via par_iter
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
