//! Step 10: Finalize overmap generation.
//!
//! Marks all chunks as immutable (via component insertion) and
//! logs generation statistics.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainHandle, TerrainRegistry};
use crate::pipeline::OvermapGenConfig;
use tracing::info;

/// Marker component indicating a chunk has been finalized.
/// After finalization, terrain should not be modified.
#[derive(Component)]
pub struct Finalized;

/// Mark all chunks as finalized and log statistics.
pub fn finalize_overmap(
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
) {
    let mut terrain_counts: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut total_tiles = 0usize;

    for (entity, _chunk_pos, chunk) in &chunks {
        // Mark as finalized
        commands.entity(entity).insert(Finalized);

        // Count terrain types
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let h = chunk.get(lx, ly);
                *terrain_counts.entry(h.type_index()).or_default() += 1;
                total_tiles += 1;
            }
        }
    }

    // Log top terrain types
    let mut counts: Vec<(u32, usize)> = terrain_counts.into_iter().collect();
    counts.sort_by_key(|&(_, c)| std::cmp::Reverse(c));

    info!(
        "Overmap ({}, {}) finalized: {} tiles across {} chunks",
        config.om_x, config.om_y, total_tiles,
        chunks.iter().count()
    );

    for (type_idx, count) in counts.iter().take(10) {
        let pct = *count as f32 / total_tiles as f32 * 100.0;
        let name = registry.mapgen_id(TerrainHandle::new(*type_idx, 0));
        info!("  {}: {} tiles ({:.1}%)", name, count, pct);
    }
}
