//! Step 6b: Highway interchange placement and highway finalization.
//!
//! Verbatim port of C++ `overmap::place_highway_interchanges()` (overmap_highway.cpp
//! L1130–1155) and `overmap::finalize_highways()` (overmap_highway.cpp L931–971),
//! adapted to Bevy ECS chunk-entity terrain storage.
//!
//! # Algorithm — `place_highway_interchanges`
//!
//! 1. Build a `[[u32; 180]; 180]` terrain grid from all z=0 chunks.
//! 2. Walk every tile; when we find a highway tile, decrement `node_count`.
//! 3. When `node_count` reaches 0, place `hiway_4way` terrain (the interchange)
//!    and reset `node_count` to `INTERCHANGE_SPACING`.
//! 4. Flush all writes back to chunk entities.
//!
//! Port of C++:
//! ```cpp
//! int node_count = HIGHWAY_INTERCHANGE_SPACING +
//!     rng(-HIGHWAY_INTERCHANGE_VARIANCE, HIGHWAY_INTERCHANGE_VARIANCE);
//! for (auto &node : path) {
//!     if (node.is_segment && node_count == 0) {
//!         place_special(*interchange, ...);
//!         node.is_interchange = true;
//!         node_count = HIGHWAY_INTERCHANGE_SPACING;
//!     }
//!     if (node_count > 0) node_count--;
//! }
//! ```
//!
//! # Algorithm — `finalize_highways`
//!
//! 1. Build a `[[u32; 180]; 180]` terrain grid from all z=0 chunks.
//! 2. Build a `[[bool; 180]; 180]` highway-presence grid from z=0 chunks.
//! 3. For each highway tile, check its 4 cardinal neighbours in the terrain grid.
//! 4. If any neighbour has water flags (RIVER|LAKE|OCEAN), replace the highway
//!    tile with the appropriate bridge terrain (hiway_bridge_ns or hiway_bridge_ew).
//! 5. Flush all writes back to chunk entities.
//!
//! Port of C++ `finalize_highways` which iterates over `Highway_path` nodes,
//! marks road segments for bridge specials, and places them. Since we lack the
//! `Highway_path` vector in this step, we use a grid-based equivalent: any
//! highway tile adjacent to water gets a bridge terrain.

use crate::pipeline::OvermapGenConfig;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

// ---------------------------------------------------------------------------
// Constants — match C++ HIGHWAY_INTERCHANGE_SPACING and VARIANCE
// ---------------------------------------------------------------------------

/// Spacing between highway interchanges in OMT units.
/// C++: `const int HIGHWAY_INTERCHANGE_SPACING = OMAPX / 4;`  (180/4 = 45)
const INTERCHANGE_SPACING: i32 = OMAP_DIM / 4;

/// Random variance applied to interchange spacing.
/// C++: `const int HIGHWAY_INTERCHANGE_VARIANCE = HIGHWAY_INTERCHANGE_SPACING / 10;`
const INTERCHANGE_VARIANCE: i32 = INTERCHANGE_SPACING / 10;

/// The number of cardinal directions to check for water adjacency.
const NUM_CARDINAL_DIRS: usize = 4;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return `true` if a terrain handle represents water (river, lake, or ocean).
#[inline]
fn terrain_is_water(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    flags.contains(TerrainFlags::RIVER)
        || flags.contains(TerrainFlags::LAKE)
        || flags.contains(TerrainFlags::OCEAN)
}

/// Return `true` if a terrain handle represents a road.
#[inline]
fn terrain_is_road(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).contains(TerrainFlags::ROAD)
}

/// Return `true` if a terrain handle represents a highway.
#[inline]
fn terrain_is_highway(handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    registry.flags_for(handle).contains(TerrainFlags::HIGHWAY)
}

/// Cardinal offset deltas: N, E, S, W.
const CARDINAL_OFFSETS: [(i32, i32); NUM_CARDINAL_DIRS] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

// ---------------------------------------------------------------------------
// Grid builder
// ---------------------------------------------------------------------------

/// Build a dense `[[u32; 180]; 180]` terrain grid from all z=0 chunks.
///
/// In C++ this is a flat `oter_id grid[OMAPX][OMAPY]` member of `overmap`.
/// In Bevy we reconstruct it from chunk entities for the duration of the step.
fn build_terrain_grid(chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>) -> [[u32; 180]; 180] {
    let mut grid = [[0u32; 180]; 180];
    for (_entity, chunk_pos, chunk) in chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < OMAP_DIM as usize && gy < OMAP_DIM as usize {
                    grid[gx][gy] = chunk.get(lx, ly).0;
                }
            }
        }
    }
    grid
}

