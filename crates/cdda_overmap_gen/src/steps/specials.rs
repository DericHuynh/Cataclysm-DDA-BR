//! Overmap specials placement.
//!
//! Port of C++ `overmap::place_specials()` and related functions.
//! Places fixed overmap specials (lab, military base, etc.) by checking
//! location constraints and city distance/size requirements.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::steps::cities::City;

// ---------------------------------------------------------------------------
// PlacedSpecial marker component
// ---------------------------------------------------------------------------

/// Marker component spawned for each placed overmap special.
#[derive(Component)]
pub struct PlacedSpecial {
    pub special_id: String,
    pub omt_x: i32,
    pub omt_y: i32,
}

// ---------------------------------------------------------------------------
// SpecialPlacement — simplified representation of a special's placement rules
// ---------------------------------------------------------------------------

/// Placement constraints for a single overmap special.
#[derive(Debug, Clone)]
pub struct SpecialPlacement {
    /// Unique identifier for this special.
    pub id: String,
    /// Minimum occurrences per overmap.
    pub occ_min: i32,
    /// Maximum occurrences per overmap.
    pub occ_max: i32,
    /// Minimum city distance (OMT tiles). -1 = no constraint.
    pub min_city_distance: i32,
    /// Maximum city distance. -1 = no constraint.
    pub max_city_distance: i32,
    /// Minimum city size (for distance checks). -1 = any.
    pub min_city_size: i32,
    /// Maximum city size (for distance checks). -1 = any.
    pub max_city_size: i32,
    /// Location constraint: "land", "forest", "swamp", "water", or a terrain ID prefix.
    pub location: String,
    /// OMT terrain writes: for each tile making up this special.
    /// Each entry is (dx, dy, terrain_string_id) relative to the placed origin.
    pub overmaps: Vec<(i32, i32, String)>,
}

