//! Step 5: Place city buildings within city boundaries.
//!
//! Port of CDDA master's `overmap::place_building()`.
//! Reads `CityBuildingDef` from `DefRegistry` and places them near city centers.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, OMAP_DIM};
use cdda_overmap::registry::TerrainRegistry;
use cdda_core_types::core::raw_defs::city_building::CityBuildingDef;
use cdda_overmap::rng::XorShiftRng;
use crate::pipeline::OvermapGenConfig;
use crate::steps::cities::City;
use std::sync::Arc;
use tracing::info;

/// Resource holding city building definitions for generation.
/// Populated from `DefRegistry.city_buildings` during data loading.
#[derive(Resource, Debug, Clone, Default)]
pub struct CityBuildingCatalog {
    pub buildings: Vec<Arc<CityBuildingDef>>,
}

/// Place buildings from the catalog around city centers.
pub fn place_city_buildings(
    _commands: Commands,
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    catalog: Option<Res<CityBuildingCatalog>>,
) {
    let Some(catalog) = catalog else {
        info!("No city building catalog — skipping building placement");
        return;
    };
    if catalog.buildings.is_empty() {
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 2);

    for city in &cities {
        let cx = city.omt_x;
        let cy = city.omt_y;
        let radius = city.size as i32;

        // Place buildings in a ring around the city center.
        let num_buildings = (radius.max(1) * 2) as usize;
        for _ in 0..num_buildings {
            let angle = rng.range_f32(0.0, std::f32::consts::TAU);
            let dist = rng.range_f32(2.0, radius as f32);
            let bx = cx + ((angle as f64).cos() * dist as f64) as i32;
            let by = cy + ((angle as f64).sin() * dist as f64) as i32;

            if bx < 0 || bx >= OMAP_DIM || by < 0 || by >= OMAP_DIM {
                continue;
            }

            // Pick a random building.
            let idx = rng.range_i32(0, catalog.buildings.len() as i32 - 1) as usize;
            let building = &catalog.buildings[idx];

            // Place each OMT in the building definition.
            for omt in building.overmaps.iter().flatten() {
                let px = bx + omt.point.first().copied().unwrap_or(0);
                let py = by + omt.point.get(1).copied().unwrap_or(0);

                if px < 0 || px >= OMAP_DIM || py < 0 || py >= OMAP_DIM {
                    continue;
                }

                if let Some(handle) = registry.handle_by_id(&omt.overmap) {
                    for (chunk_pos, mut chunk) in &mut chunks {
                        if chunk_pos.z.0 != 0 { continue; }
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
        }
    }

    info!(
        "City buildings placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
