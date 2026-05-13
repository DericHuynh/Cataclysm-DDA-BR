//! Step: Place monster groups based on terrain and city proximity.
//!
//! Simplified port of CDDA's `overmap::place_mongroups()` (overmap.cpp L3448-3649).
//! The C++ function handles city spawns, swamp, river, ocean spawns in ~200 lines.
//! This version covers city zombie spawns and river spawns; other biomes can be
//! added as additional systems.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use crate::pipeline::OvermapGenConfig;
use crate::steps::cities::City;
use tracing::info;

/// A placed monster group entity.
#[derive(Component)]
pub struct MonsterGroup {
    pub group_type: String, // "GROUP_ZOMBIE", "GROUP_SWAMP", etc.
    pub population: u32,
    pub omt_x: i32,
    pub omt_y: i32,
    pub z: i32,
}

/// Place monster groups based on terrain and city proximity.
pub fn place_mongroups(
    mut commands: Commands,
    chunks: Query<(&ChunkPosition, &OvermapChunk)>,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 13);

    // Build a dense grid of terrain type indices for z=0.
    let mut terrain_grid = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    terrain_grid[gx][gy] = chunk.get(lx, ly).type_index();
                }
            }
        }
    }

    // City spawns — place zombie groups near cities.
    for city in &cities {
        let radius = city.size as i32 + 2;
        let num_groups = rng.range_i32(radius, radius * 2);
        for _ in 0..num_groups {
            let angle = rng.range_f32(0.0, std::f64::consts::TAU as f32);
            let dist = rng.range_f32(1.0, radius as f32);
            let x = city.omt_x + (angle.cos() * dist) as i32;
            let y = city.omt_y + (angle.sin() * dist) as i32;
            if x >= 0 && x < OMAP_DIM && y >= 0 && y < OMAP_DIM {
                commands.spawn(MonsterGroup {
                    group_type: "GROUP_ZOMBIE".into(),
                    population: rng.range_i32(3, 8) as u32,
                    omt_x: x,
                    omt_y: y,
                    z: 0,
                });
            }
        }
    }

    // Swamp spawns — check terrain flags for swamp-related terrain.
    for x in (0..OMAP_DIM as usize).step_by(2) {
        for y in (0..OMAP_DIM as usize).step_by(2) {
            let handle = TerrainHandle::new(terrain_grid[x][y], 0);
            let flags = registry.flags_for(handle);
            if flags.contains(TerrainFlags::RIVER) && rng.one_in(3) {
                commands.spawn(MonsterGroup {
                    group_type: "GROUP_RIVER".into(),
                    population: rng.range_i32(2, 5) as u32,
                    omt_x: x as i32,
                    omt_y: y as i32,
                    z: 0,
                });
            }
        }
    }

    info!(
        "Mongroups placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
