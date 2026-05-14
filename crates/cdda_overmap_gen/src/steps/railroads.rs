//! Step 4b: Place railroads — verbatim port of C++ `overmap::place_railroads()`.
//!
//! Mirrors CDDA master `overmap.cpp` lines 2227–2297.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;
use crate::steps::neighbor_connections::ConnectionExits;
use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::{
    connect_closest_points, inbounds_omt, line_between, ConnectionType,
};
use cdda_overmap::direction::FOUR_ADJACENT_OFFSETS;
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use cdda_overmap::Rng;
use tracing::info;

// ---------------------------------------------------------------------------
// Utilities (Rust conversions of C++ helpers)
// ---------------------------------------------------------------------------

/// Fisher–Yates shuffle — replaces `std::shuffle(…, rng_get_engine())`.
fn shuffle<T>(slice: &mut [T], rng: &mut XorShiftRng) {
    for i in (1..slice.len()).rev() {
        let j = rng.random_usize(i + 1);
        slice.swap(i, j);
    }
}

/// C++ `random_entry(vec)` — panics on empty slice (must not happen here).
fn random_entry<'a, T>(slice: &'a [T], rng: &mut XorShiftRng) -> &'a T {
    &slice[rng.random_usize(slice.len())]
}

/// Chebyshev-distance radius (C++ `points_in_radius(tripoint, radius)` without z).
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

/// River check equivalent to C++ `is_river(ter(pos))`.
fn is_river(grid: &[[u32; OMAP_DIM as usize]; OMAP_DIM as usize], x: i32, y: i32, registry: &TerrainRegistry) -> bool {
    if x < 0 || x >= OMAP_DIM || y < 0 || y >= OMAP_DIM {
        return false;
    }
    let handle = TerrainHandle(grid[x as usize][y as usize]);
    registry.flags_for(handle).contains(TerrainFlags::RIVER)
}

// ---------------------------------------------------------------------------
// Build callback (placed inline as connect_closest_points closure)
// ---------------------------------------------------------------------------

/// Mark every tile on the straight line `from → to` in the boolean grid.
fn mark_line_on_grid(from: (i32, i32), to: (i32, i32), grid: &mut [[bool; OMAP_DIM as usize]; OMAP_DIM as usize]) {
    let path = line_between(from, to);
    for &(x, y) in &path {
        if inbounds_omt((x, y)) {
            grid[x as usize][y as usize] = true;
        }
    }
}

// ---------------------------------------------------------------------------
// place_railroads system
// ---------------------------------------------------------------------------

