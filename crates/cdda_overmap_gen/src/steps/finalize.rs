//! Finalization system — log generation statistics and mark chunks as finalized.
//!
//! This is the last step in the overmap generation pipeline. It:
//! 1. Iterates all z=0 chunks, counting terrain type frequencies.
//! 2. Logs the top 10 most common terrain types.
//! 3. Inserts a [`Finalized`] component on every chunk to signal completion.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use tracing::info;

use crate::pipeline::OvermapGenConfig;

// ---------------------------------------------------------------------------
// Finalized marker component
// ---------------------------------------------------------------------------

/// Marker component indicating a chunk has been through the full generation
/// pipeline and is ready for gameplay.
#[derive(Component)]
pub struct Finalized;

// ---------------------------------------------------------------------------
// finalize_overmap — system entry point
// ---------------------------------------------------------------------------

/// Log terrain statistics and mark all chunks as finalized.
pub fn finalize_overmap(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
) {
    info!(
        om_x = config.om_x,
        om_y = config.om_y,
        "finalize_overmap: generation complete, computing statistics"
    );

    // --- Count terrain types across all z=0 chunks ---------------------------
    let mut type_counts: HashMap<u32, usize> = HashMap::new();
    let mut total_tiles = 0usize;

    for (_entity, chunk_pos, chunk) in &chunks {
        // Insert Finalized marker on every chunk (all z-levels)
        commands.entity(_entity).insert(Finalized);

        if chunk_pos.z.0 != 0 {
            continue;
        }

        for ly in 0..CHUNK_DIM {
            for lx in 0..CHUNK_DIM {
                let handle = chunk.get(lx as u8, ly as u8);
                if handle != TerrainHandle::NULL {
                    let type_idx = handle.type_index();
                    *type_counts.entry(type_idx).or_insert(0) += 1;
                    total_tiles += 1;
                }
            }
        }
    }

    // --- Log top 10 terrain types --------------------------------------------
    let mut sorted: Vec<(u32, usize)> = type_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let top_n = 10usize.min(sorted.len());
    info!(
        total_tiles,
        unique_types = sorted.len(),
        "finalize_overmap: terrain statistics for overmap ({}, {})",
        config.om_x,
        config.om_y
    );

    for (i, (type_idx, count)) in sorted.iter().take(top_n).enumerate() {
        let handle = TerrainHandle::new(*type_idx, 0);
        let name = registry.string_id_for(handle).unwrap_or("<unknown>");
        let pct = if total_tiles > 0 {
            (*count as f64 / total_tiles as f64) * 100.0
        } else {
            0.0
        };
        info!(
            rank = i + 1,
            terrain = name,
            count,
            pct = format!("{:.1}%", pct),
            "finalize_overmap: top terrain"
        );
    }

    info!("finalize_overmap: done");
}
