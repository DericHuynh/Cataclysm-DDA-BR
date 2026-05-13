//! Step 7: Place overmap specials from loaded JSON definitions.
//!
//! Port of CDDA master's `overmap::place_specials()` and `overmap::place_special()`.
//! Uses `SpecialCatalog` (populated from `DefRegistry.overmap_specials` during data loading).

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::rng::XorShiftRng;

use cdda_overmap::connections::inbounds_omt;
use cdda_core_types::core::raw_defs::overmap_terrain::OvermapSpecialDef;
use cdda_core_types::core::raw_defs::cdda_types::StringOrArray;
use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::special_catalog::SpecialCatalog;
use crate::steps::cities::City;
use std::sync::Arc;
use tracing::info;

/// A placed overmap special entity marker.
#[derive(Component)]
pub struct PlacedSpecial {
    pub special_id: String,
    pub omt_x: i32,
    pub omt_y: i32,
}

/// Place overmap specials on the generated terrain.
///
/// For each special in the catalog:
/// 1. Check location constraints (e.g. "land", "forest", "swamp")
/// 2. Check city distance constraints
/// 3. Find valid placement positions
/// 4. Place terrain tiles from the special definition
/// 5. Handle uniqueness flags
pub fn place_specials(
    mut commands: Commands,
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    _settings: Res<OvermapRegionSettings>,
    catalog: Option<Res<SpecialCatalog>>,
) {
    let Some(catalog) = catalog else {
        info!("No special catalog — skipping overmap special placement");
        return;
    };
    if catalog.specials.is_empty() {
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 31);

    // Build dense grid of terrain type indices for z=0.
    let mut grid = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 { continue; }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    grid[gx][gy] = chunk.get(lx, ly).type_index();
                }
            }
        }
    }

    // Collect city info for distance checks.
    let city_positions: Vec<(i32, i32, i32)> = cities.iter()
        .map(|c| (c.omt_x, c.omt_y, c.size as i32))
        .collect();

    // Check if overmap has lakes or oceans (for LAKE/OCEAN flagged specials).
    let has_lake = grid.iter().flatten().any(|&idx| {
        let flags = registry.flags_for(TerrainHandle::new(idx, 0));
        flags.contains(TerrainFlags::LAKE)
    });
    let has_ocean = grid.iter().flatten().any(|&idx| {
        let flags = registry.flags_for(TerrainHandle::new(idx, 0));
        flags.contains(TerrainFlags::OCEAN)
    });

    let mut placed_count = 0usize;

    for special_def in &catalog.specials {
        let flags: Vec<&str> = match &special_def.flags {
            StringOrArray::Single(s) => vec![s.as_str()],
            StringOrArray::Multi(v) => v.iter().map(|s| s.as_str()).collect(),
        };

        // Apply overmap-level filters
        if flags.contains(&"LAKE") && !has_lake { continue; }
        if flags.contains(&"OCEAN") && !has_ocean { continue; }
        if flags.contains(&"GLOBALLY_UNIQUE") || flags.contains(&"OVERMAP_UNIQUE") {
            // For unique specials, skip for now (tracking not yet implemented)
            continue;
        }

        // Parse location constraints
        let location_strs: Vec<&str> = match &special_def.locations {
            StringOrArray::Single(s) => vec![s.as_str()],
            StringOrArray::Multi(v) => v.iter().map(|s| s.as_str()).collect(),
        };

        // Parse occurrence constraints [min, max]
        let (occ_min, occ_max) = special_def.occurrences
            .map(|o| (o[0] as usize, o[1] as usize))
            .unwrap_or((0, 1));

        // Parse city_distance [min, max]
        let city_dist = special_def.city_distance.unwrap_or([0, i32::MAX as i32]);

        // Parse city_sizes [min, max]
        let city_sizes = special_def.city_sizes.unwrap_or([0, i32::MAX as i32]);

        // Determine number to place (random between min and max)
        let to_place = if occ_min >= occ_max { occ_min }
            else { rng.range_i32(occ_min as i32, occ_max as i32) as usize };

        let mut placed = 0usize;

        // Try random positions
        for _ in 0..(to_place * 50).max(100) {
            if placed >= to_place { break; }

            let x = rng.range_i32(2, OMAP_DIM - 2);
            let y = rng.range_i32(2, OMAP_DIM - 2);

            // Check city distance constraints
            let mut city_ok = city_dist[0] == 0 && city_dist[1] == i32::MAX;
            if !city_ok {
                for &(cx, cy, csize) in &city_positions {
                    let dx = (x - cx).abs();
                    let dy = (y - cy).abs();
                    let dist = dx.max(dy);
                    if csize >= city_sizes[0] && csize <= city_sizes[1]
                        && dist >= city_dist[0] && dist <= city_dist[1] {
                        city_ok = true;
                        break;
                    }
                }
                // If no city exists and min distance is 0, also OK
                if city_positions.is_empty() && city_dist[0] == 0 {
                    city_ok = true;
                }
            }
            if !city_ok { continue; }

            // Check that each of the special's OMTs can be placed.
            if !can_place_special_at(x, y, special_def, &grid, &location_strs, &registry) {
                continue;
            }

            // Place the special's OMTs
            place_special_omts(
                x, y, special_def, &mut chunks, &mut commands, &registry
            );
            placed += 1;
            placed_count += 1;
        }

        if placed > 0 {
            info!("Placed special '{}' {} times at overmap ({}, {})",
                special_def.id.as_str(), placed, config.om_x, config.om_y);
        }
    }

    if placed_count > 0 {
        info!("Total overmap specials placed: {} for overmap ({}, {})",
            placed_count, config.om_x, config.om_y);
    }
}

