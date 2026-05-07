//! # cdda_map — Map storage, tile queries, and mapgen
//!
//! Stores terrain, furniture, trap, field, and item data in a
//! struct-of-arrays layout for cache efficiency.
//!
//! ## Tile storage
//!
//! Each `OmtTerrain` is a 24×24 overmap-terrain tile block. The world is
//! composed of many bubbles accessed via `WorldMap`.
//!
//! ## Future additions
//!
//! - `ItemStore` — dense item storage with linked-list per tile
//! - `OvermapIndex` — overmap terrain, cities, specials
//! - `EntitySpatialIndex` bridge — spatial lookup for dynamic entities
//! - Pathfinding — A* over terrain graph

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// PlacedOmt — debug metadata for dev worldgen
// ---------------------------------------------------------------------------

/// Records which building and OMT was placed at a bubble position.
/// Used by dev-worldgen and map-debug tools to show what building each
/// overmap terrain tile belongs to.
#[derive(Debug, Clone)]
pub struct PlacedOmt {
    /// The city_building definition ID (e.g. "2storyModern01").
    pub building_id: String,
    /// The overmap_terrain ID placed at this position (e.g. "2storyModern01_1_north").
    pub omt_id: String,
    /// Offset within the building layout: (dx, dy, dz).
    pub building_offset: (i32, i32, i32),
}

/// Maps bubble coordinates to building placement metadata.
/// Parallel to `WorldMap::bubbles` — not every bubble has an entry.
pub type BuildingPlacements = HashMap<(i32, i32, i32), PlacedOmt>;

/// Number of tiles per side of an overmap-terrain tile block.
pub const OMT_DIM: usize = 24;

/// Total tiles in an overmap-terrain tile block.
pub const OMT_SIZE: usize = OMT_DIM * OMT_DIM;

/// A 24×24 overmap-terrain tile block using struct-of-arrays storage.
///
/// Each property is a flat array indexed by `(y * OMT_DIM + x)`.
/// This is more cache-friendly than an array-of-structs for systems
/// that only read one or two properties (e.g. movement only reads
/// terrain move cost; line-of-sight only reads terrain opacity).
#[derive(Debug, Clone)]
pub struct OmtTerrain {
    /// Terrain type indices (index into DefRegistry terrain array).
    pub terrains: Box<[u32; OMT_SIZE]>,
    /// Furniture type indices.
    pub furniture: Box<[u32; OMT_SIZE]>,
}

impl OmtTerrain {
    /// Create an empty OMT block (all tiles default to 0 = empty/air).
    pub fn new() -> Self {
        Self {
            terrains: Box::new([0u32; OMT_SIZE]),
            furniture: Box::new([0u32; OMT_SIZE]),
        }
    }

    /// Index a tile within the block (no bounds check — caller guarantees valid coords).
    #[inline]
    fn index(x: u8, y: u8) -> usize {
        (y as usize) * OMT_DIM + (x as usize)
    }

    /// Get the terrain index at (x, y).
    #[inline]
    pub fn terrain(&self, x: u8, y: u8) -> u32 {
        self.terrains[Self::index(x, y)]
    }

    /// Get a mutable reference to the terrain index at (x, y).
    #[inline]
    pub fn terrain_mut(&mut self, x: u8, y: u8) -> &mut u32 {
        &mut self.terrains[Self::index(x, y)]
    }

    /// Get the furniture index at (x, y).
    #[inline]
    pub fn furniture(&self, x: u8, y: u8) -> u32 {
        self.furniture[Self::index(x, y)]
    }

    /// Get a mutable reference to the furniture index at (x, y).
    #[inline]
    pub fn furniture_mut(&mut self, x: u8, y: u8) -> &mut u32 {
        &mut self.furniture[Self::index(x, y)]
    }

    /// Set terrain for all tiles.
    pub fn fill_terrain(&mut self, terrain_id: u32) {
        self.terrains.fill(terrain_id);
    }

    /// Set furniture for all tiles.
    pub fn fill_furniture(&mut self, furniture_id: u32) {
        self.furniture.fill(furniture_id);
    }
}

impl Default for OmtTerrain {
    fn default() -> Self {
        Self::new()
    }
}