/// Build a `[[bool; 180]; 180]` highway-presence grid from all z=0 chunks.
fn build_highway_presence_grid(
    chunks: &Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    registry: &TerrainRegistry,
) -> [[bool; 180]; 180] {
    let mut hgrid = [[false; 180]; 180];
    for (_entity, chunk_pos, chunk) in chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < OMAP_DIM as usize && gy < OMAP_DIM as usize {
                    let handle = chunk.get(lx, ly);
                    if terrain_is_highway(handle, registry) {
                        hgrid[gx][gy] = true;
                    }
                }
            }
        }
    }
    hgrid
}

/// Determine the cardinal orientation for a bridge based on which neighbours
/// are highway tiles. Returns `true` for NS orientation, `false` for EW.
fn bridge_is_ns(terrain_grid: &[[u32; 180]; 180], x: usize, y: usize, registry: &TerrainRegistry) -> bool {
    let x_i32 = x as i32;
    let y_i32 = y as i32;

    // Check north neighbour
    let north_hwy = if y_i32 > 0 {
        terrain_is_highway(TerrainHandle(terrain_grid[x][y - 1]), registry)
    } else {
        false
    };

    // Check south neighbour
    let south_hwy = if y_i32 + 1 < OMAP_DIM {
        terrain_is_highway(TerrainHandle(terrain_grid[x][y + 1]), registry)
    } else {
        false
    };

    // Check east neighbour
    let east_hwy = if x_i32 + 1 < OMAP_DIM {
        terrain_is_highway(TerrainHandle(terrain_grid[x + 1][y]), registry)
    } else {
        false
    };

    // Check west neighbour
    let west_hwy = if x_i32 > 0 {
        terrain_is_highway(TerrainHandle(terrain_grid[x - 1][y]), registry)
    } else {
        false
    };

    let ns = north_hwy || south_hwy;
    let ew = east_hwy || west_hwy;

    if ns && !ew {
        true
    } else if ew && !ns {
        false
    } else {
        // Both or neither — prefer NS (arbitrary but deterministic)
        true
    }
}

// ===========================================================================
// place_highway_interchanges
// ===========================================================================

