//! Mutable overmap special placement.
//!
//! Simplified mutable special placement — places multi-overmap structures
//! (e.g. refugee centre, bandit camp) that use joins to connect sub-maps.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::query::{is_ot_match, OtMatchType};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::HashSet;
use tracing::info;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// PlacedMutableSpecial marker component
// ---------------------------------------------------------------------------

/// Marker component spawned for each placed mutable overmap special.
#[derive(Component)]
pub struct PlacedMutableSpecial {
    pub special_id: String,
    pub omt_x: i32,
    pub omt_y: i32,
}

// ---------------------------------------------------------------------------
// MutableSpecial — simplified representation
// ---------------------------------------------------------------------------

/// A single mutable overmap special to place.
#[derive(Debug, Clone)]
pub struct MutableSpecialPlacement {
    /// Unique identifier.
    pub id: String,
    /// Minimum occurrences.
    pub occ_min: i32,
    /// Maximum occurrences.
    pub occ_max: i32,
    /// Location constraint (same format as `SpecialPlacement::location`).
    pub location: String,
    /// Minimum city distance. -1 = no constraint.
    pub min_city_distance: i32,
    /// OMT terrain writes: each entry is (dx, dy, terrain_string_id).
    pub overmaps: Vec<(i32, i32, String)>,
    /// Connected sub-overmaps: each entry is (dx, dy, terrain_string_id, join_string_id).
    /// These are placed when the root is placed and connected via the join terrain.
    pub connected: Vec<(i32, i32, String, String)>,
}

impl Default for MutableSpecialPlacement {
    fn default() -> Self {
        Self {
            id: String::new(),
            occ_min: 0,
            occ_max: 1,
            location: "land".into(),
            min_city_distance: -1,
            overmaps: Vec::new(),
            connected: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Location check helper
// ---------------------------------------------------------------------------

fn matches_location(var: &str, handle: TerrainHandle, registry: &TerrainRegistry) -> bool {
    let flags = registry.flags_for(handle);
    let water = TerrainFlags::from_bits(TerrainFlags::LAKE | TerrainFlags::OCEAN | TerrainFlags::RIVER);

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
// Default mutable specials catalog
// ---------------------------------------------------------------------------

fn default_mutable_specials() -> Vec<MutableSpecialPlacement> {
    vec![
        MutableSpecialPlacement {
            id: "refugee_center".into(),
            occ_min: 1,
            occ_max: 1,
            location: "land".into(),
            min_city_distance: 8,
            overmaps: vec![(0, 0, "refugee_center".into())],
            connected: vec![],
            ..Default::default()
        },
        MutableSpecialPlacement {
            id: "bandit_camp".into(),
            occ_min: 0,
            occ_max: 2,
            location: "forest".into(),
            min_city_distance: 10,
            overmaps: vec![(0, 0, "bandit_camp".into())],
            connected: vec![],
            ..Default::default()
        },
        MutableSpecialPlacement {
            id: "hub_01".into(),
            occ_min: 0,
            occ_max: 1,
            location: "land".into(),
            min_city_distance: 12,
            overmaps: vec![(0, 0, "hub_01".into())],
            connected: vec![],
            ..Default::default()
        },
        MutableSpecialPlacement {
            id: "necropolis".into(),
            occ_min: 0,
            occ_max: 1,
            location: "land".into(),
            min_city_distance: 6,
            overmaps: vec![(0, 0, "necropolis".into())],
            connected: vec![],
            ..Default::default()
        },
        MutableSpecialPlacement {
            id: "island_prison".into(),
            occ_min: 0,
            occ_max: 1,
            location: "water".into(),
            min_city_distance: 15,
            overmaps: vec![(0, 0, "island_prison".into())],
            connected: vec![],
            ..Default::default()
        },
        MutableSpecialPlacement {
            id: "temple_stairs".into(),
            occ_min: 0,
            occ_max: 1,
            location: "forest".into(),
            min_city_distance: 10,
            overmaps: vec![(0, 0, "temple_stairs".into())],
            connected: vec![],
            ..Default::default()
        },
    ]
}

// ---------------------------------------------------------------------------
// place_mutable_specials — system entry point
// ---------------------------------------------------------------------------

/// Place mutable overmap specials.
///
/// Algorithm:
/// 1. Build terrain grid from z=0 chunks.
/// 2. For each mutable special from the catalog:
///    a. Parse its overmaps and placement rules.
///    b. Try random positions checking location constraints.
///    c. Place the root overmap + connected overmaps using joins.
///    d. Spawn a marker entity.
/// 3. Write terrain back to chunks.
pub fn place_mutable_specials(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
) {
    if !settings.place_specials {
        info!("place_mutable_specials: skipped — place_specials is false");
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

    // --- Build bounding overlaps for collision avoidance --------------------
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();

    // --- RNG -----------------------------------------------------------------
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 19);

    // --- Get catalog ---------------------------------------------------------
    let specials = default_mutable_specials();

    let mut tile_writes: Vec<(i32, i32, TerrainHandle)> = Vec::new();
    let mut placed_count = 0usize;
    let max_attempts = 100;

    for spec in &specials {
        let count = if spec.occ_max <= spec.occ_min {
            spec.occ_min
        } else {
            rng.range_i32(spec.occ_min, spec.occ_max)
        };

        for _ in 0..count {
            let mut placed = false;

            for _attempt in 0..max_attempts {
                let px = rng.range_i32(8, OMAP_DIM - 9);
                let py = rng.range_i32(8, OMAP_DIM - 9);

                let handle = ter_at(px, py);

                // Location constraint
                if !matches_location(&spec.location, handle, &registry) {
                    continue;
                }

                // Avoid overlap with already-placed mutable specials
                if occupied.contains(&(px, py)) {
                    continue;
                }

                // Place root overmaps
                for &(dx, dy, ref terrain_id) in &spec.overmaps {
                    let tx = px + dx;
                    let ty = py + dy;
                    if tx >= 0 && tx < OMAP_DIM && ty >= 0 && ty < OMAP_DIM {
                        if let Some(handle) = registry.handle_by_id(terrain_id) {
                            tile_writes.push((tx, ty, handle));
                            occupied.insert((tx, ty));
                        }
                    }
                }

                // Place connected sub-overmaps
                for &(dx, dy, ref terrain_id, ref _join_id) in &spec.connected {
                    let tx = px + dx;
                    let ty = py + dy;
                    if tx >= 0 && tx < OMAP_DIM && ty >= 0 && ty < OMAP_DIM {
                        if let Some(handle) = registry.handle_by_id(terrain_id) {
                            tile_writes.push((tx, ty, handle));
                            occupied.insert((tx, ty));
                        }
                    }
                }

                // Spawn marker
                commands.spawn(PlacedMutableSpecial {
                    special_id: spec.id.clone(),
                    omt_x: px,
                    omt_y: py,
                });

                placed = true;
                break;
            }

            if placed {
                placed_count += 1;
            }
        }
    }

    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        mutable_specials = placed_count,
        "place_mutable_specials: complete"
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
