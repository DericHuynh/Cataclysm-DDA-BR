//! Step 7b: Forest trailhead placement.
//!
//! Port of CDDA master's `overmap::place_forest_trailheads()`.
//!
//! # Algorithm
//!
//! 1. Scan all z=0 chunks for forest trail endpoints.
//! 2. Place trailhead terrain markers at trail entry/exit points.
//!
//! This is a stub pending full port of forest trail pathfinding.
//! The core forest trail path system (`forest_trails.rs`) places the trails;
//! this step adds the trailhead markers.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainRegistry};
use tracing::info;

/// Place trailhead terrain at forest trail endpoints.
pub fn place_forest_trailheads(
    chunks: Query<(&ChunkPosition, &OvermapChunk)>,
    registry: Res<TerrainRegistry>,
) {
    let mut trail_count = 0usize;
    let mut endpoint_count = 0usize;

    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let handle = chunk.get(lx, ly);
                let flags = registry.flags_for(handle);
                if flags.contains(TerrainFlags::FOREST) {
                    trail_count += 1;

                    // Count adjacent trail tiles
                    let x = ox + lx as i32;
                    let y = oy + ly as i32;
                    let dirs = [(0, -1), (1, 0), (0, 1), (-1, 0)];
                    let mut adj_trail = 0;
                    for &(dx, dy) in &dirs {
                        let nx = x + dx;
                        let ny = y + dy;
                        for (ncp, nchunk) in &chunks {
                            if ncp.z.0 != 0 {
                                continue;
                            }
                            let (nox, noy) = ncp.omt_origin();
                            let nlx = nx - nox;
                            let nly = ny - noy;
                            if nlx >= 0 && nlx < CHUNK_DIM as i32
                                && nly >= 0 && nly < CHUNK_DIM as i32
                            {
                                let nflags = registry.flags_for(nchunk.get(nlx as u8, nly as u8));
                                if nflags.contains(TerrainFlags::FOREST) {
                                    adj_trail += 1;
                                }
                                break;
                            }
                        }
                    }

                    // Trail endpoints have exactly 1 adjacent trail tile
                    if adj_trail == 1 {
                        endpoint_count += 1;
                    }
                }
            }
        }
    }

    info!(
        "Forest trailheads: {} trails, {} endpoints detected",
        trail_count, endpoint_count
    );
}