/// A map layer consisting of multiple bubbles arranged in a grid.
///
/// Keyed by `(bubble_x, bubble_y, z)` — each bubble is a 24×24 tile chunk.
/// Bubbles are loaded and cached on demand from the submap store.
#[derive(Debug, Clone, Default)]
pub struct WorldMap {
    /// Bubbles keyed by (bubble_x, bubble_y, z).
    pub bubbles: HashMap<(i32, i32, i32), OmtTerrain>,
    /// Dev/debug metadata: which building was placed at each bubble.
    /// Populated by dev-worldgen; empty in normal gameplay.
    pub placements: BuildingPlacements,
}

impl WorldMap {
    /// Create an empty world map.
    pub fn new() -> Self {
        Self {
            bubbles: HashMap::new(),
            placements: HashMap::new(),
        }
    }

    /// Get a reference to a bubble at the given coordinates.
    /// Returns `None` if the bubble hasn't been loaded yet.
    pub fn bubble(&self, bx: i32, by: i32, z: i32) -> Option<&OmtTerrain> {
        self.bubbles.get(&(bx, by, z))
    }

    /// Get a mutable reference to a bubble, creating it if absent.
    pub fn bubble_or_create(&mut self, bx: i32, by: i32, z: i32) -> &mut OmtTerrain {
        self.bubbles.entry((bx, by, z)).or_default()
    }

    /// Number of loaded bubbles.
    pub fn bubble_count(&self) -> usize {
        self.bubbles.len()
    }

    /// Remove all bubbles and placements (for map reset / new game).
    pub fn clear(&mut self) {
        self.bubbles.clear();
        self.placements.clear();
    }

    /// Record that a building OMT was placed at the given bubble position.
    pub fn mark_placement(
        &mut self,
        bx: i32,
        by: i32,
        bz: i32,
        building_id: String,
        omt_id: String,
        offset: (i32, i32, i32),
    ) {
        self.placements.insert(
            (bx, by, bz),
            PlacedOmt {
                building_id,
                omt_id,
                building_offset: offset,
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omt_new_all_zero() {
        let ot = OmtTerrain::new();
        for y in 0..OMT_DIM as u8 {
            for x in 0..OMT_DIM as u8 {
                assert_eq!(ot.terrain(x, y), 0);
                assert_eq!(ot.furniture(x, y), 0);
            }
        }
    }

    #[test]
    fn test_omt_set_and_get() {
        let mut ot = OmtTerrain::new();
        *ot.terrain_mut(5, 10) = 42;
        assert_eq!(ot.terrain(5, 10), 42);
        assert_eq!(ot.terrain(0, 0), 0); // other tiles unchanged
    }

    #[test]
    fn test_omt_fill() {
        let mut ot = OmtTerrain::new();
        ot.fill_terrain(7);
        assert_eq!(ot.terrain(0, 0), 7);
        assert_eq!(ot.terrain(23, 23), 7);
    }

    #[test]
    fn test_world_map_empty() {
        let wm = WorldMap::new();
        assert_eq!(wm.bubble_count(), 0);
    }

    #[test]
    fn test_world_map_create_and_retrieve() {
        let mut wm = WorldMap::new();
        let bg = wm.bubble_or_create(0, 0, 0);
        *bg.terrain_mut(12, 12) = 99;
        drop(bg);

        let bg = wm.bubble(0, 0, 0).unwrap();
        assert_eq!(bg.terrain(12, 12), 99);
        assert_eq!(wm.bubble_count(), 1);
    }

    #[test]
    fn test_world_map_multiple_bubbles() {
        let mut wm = WorldMap::new();
        wm.bubble_or_create(0, 0, 0);
        wm.bubble_or_create(1, 0, 0);
        wm.bubble_or_create(0, 1, 0);
        assert_eq!(wm.bubble_count(), 3);
    }

    #[test]
    fn test_world_map_missing_bubble() {
        let wm = WorldMap::new();
        assert!(wm.bubble(99, 99, 0).is_none());
    }

    #[test]
    fn test_world_map_clear() {
        let mut wm = WorldMap::new();
        wm.bubble_or_create(0, 0, 0);
        wm.clear();
        assert_eq!(wm.bubble_count(), 0);
    }
}