/// Check if a special can be placed at the given position.
fn can_place_special_at(
    origin_x: i32, origin_y: i32,
    special: &Arc<OvermapSpecialDef>,
    grid: &[[u32; 180]; 180],
    location_strs: &[&str],
    registry: &TerrainRegistry,
) -> bool {
    // Get the overmaps from the special definition.
    let overmaps_raw = match &special.overmaps {
        Some(cdda_core_types::core::raw_defs::cdda_types::RawValue::Array(arr)) => arr,
        _ => return false,
    };

    for entry in overmaps_raw {
        // Each entry can be a string (OMT ID) or an object {overmap, point}
        let (_omt_id, dx, dy, _dz) = match entry {
            cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s) => {
                (s.as_str(), 0i32, 0i32, 0i32)
            }
            cdda_core_types::core::raw_defs::cdda_types::RawValue::Object(obj) => {
                let omt = obj.get("overmap")
                    .and_then(|v| match v {
                        cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                let point = obj.get("point")
                    .and_then(|v| match v {
                        cdda_core_types::core::raw_defs::cdda_types::RawValue::Array(a) => {
                            Some((
                                a.first().and_then(|v| match v { cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0),
                                a.get(1).and_then(|v| match v { cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0),
                                a.get(2).and_then(|v| match v { cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0),
                            ))
                        }
                        _ => None,
                    })
                    .unwrap_or((0, 0, 0));
                (omt, point.0, point.1, point.2)
            }
            _ => continue,
        };

        let px = origin_x + dx;
        let py = origin_y + dy;

        if !inbounds_omt((px, py)) {
            return false;
        }

        let tile_handle = TerrainHandle::new(grid[px as usize][py as usize], 0);
        let tile_flags = registry.flags_for(tile_handle);

        // Check that this tile matches at least one of the required locations.
        let mut matches = false;
        for loc in location_strs {
            match *loc {
                "land" => {
                    if !tile_flags.contains(TerrainFlags::LAKE)
                        && !tile_flags.contains(TerrainFlags::OCEAN)
                        && !tile_flags.contains(TerrainFlags::RIVER)
                    { matches = true; break; }
                }
                "forest" => {
                    if tile_flags.contains(TerrainFlags::FOREST)
                    { matches = true; break; }
                }
                "swamp" => {
                    if tile_flags.contains(TerrainFlags::FOREST)
                        && tile_flags.contains(TerrainFlags::LAKE)
                    { matches = true; break; }
                }
                "water" => {
                    if tile_flags.contains(TerrainFlags::LAKE)
                        || tile_flags.contains(TerrainFlags::OCEAN)
                        || tile_flags.contains(TerrainFlags::RIVER)
                    { matches = true; break; }
                }
                _ => {
                    // Try is_ot_match for unknown location types
                    if is_ot_match(loc, tile_handle, registry, OtMatchType::Contains)
                        || is_ot_match(loc, tile_handle, registry, OtMatchType::Prefix)
                    { matches = true; break; }
                }
            }
        }
        if !matches {
            return false;
        }
    }
    true
}

/// Place the terrain tiles of a special at the given origin.
fn place_special_omts(
    origin_x: i32, origin_y: i32,
    special: &Arc<OvermapSpecialDef>,
    chunks: &mut Query<(&ChunkPosition, &mut OvermapChunk)>,
    commands: &mut Commands,
    registry: &TerrainRegistry,
) {
    let overmaps_raw = match &special.overmaps {
        Some(cdda_core_types::core::raw_defs::cdda_types::RawValue::Array(arr)) => arr,
        _ => return,
    };

    for entry in overmaps_raw {
        let (omt_id, dx, dy, dz) = match entry {
            cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s) => {
                (s.as_str(), 0i32, 0i32, 0i32)
            }
            cdda_core_types::core::raw_defs::cdda_types::RawValue::Object(obj) => {
                let omt = obj.get("overmap")
                    .and_then(|v| match v {
                        cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                let point = obj.get("point")
                    .and_then(|v| match v {
                        cdda_core_types::core::raw_defs::cdda_types::RawValue::Array(a) => {
                            Some((
                                a.first().and_then(|v| match v { cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0),
                                a.get(1).and_then(|v| match v { cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0),
                                a.get(2).and_then(|v| match v { cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n) => Some(*n as i32), _ => None }).unwrap_or(0),
                            ))
                        }
                        _ => None,
                    })
                    .unwrap_or((0, 0, 0));
                (omt, point.0, point.1, point.2)
            }
            _ => continue,
        };

        let px = origin_x + dx;
        let py = origin_y + dy;

        if !inbounds_omt((px, py)) { continue; }

        if let Some(handle) = registry.handle_by_id(omt_id) {
            for (chunk_pos, mut chunk) in &mut *chunks {
                if chunk_pos.z.0 != dz as i8 { continue; }
                let (ox, oy) = chunk_pos.omt_origin();
                let lx = px - ox;
                let ly = py - oy;
                if lx >= 0 && lx < 32 && ly >= 0 && ly < 32 {
                    chunk.set(lx as u8, ly as u8, handle);
                    break;
                }
            }
        }
    }

    // Spawn marker entity
    commands.spawn(PlacedSpecial {
        special_id: special.id.as_str().to_string(),
        omt_x: origin_x,
        omt_y: origin_y,
    });
}
