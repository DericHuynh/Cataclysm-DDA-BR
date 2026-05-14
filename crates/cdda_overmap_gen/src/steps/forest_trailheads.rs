//! Step 7b: Forest trailhead placement.
//!
//! Verbatim port of CDDA master's `overmap::place_forest_trailheads()`
//! (overmap.cpp L2000–2049).
//!
//! # Algorithm
//!
//! 1. Build a dense 180×180 tile grid for z=0.
//! 2. If `city_size ≤ 0`, return early.
//! 3. Scan each tile from row/col 2 to `OMAP_DIM - 2`.
//! 4. If the terrain's string ID starts with `"forest_trail_end"`:
//!    a. With `1-in-trailhead_chance` probability, check whether any
//!       tile within `trailhead_road_distance` (Chebyshev radius)
//!       contains `"road"` in its string ID.
//!    b. If both conditions are met, place `forest_trailhead` terrain
//!       at the trail end, preserving the tile's rotation.
//! 5. Write all changes back to chunks.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::closest_points_first;
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

/// Place forest trailheads at forest trail endpoints.
///
/// Verbatim port of `overmap::place_forest_trailheads()` from CDDA master
/// (overmap.cpp L2000–2049).
pub fn place_forest_trailheads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    // --- Early exit: no cities → no trailheads (C++ L2001–2002) ---
    let city_size = settings.city_size;
    if city_size <= 0 {
        return;
    }

    let trailhead_road_distance = settings.trailhead_road_distance;
    let trailhead_chance = settings.trailhead_chance;

    // Deterministic RNG, off-by-one seed to avoid colliding with other steps.
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 17);

    // Resolve the trailhead base handle (rotation 0).
    let trailhead_base = match registry.handle_by_id("forest_trailhead") {
        Some(h) => h,
        None => {
            info!("forest_trailhead not in registry — skipping trailhead placement");
            return;
        }
    };

    // --- Build a dense 180×180 grid of TerrainHandles for z=0 ---
    // This lets us read/write without fighting ECS borrow rules.
    let mut grid = [[TerrainHandle::NULL; 180]; 180];
    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
                    grid[gx][gy] = chunk.terrain[idx];
                }
            }
        }
    }

    // --- Scan for forest trail endpoints (C++ L2038–2048) ---
    let mut writes: Vec<(usize, usize, TerrainHandle)> = Vec::new();

    for x in 2..OMAP_DIM as usize - 2 {
        for y in 2..OMAP_DIM as usize - 2 {
            let handle = grid[x][y];
            if handle == TerrainHandle::NULL {
                continue;
            }

            // Check C++ `is_ot_match("forest_trail_end", oter, ot_match_type::prefix)`.
            let Some(id) = registry.string_id_for(handle) else {
                continue;
            };
            let matches_prefix = id == "forest_trail_end"
                || (id.starts_with("forest_trail_end")
                    && id.as_bytes().get("forest_trail_end".len()) == Some(&b'_'));

            if !matches_prefix {
                continue;
            }

            // --- try_place_trailhead_special (C++ L2012–2020, inlined) ---

            // 1-in-trailhead_chance (C++ L2016)
            if !rng.one_in(trailhead_chance) {
                continue;
            }

            // trailhead_close_to_road (C++ L2004–2009)
            let pos = (x as i32, y as i32);
            let mut close = false;
            for &(px, py) in &closest_points_first(pos, trailhead_road_distance) {
                if px < 0 || px >= 180 || py < 0 || py >= 180 {
                    continue;
                }
                if is_ot_match("road", grid[px as usize][py as usize], &registry, OtMatchType::Contains) {
                    close = true;
                    break;
                }
            }
            if !close {
                continue;
            }

            // Place trailhead with the same rotation as the trail-end tile.
            let dir = handle.rotation();
            let trailhead_handle = TerrainHandle::new(trailhead_base.type_index(), dir);
            writes.push((x, y, trailhead_handle));
        }
    }

    if writes.is_empty() {
        return;
    }

    // --- Write trailhead terrain back to chunks (C++: place_special writes) ---
    let write_count = writes.len();
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, h) in &writes {
            let lx = wx as i32 - ox;
            let ly = wy as i32 - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM as usize + lx as usize;
                if new_terrain[idx] != h {
                    new_terrain[idx] = h;
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

    info!("Forest trailheads placed: {}", write_count);
}