/// Place highway interchange terrain (`hiway_4way`) at regular spacing
/// intervals along highway tiles.
///
/// Port of C++ `overmap::place_highway_interchanges()`.
///
/// In C++ this function iterates over `Highway_path` nodes with a per-path
/// counter. Since we lack `Highway_path` in this step, we scan the highway
/// presence grid in row-major order and apply the same spacing logic.
pub fn place_highway_interchanges(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
    config: Res<OvermapGenConfig>,
) {
    // ------------------------------------------------------------------
    // 1. Build terrain grid and highway presence grid (immutable pass)
    // ------------------------------------------------------------------
    let terrain_grid = build_terrain_grid(&chunks);
    let highway_grid = build_highway_presence_grid(&chunks, &registry);

    // ------------------------------------------------------------------
    // 2. Look up interchange terrain handle
    // ------------------------------------------------------------------
    let interchange_handle = registry
        .handle_by_id("hiway_4way")
        .or_else(|| registry.handle_by_id("highway_4way"));

    let Some(interchange) = interchange_handle else {
        info!("Highway interchanges skipped: no hiway_4way/highway_4way in registry");
        return;
    };

    // ------------------------------------------------------------------
    // 3. Walk highway tiles and record interchange placements
    // ------------------------------------------------------------------
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);

    // C++: `int node_count = HIGHWAY_INTERCHANGE_SPACING +
    //       rng(-HIGHWAY_INTERCHANGE_VARIANCE, HIGHWAY_INTERCHANGE_VARIANCE);`
    let mut node_count = INTERCHANGE_SPACING
        + rng.range_i32(-INTERCHANGE_VARIANCE, INTERCHANGE_VARIANCE);
    if node_count <= 0 {
        node_count = INTERCHANGE_SPACING;
    }

    let mut writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();
    let mut interchange_count: usize = 0;

    for y in 0..OMAP_DIM as usize {
        for x in 0..OMAP_DIM as usize {
            if !highway_grid[x][y] {
                continue;
            }

            // C++: `if (node.is_segment && node_count == 0)`
            if node_count == 0 {
                let pos = (x as i32, y as i32);

                // C++: `if (can_place_special(*interchange, node_pos, node_dir, false))`
                // In our grid-based approach, we check that the tile is still a highway
                // tile before recording the write.
                let current = TerrainHandle(terrain_grid[x][y]);
                if terrain_is_highway(current, &registry) {
                    writes.push((pos.0, pos.1, interchange));
                    interchange_count += 1;

                    // C++: `node_count = HIGHWAY_INTERCHANGE_SPACING;`
                    node_count = INTERCHANGE_SPACING
                        + rng.range_i32(-INTERCHANGE_VARIANCE, INTERCHANGE_VARIANCE);
                    if node_count <= 0 {
                        node_count = INTERCHANGE_SPACING;
                    }
                }
            }

            // C++: `if (node_count > 0) { node_count--; }`
            if node_count > 0 {
                node_count -= 1;
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. Write-back: apply all interchange writes via par_iter
    // ------------------------------------------------------------------
    if writes.is_empty() {
        info!(
            "Highway interchanges: 0 placed for overmap ({}, {})",
            config.om_x, config.om_y
        );
        return;
    }

    let reg = &*registry;
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, handle) in &writes {
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let current = new_terrain[idx];
                // Only overwrite highway tiles.
                if reg.flags_for(current).contains(TerrainFlags::HIGHWAY) {
                    if current != handle {
                        new_terrain[idx] = handle;
                        modified = true;
                    }
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

    info!(
        "Highway interchanges placed: {} interchanges on overmap ({}, {})",
        interchange_count, config.om_x, config.om_y
    );
}

// ===========================================================================
// finalize_highways
// ===========================================================================

/// Finalize highway tiles: replace highway tiles adjacent to water with
/// bridge terrain variants.
///
/// Port of C++ `overmap::finalize_highways()`.
///
/// In C++ this function iterates `Highway_path` nodes, marks road segments
/// for bridge specials (`segment_road_bridge`), and places the special when
/// no existing highway special overlaps. Since we lack `Highway_path` in this
/// step, we use a grid-based equivalent: any highway tile with a water
/// neighbour gets a bridge terrain.
pub fn finalize_highways(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    registry: Res<TerrainRegistry>,
    config: Res<OvermapGenConfig>,
) {
    // ------------------------------------------------------------------
    // 1. Build terrain grid and highway presence grid (immutable pass)
    // ------------------------------------------------------------------
    let terrain_grid = build_terrain_grid(&chunks);
    let highway_grid = build_highway_presence_grid(&chunks, &registry);

    // ------------------------------------------------------------------
    // 2. Look up bridge terrain handles
    // ------------------------------------------------------------------
    // C++: `const overmap_special_id &segment_road_bridge =
    //       settings->get_settings_highway().segment_road_bridge;`
    // Since we're terrain-based, we look up bridge terrains directly.
    let bridge_ns = registry
        .handle_by_id("hiway_bridge_ns")
        .or_else(|| registry.handle_by_id("highway_bridge_ns"))
        .or_else(|| registry.handle_by_id("bridge_ns"))
        .or_else(|| registry.handle_by_id("bridge"));

    let bridge_ew = registry
        .handle_by_id("hiway_bridge_ew")
        .or_else(|| registry.handle_by_id("highway_bridge_ew"))
        .or_else(|| registry.handle_by_id("bridge_ew"))
        .or_else(|| registry.handle_by_id("bridge"));

    let (Some(bridge_ns_h), Some(bridge_ew_h)) = (bridge_ns, bridge_ew) else {
        info!("Highway finalize skipped: no bridge terrain handles in registry");
        return;
    };

    // ------------------------------------------------------------------
    // 3. Scan every highway tile for water adjacency
    // ------------------------------------------------------------------
    let mut writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();

    for y in 0..OMAP_DIM as usize {
        for x in 0..OMAP_DIM as usize {
            if !highway_grid[x][y] {
                continue;
            }

            // C++: `if (is_road(ter(node.path_node.pos))) { node.placed_special = segment_road_bridge; }`
            // In our grid approach: check water neighbours to determine bridge placement.
            //
            // Check all 4 cardinal neighbours for water.
            let mut water_adjacent = false;
            let mut water_north = false;
            let mut water_south = false;
            let mut water_east = false;
            let mut water_west = false;

            for &(dx, dy) in &CARDINAL_OFFSETS {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || nx >= OMAP_DIM || ny < 0 || ny >= OMAP_DIM {
                    continue;
                }
                let nh = TerrainHandle(terrain_grid[nx as usize][ny as usize]);
                if terrain_is_water(nh, &registry) {
                    water_adjacent = true;
                    match (dx, dy) {
                        (0, -1) => water_north = true,
                        (1, 0) => water_east = true,
                        (0, 1) => water_south = true,
                        (-1, 0) => water_west = true,
                        _ => {}
                    }
                }
            }

            if !water_adjacent {
                continue;
            }

            // Determine bridge orientation based on which directions have water.
            //
            // If water is east or west → bridge_ns (road crosses water north–south)
            // If water is north or south → bridge_ew (road crosses water east–west)
            // If water on both axes, prefer the orientation that connects to
            // highway neighbours.
            let bridge_handle = if (water_north || water_south) && (water_east || water_west) {
                // Water on both axes → use orientation of neighbouring highway tiles.
                if bridge_is_ns(&terrain_grid, x, y, &registry) {
                    bridge_ns_h
                } else {
                    bridge_ew_h
                }
            } else if water_east || water_west {
                // Water to the east or west → road crosses north–south
                bridge_ns_h
            } else {
                // Water to the north or south → road crosses east–west
                bridge_ew_h
            };

            writes.push((x as i32, y as i32, bridge_handle));
        }
    }

    // ------------------------------------------------------------------
    // 4. Write-back: apply all bridge writes via par_iter
    // ------------------------------------------------------------------
    if writes.is_empty() {
        info!(
            "Highways finalized: 0 bridges for overmap ({}, {})",
            config.om_x, config.om_y
        );
        return;
    }

    let reg = &*registry;
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, handle) in &writes {
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let current = new_terrain[idx];
                // Only overwrite highway tiles.
                if reg.flags_for(current).contains(TerrainFlags::HIGHWAY) {
                    if current != handle {
                        new_terrain[idx] = handle;
                        modified = true;
                    }
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

    info!(
        "Highways finalized: {} bridges placed for overmap ({}, {})",
        writes.len(),
        config.om_x,
        config.om_y
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cdda_core_types::core::coords::ZLevel;

    /// Helper to build a TerrainFlags with a single flag set.
    fn make_flags(flag: u16) -> TerrainFlags {
        let mut f = TerrainFlags::empty();
        f.set(flag);
        f
    }

    /// Verify that INTERCHANGE_SPACING is exactly OMAP_DIM / 4.
    #[test]
    fn test_interchange_spacing() {
        assert_eq!(INTERCHANGE_SPACING, 45);
        assert_eq!(INTERCHANGE_VARIANCE, 4);
    }

    /// Verify water detection helper.
    #[test]
    fn test_terrain_is_water() {
        let mut registry = TerrainRegistry::empty();
        let water_idx = registry.register_no_entity(
            "water",
            make_flags(TerrainFlags::RIVER),
            0,
            "water".into(),
        );
        let land_idx = registry.register_no_entity(
            "land",
            TerrainFlags::empty(),
            2,
            "land".into(),
        );

        assert!(terrain_is_water(TerrainHandle::new(water_idx, 0), &registry));
        assert!(!terrain_is_water(TerrainHandle::new(land_idx, 0), &registry));
    }

    /// Verify highway detection helper.
    #[test]
    fn test_terrain_is_highway() {
        let mut registry = TerrainRegistry::empty();
        let hwy_idx = registry.register_no_entity(
            "hiway",
            make_flags(TerrainFlags::HIGHWAY),
            0,
            "hiway".into(),
        );
        let land_idx = registry.register_no_entity(
            "land",
            TerrainFlags::empty(),
            2,
            "land".into(),
        );

        assert!(terrain_is_highway(TerrainHandle::new(hwy_idx, 0), &registry));
        assert!(!terrain_is_highway(TerrainHandle::new(land_idx, 0), &registry));
    }

    /// Verify road detection helper.
    #[test]
    fn test_terrain_is_road() {
        let mut registry = TerrainRegistry::empty();
        let road_idx = registry.register_no_entity(
            "road",
            make_flags(TerrainFlags::ROAD),
            0,
            "road".into(),
        );
        let land_idx = registry.register_no_entity(
            "land",
            TerrainFlags::empty(),
            2,
            "land".into(),
        );

        assert!(terrain_is_road(TerrainHandle::new(road_idx, 0), &registry));
        assert!(!terrain_is_road(TerrainHandle::new(land_idx, 0), &registry));
    }
}