impl Default for SpecialPlacement {
    fn default() -> Self {
        Self {
            id: String::new(),
            occ_min: 0,
            occ_max: 1,
            min_city_distance: -1,
            max_city_distance: -1,
            min_city_size: -1,
            max_city_size: -1,
            location: String::new(),
            overmaps: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Location check helper
// ---------------------------------------------------------------------------

/// Check whether a tile satisfies a location constraint.
///
/// | Constraint | Condition |
/// |---|---|---|
/// | "land"     | Not LAKE, OCEAN, or RIVER |
/// | "forest"   | Has FOREST flag |
/// | "swamp"    | Has FOREST flag AND LAKE flag |
/// | "water"    | Has LAKE or OCEAN or RIVER flag |
/// | other      | Uses `is_ot_match` with `OtMatchType::Contains`, then `Prefix` |
fn matches_location(var: &str, handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    let water =
        TerrainFlags::from_bits(TerrainFlags::LAKE | TerrainFlags::OCEAN | TerrainFlags::RIVER);

    match var {
        "land" => !flags.intersects(water),
        "forest" => flags.contains(TerrainFlags::FOREST) && !flags.intersects(water),
        "swamp" => flags.contains(TerrainFlags::FOREST) && flags.contains(TerrainFlags::LAKE),
        "water" => flags.intersects(water),
        _ => {
            is_ot_match(var, handle, registry, OtMatchType::Contains)
                || is_ot_match(var, handle, registry, OtMatchType::Prefix)
        }
    }
}

// ---------------------------------------------------------------------------
// City distance / size check
// ---------------------------------------------------------------------------

/// Check whether an OMT position satisfies all city distance and size constraints.
fn check_city_constraints(
    x: i32,
    y: i32,
    placement: &SpecialPlacement,
    cities: &[&City],
    _registry: &TerrainRegistry,
    _grid: &[TerrainHandle],
    _omap_size: usize,
) -> bool {
    if placement.min_city_distance < 0 && placement.max_city_distance < 0 {
        return true; // No city constraints
    }

    // Find the closest city
    let mut closest_dist = i32::MAX;
    let mut _closest_city_size: i32 = 0;

    for city in cities {
        let d = (x - city.omt_x).abs().max((y - city.omt_y).abs()); // Chebyshev distance
        if d < closest_dist {
            closest_dist = d;
            _closest_city_size = city.size as i32;
        }
    }

    // Check distance bounds
    if placement.min_city_distance >= 0 && closest_dist < placement.min_city_distance {
        return false;
    }
    if placement.max_city_distance >= 0 && closest_dist > placement.max_city_distance {
        return false;
    }

    // Additional city size filtering — if both min and max are specified,
    // only count cities within the distance range whose size matches
    if placement.min_city_size >= 0 || placement.max_city_size >= 0 {
        let mut any_matching_city = false;
        for city in cities {
            let d = (x - city.omt_x).abs().max((y - city.omt_y).abs());
            let in_range = if placement.min_city_distance >= 0 {
                d >= placement.min_city_distance
            } else {
                true
            } && if placement.max_city_distance >= 0 {
                d <= placement.max_city_distance
            } else {
                true
            };

            if !in_range {
                continue;
            }

            let size_ok = if placement.min_city_size >= 0 {
                (city.size as i32) >= placement.min_city_size
            } else {
                true
            } && if placement.max_city_size >= 0 {
                (city.size as i32) <= placement.max_city_size
            } else {
                true
            };

            if size_ok {
                any_matching_city = true;
                break;
            }
        }
        if !any_matching_city {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Default specials catalog
// ---------------------------------------------------------------------------

/// Build a default set of specials to place.
///
/// In a full implementation this would be loaded from JSON data.
/// For now we provide a representative set that mirrors CDDA vanilla specials.
fn default_specials_catalog() -> Vec<SpecialPlacement> {
    vec![
        SpecialPlacement {
            id: "lab".into(),
            occ_min: 1,
            occ_max: 4,
            min_city_distance: 4,
            max_city_distance: -1,
            location: "land".into(),
            overmaps: vec![(0, 0, "lab".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "lab_ice".into(),
            occ_min: 0,
            occ_max: 1,
            min_city_distance: 10,
            max_city_distance: -1,
            location: "forest".into(),
            overmaps: vec![(0, 0, "ice_lab".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "military_base".into(),
            occ_min: 0,
            occ_max: 1,
            min_city_distance: 12,
            max_city_distance: -1,
            location: "land".into(),
            overmaps: vec![
                (0, 0, "military_base".into()),
                (0, -1, "mil_base_north".into()),
                (0, 1, "mil_base_south".into()),
                (1, 0, "mil_base_east".into()),
                (-1, 0, "mil_base_west".into()),
            ],
            ..Default::default()
        },
        SpecialPlacement {
            id: "mine".into(),
            occ_min: 1,
            occ_max: 3,
            min_city_distance: 6,
            max_city_distance: -1,
            location: "land".into(),
            overmaps: vec![(0, 0, "mine".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "radio_tower".into(),
            occ_min: 2,
            occ_max: 6,
            min_city_distance: 3,
            max_city_distance: -1,
            location: "land".into(),
            overmaps: vec![(0, 0, "radio_tower".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "anthill".into(),
            occ_min: 2,
            occ_max: 5,
            min_city_distance: 5,
            max_city_distance: -1,
            location: "forest".into(),
            overmaps: vec![(0, 0, "anthill".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "triffid_grove".into(),
            occ_min: 1,
            occ_max: 3,
            min_city_distance: 8,
            max_city_distance: -1,
            location: "forest".into(),
            overmaps: vec![(0, 0, "triffid_grove".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "fungal_bloom".into(),
            occ_min: 1,
            occ_max: 3,
            min_city_distance: 6,
            max_city_distance: -1,
            location: "swamp".into(),
            overmaps: vec![(0, 0, "fungal_bloom".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "spider_pit".into(),
            occ_min: 1,
            occ_max: 4,
            min_city_distance: 4,
            max_city_distance: -1,
            location: "forest".into(),
            overmaps: vec![(0, 0, "spider_pit".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "cabin".into(),
            occ_min: 2,
            occ_max: 8,
            min_city_distance: 5,
            max_city_distance: 40,
            location: "forest".into(),
            overmaps: vec![(0, 0, "cabin".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "lmoe".into(),
            occ_min: 0,
            occ_max: 1,
            min_city_distance: 8,
            max_city_distance: -1,
            location: "forest".into(),
            overmaps: vec![(0, 0, "lmoe".into())],
            ..Default::default()
        },
        SpecialPlacement {
            id: "haz_sar".into(),
            occ_min: 1,
            occ_max: 2,
            min_city_distance: 6,
            max_city_distance: -1,
            location: "land".into(),
            overmaps: vec![(0, 0, "haz_sar".into())],
            ..Default::default()
        },
    ]
}

// ---------------------------------------------------------------------------
// place_specials — system entry point
// ---------------------------------------------------------------------------

/// Place fixed overmap specials.
///
/// Port of C++ `overmap::place_specials()`.
///
/// Algorithm:
/// 1. Build terrain grid from z=0 chunks.
/// 2. Check if overmap has lakes/oceans (for water-flagged specials).
/// 3. For each special in the catalog:
///    a. Determine random count to place.
///    b. Try random positions, checking location + city constraints.
///    c. If all checks pass, collect terrain writes and spawn a marker.
/// 4. Write all terrain changes back to chunks.
pub fn place_specials(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.place_specials {
        info!("place_specials: skipped — place_specials is false");
        return;
    }

    // --- Build terrain grid from z=0 chunks ---------------------------------
    let omap_size = OMAP_DIM as usize;
    let mut grid = vec![TerrainHandle::NULL; omap_size * omap_size];

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
                    grid[(gy as usize) * omap_size + (gx as usize)] = chunk.get(lx as u8, ly as u8);
                }
            }
        }
    }

    let ter_at = |x: i32, y: i32| -> TerrainHandle {
        if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
            grid[(y as usize) * omap_size + (x as usize)]
        } else {
            TerrainHandle::NULL
        }
    };

    // --- Check if overmap has lakes/oceans for water specials ---------------
    let has_lake = grid
        .iter()
        .any(|h| registry.flags_for(*h).contains(TerrainFlags::LAKE));
    let has_ocean = grid
        .iter()
        .any(|h| registry.flags_for(*h).contains(TerrainFlags::OCEAN));

    // --- Collect cities for distance checks ---------------------------------
    let city_refs: Vec<&City> = cities.iter().collect();

    // --- RNG -----------------------------------------------------------------
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 17);

    // --- Get catalog ---------------------------------------------------------
    let specials = default_specials_catalog();

    let mut tile_writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();
    let mut placed_count = 0usize;
    let max_placement_attempts = 200;

    for spec in &specials {
        // Skip water specials if no water bodies exist on this overmap
        if spec.location == "water" && !has_lake && !has_ocean {
            continue;
        }

        // Determine random count
        let count = if spec.occ_max <= spec.occ_min {
            spec.occ_min
        } else {
            rng.range_i32(spec.occ_min, spec.occ_max)
        };

        let mut placed_for_this = 0;

        for _ in 0..count {
            let mut placed = false;

            for _attempt in 0..max_placement_attempts {
                let px = rng.range_i32(4, OMAP_DIM - 5);
                let py = rng.range_i32(4, OMAP_DIM - 5);

                let handle = ter_at(px, py);

                // Check location constraint
                if !matches_location(&spec.location, handle, &registry) {
                    continue;
                }

                // Check city constraints
                if !check_city_constraints(px, py, spec, &city_refs, &registry, &grid, omap_size) {
                    continue;
                }

                // All checks passed — collect terrain writes and place
                for &(dx, dy, ref terrain_id) in &spec.overmaps {
                    let tx = px + dx;
                    let ty = py + dy;
                    if tx >= 0 && tx < OMAP_DIM && ty >= 0 && ty < OMAP_DIM {
                        if let Some(handle) = registry.handle_by_id(terrain_id) {
                            tile_writes.push((tx, ty, handle));
                        }
                    }
                }

                // Spawn marker entity
                commands.spawn(PlacedSpecial {
                    special_id: spec.id.clone(),
                    omt_x: px,
                    omt_y: py,
                });

                placed = true;
                break;
            }

            if placed {
                placed_for_this += 1;
            }
        }

        if placed_for_this > 0 {
            placed_count += placed_for_this;
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        specials_placed = placed_count,
        "place_specials: complete"
    );

    // --- Write back to chunks -----------------------------------------------
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