/// System: place railroad network.
///
/// **Verbatim port** of CDDA master `overmap::place_railroads()` L2227–2297.
///
/// # Mapping from C++
///
/// | C++ | Rust |
/// |---|---|
/// | `neighbor_overmaps[dir] == nullptr` | `ConnectionExits` direction empty |
/// | `connections_out[rail_connection]` | local `Vec<(i32,i32)>` (z=0 implied) |
/// | `four_adjacent_offsets` | `FOUR_ADJACENT_OFFSETS` (same layout) |
/// | `rng_get_engine()` | `rng: XorShiftRng` |
/// | `settings->get_settings_city().city_size` | `settings.city_size` |
/// | `*overmap_connection_local_railroad` | `ConnectionType::Railroad` |
/// | `elem.xy()` | `(elem.0, elem.1)` — z dropped |
pub fn place_railroads(
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
    exits: Option<Res<ConnectionExits>>,
) {
    // int op_city_size = settings->get_settings_city().city_size;
    let op_city_size = settings.city_size;
    // if( op_city_size <= 0 ) { return; }
    if op_city_size <= 0 || !settings.place_railroads {
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 7);

    // Build dense terrain grid at z=0 for fast lookup
    // (ter( tmp ) equivalent)
    let mut terrain_grid = [[0u32; OMAP_DIM as usize]; OMAP_DIM as usize];
    for (_entity, chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < OMAP_DIM as usize && gy < OMAP_DIM as usize {
                    let idx = ly as usize * CHUNK_DIM + lx as usize;
                    terrain_grid[gx][gy] = chunk.terrain[idx].0;
                }
            }
        }
    }

    // const overmap_connection_id &overmap_connection_local_railroad = settings->overmap_connection.rail_connection;
    // (Rust: ConnectionType::Railroad is always the railroad type)

    // std::vector<tripoint_om_omt> &railroads_out = connections_out[overmap_connection_local_railroad];
    // In Rust: local Vec<(i32,i32)> (points stored as xy, z=0 implied)
    let mut railroads_out: Vec<(i32, i32)> = Vec::new();

    // C++ connections_out persists across calls; neighbor exits are pre-populated.
    // In Rust we use ConnectionExits for neighbor-provided exits.
    if let Some(ref exits_res) = exits {
        railroads_out = exits_res.all();
    }

    // if( railroads_out.size() < 3 ) {
    if railroads_out.len() < 3 {
        // static constexpr std::array<int, 4> edge_coords_x = {OMAPX - 1, -1, 0, -1};
        // static constexpr std::array<int, 4> edge_coords_y = {-1, OMAPY - 1, -1, 0};
        const EDGE_COORDS_X: [i32; 4] = [OMAP_DIM - 1, -1, 0, -1];
        const EDGE_COORDS_Y: [i32; 4] = [-1, OMAP_DIM - 1, -1, 0];

        // std::array < int, OMAPX - 20 > omap_num;
        // for( int i = 0; i < OMAPX - 20; i++ ) { omap_num[i] = i + 10; }
        let omap_dim = OMAP_DIM as usize;
        let mut omap_num: Vec<i32> = Vec::with_capacity(omap_dim - 20);
        for i in 0..(omap_dim - 20) {
            omap_num.push((i + 10) as i32);
        }

        // std::array < size_t, 4 > dirs = {0, 1, 2, 3};
        // std::shuffle( dirs.begin(), dirs.end(), rng_get_engine() );
        let mut dirs: [usize; 4] = [0, 1, 2, 3];
        shuffle(&mut dirs, &mut rng);

        // for( size_t dir : dirs ) {
        for &dir in &dirs {
            // if( neighbor_overmaps[dir] == nullptr ) {
            //   →  ConnectionExits direction has no entries → no neighbor
            let has_neighbor = exits.as_ref().map_or(false, |e| {
                match dir {
                    0 => !e.north.is_empty(), // north neighbor
                    1 => !e.east.is_empty(),  // east neighbor
                    2 => !e.south.is_empty(), // south neighbor
                    3 => !e.west.is_empty(),  // west neighbor
                    _ => false,
                }
            });

            if !has_neighbor {
                // std::shuffle( omap_num.begin(), omap_num.end(), rng_get_engine() );
                shuffle(&mut omap_num, &mut rng);

                // for( const int &i : omap_num ) {
                for &i in &omap_num {
                    // tripoint_om_omt tmp = tripoint_om_omt(
                    //     edge_coords_x[dir] >= 0 ? edge_coords_x[dir] : i,
                    //     edge_coords_y[dir] >= 0 ? edge_coords_y[dir] : i, 0 );
                    let x = if EDGE_COORDS_X[dir] >= 0 { EDGE_COORDS_X[dir] } else { i };
                    let y = if EDGE_COORDS_Y[dir] >= 0 { EDGE_COORDS_Y[dir] } else { i };
                    let tmp = (x, y);

                    // is_river( ter( tmp ) )
                    let river_at_tmp = is_river(&terrain_grid, tmp.0, tmp.1, &registry);

                    // is_river( ter( tmp + point_rel_omt( four_adjacent_offsets[( dir + 1 ) % 4] ) ) )
                    let off1 = FOUR_ADJACENT_OFFSETS[(dir + 1) % 4];
                    let p1 = (tmp.0 + off1.0, tmp.1 + off1.1);
                    let river_at_p1 = is_river(&terrain_grid, p1.0, p1.1, &registry);

                    // is_river( ter( tmp + point_rel_omt( four_adjacent_offsets[( dir + 3 ) % 4] ) ) )
                    let off3 = FOUR_ADJACENT_OFFSETS[(dir + 3) % 4];
                    let p3 = (tmp.0 + off3.0, tmp.1 + off3.1);
                    let river_at_p3 = is_river(&terrain_grid, p3.0, p3.1, &registry);

                    // if( !( river1 || river2 || river3 ) )
                    if !river_at_tmp && !river_at_p1 && !river_at_p3 {
                        // railroads_out.push_back( tmp );
                        railroads_out.push(tmp);
                        break;
                    }
                }
                // if( railroads_out.size() == 3 ) { break; }
                if railroads_out.len() == 3 {
                    break;
                }
            }
        }
    }

    // std::vector<point_om_omt> railroad_points;
    // railroad_points.reserve( railroads_out.size() + cities.size() );
    let mut railroad_points: Vec<(i32, i32)> = Vec::with_capacity(
        railroads_out.len() + cities.iter().count(),
    );

    // for( const auto &elem : railroads_out ) {
    //     railroad_points.emplace_back( elem.xy() );
    // }
    for &elem in &railroads_out {
        railroad_points.push(elem);
    }

    // for( const city &elem : cities ) {
    //     railroad_points.emplace_back(
    //         random_entry( points_in_radius( tripoint_om_omt( elem.pos, 0 ), elem.size * 4 ) ).xy() );
    // }
    for city in &cities {
        let center = (city.omt_x, city.omt_y);
        let radius = (city.size as i32).saturating_mul(4).max(1);
        let candidates = points_in_radius(center, radius);
        if candidates.is_empty() {
            railroad_points.push(center);
        } else {
            let &p = random_entry(&candidates, &mut rng);
            railroad_points.push(p);
        }
    }

    // connect_closest_points( railroad_points, 0, *overmap_connection_local_railroad );
    // ----------------------------------------------------------------
    // Build boolean grid, then MST via connect_closest_points, then
    // write railroad terrain back to chunks.
    // ----------------------------------------------------------------
    if railroad_points.len() < 2 {
        return;
    }

    let mut rail_grid = [[false; OMAP_DIM as usize]; OMAP_DIM as usize];
    for &(x, y) in &railroad_points {
        if inbounds_omt((x, y)) {
            rail_grid[x as usize][y as usize] = true;
        }
    }

    connect_closest_points(
        &railroad_points,
        0,
        ConnectionType::Railroad,
        &mut rng,
        |from, to, z, _ct| {
            if z != 0 {
                return;
            }
            mark_line_on_grid(from, to, &mut rail_grid);
        },
    );

    // --- Write railroad terrain back to chunks ---
    let rail_handle = registry
        .handle_by_id("railroad")
        .unwrap_or(TerrainHandle::NULL);
    if rail_handle == TerrainHandle::NULL {
        info!("Railroad terrain handle missing, skipping railroad terrain");
        return;
    }
    let rail_ns_handle = registry.rotate(rail_handle, 0);
    let rail_ew_handle = registry.rotate(rail_handle, 1);
    let rail_nesw_handle = registry
        .handle_by_id("railroad_nesw")
        .unwrap_or_else(|| registry.rotate(rail_handle, 3));

    let field_index = registry.field_index;
    let forest_index = registry.forest_index;
    let forest_thick_index = registry.forest_thick_index;
    let forest_water_index = registry.forest_water_index;
    let rail_ew_idx = rail_ew_handle.type_index();
    let rail_ns_idx = rail_ns_handle.type_index();
    let rail_nesw_idx = rail_nesw_handle.type_index();
    let reg = &*registry;

    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx >= OMAP_DIM as usize || gy >= OMAP_DIM as usize {
                    continue;
                }
                if !rail_grid[gx][gy] {
                    continue;
                }

                let idx = ly as usize * CHUNK_DIM + lx as usize;
                let current = chunk.terrain[idx];
                let ct = current.type_index();

                // Skip water (matches C++ is_river check on terrain placement)
                let flags = reg.flags_for(current);
                if flags.contains(TerrainFlags::RIVER)
                    || flags.contains(TerrainFlags::LAKE)
                    || flags.contains(TerrainFlags::OCEAN)
                {
                    continue;
                }

                let is_field = ct == field_index;
                let is_forest =
                    ct == forest_index || ct == forest_thick_index || ct == forest_water_index;
                let is_rail = ct == rail_ew_idx || ct == rail_ns_idx || ct == rail_nesw_idx;

                if !is_field && !is_forest && !is_rail {
                    continue;
                }

                let north = gy > 0 && rail_grid[gx][gy - 1];
                let south = gy + 1 < OMAP_DIM as usize && rail_grid[gx][gy + 1];
                let east = gx + 1 < OMAP_DIM as usize && rail_grid[gx + 1][gy];
                let west = gx > 0 && rail_grid[gx - 1][gy];

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
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: new_terrain,
                });
            });
        }
    });

    info!(
        "Railroads placed: {} rail points, {} exit points for overmap ({}, {})",
        railroad_points.len(),
        railroads_out.len(),
        config.om_x,
        config.om_y
    );
}
