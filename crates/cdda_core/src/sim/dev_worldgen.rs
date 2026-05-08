//! # Dev Worldgen — spawn one of every building for testing
//!
//! When enabled, generates a showcase world where every `city_building`
//! definition is placed in a grid. Buildings are sorted by ID, laid out
//! left-to-right with enough vertical spacing to avoid overlap.
//!
//! The player spawns at the top-left of the grid.

use bevy_ecs::prelude::Resource;
use crate::data::raw_defs::city_building::{CityBuildingDef, CityBuildingOvermap};
use crate::map::WorldMap;

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// DevWorldgenConfig
// ---------------------------------------------------------------------------

/// Resource that enables dev-worldgen mode.
///
/// Insert this before `AppState::WorldGen` to generate a building showcase
/// instead of a normal world.
#[derive(Resource, Debug, Clone)]
pub struct DevWorldgenConfig {
    /// Horizontal gap in OMT units between buildings.
    pub gap_x: i32,
    /// Vertical gap in OMT units between rows.
    pub gap_y: i32,
    /// Maximum buildings per row before wrapping.
    pub buildings_per_row: u32,
    /// Base X position for the grid (OMT coordinates).
    pub origin_x: i32,
    /// Base Y position for the grid (OMT coordinates).
    pub origin_y: i32,
    /// Z-level where buildings are placed.
    pub z: i32,
}

impl Default for DevWorldgenConfig {
    fn default() -> Self {
        Self {
            gap_x: 2,
            gap_y: 4,
            buildings_per_row: 10,
            origin_x: 0,
            origin_y: 0,
            z: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Building layout helpers
// ---------------------------------------------------------------------------

/// The bounding box of a building in OMT-grid coordinates (relative to its origin).
#[derive(Debug, Clone)]
struct BuildingExtent {
    /// Width in OMT units (max x - min x + 1).
    width: i32,
    /// Depth in OMT units (max y - min y + 1).
    depth: i32,
    /// Height in z-levels (max z - min z + 1).
    height: i32,
    /// Offset to the origin (typically [0, 0, 0]).
    min_x: i32,
    min_y: i32,
}

fn compute_extent(overmaps: &[CityBuildingOvermap]) -> BuildingExtent {
    if overmaps.is_empty() {
        return BuildingExtent {
            width: 1,
            depth: 1,
            height: 1,
            min_x: 0,
            min_y: 0,
        };
    }

    let min_x = overmaps.iter().map(|o| o.point[0]).min().unwrap_or(0);
    let max_x = overmaps.iter().map(|o| o.point[0]).max().unwrap_or(0);
    let min_y = overmaps.iter().map(|o| o.point[1]).min().unwrap_or(0);
    let max_y = overmaps.iter().map(|o| o.point[1]).max().unwrap_or(0);
    let min_z = overmaps
        .iter()
        .map(|o| o.point.get(2).copied().unwrap_or(0))
        .min()
        .unwrap_or(0);
    let max_z = overmaps
        .iter()
        .map(|o| o.point.get(2).copied().unwrap_or(0))
        .max()
        .unwrap_or(0);

    BuildingExtent {
        width: max_x - min_x + 1,
        depth: max_y - min_y + 1,
        height: max_z - min_z + 1,
        min_x,
        min_y,
    }
}

// ---------------------------------------------------------------------------
// Dev worldgen entry point
// ---------------------------------------------------------------------------

/// Generate a `WorldMap` containing one of every city building, arranged in a grid.
///
/// Buildings are sorted by ID for deterministic ordering. Each building's
/// overmap tiles are placed at the grid position computed from the config.
///
/// Returns the number of buildings placed.
pub fn generate_dev_worldmap(
    world_map: &mut WorldMap,
    city_buildings: &HashMap<
        crate::data::raw_types::DefId<CityBuildingDef>,
        std::sync::Arc<CityBuildingDef>,
    >,
    config: &DevWorldgenConfig,
) -> usize {
    // Collect and sort building IDs for deterministic layout
    let mut building_ids: Vec<String> = city_buildings
        .keys()
        .map(|k| k.as_str().to_string())
        .collect();
    building_ids.sort();

    let mut current_x = config.origin_x;
    let mut current_y = config.origin_y;
    let mut max_row_height: i32 = 0;
    let mut placed_count: usize = 0;
    let mut col: u32 = 0;

    for bid in &building_ids {
        let Some(building) = city_buildings
            .values()
            .find(|b| b.id.as_str() == bid.as_str())
        else {
            continue;
        };
        let Some(overmaps) = &building.overmaps else {
            continue;
        };
        if overmaps.is_empty() {
            continue;
        }

        let extent = compute_extent(overmaps);

        // Wrap to next row if we've hit the column limit
        if col >= config.buildings_per_row && col > 0 {
            current_x = config.origin_x;
            current_y += max_row_height + config.gap_y;
            max_row_height = 0;
            col = 0;
        }

        // Place each OMT of this building
        for omt in overmaps {
            // OMT coordinates in the world
            let omt_x = current_x + omt.point[0] - extent.min_x;
            let omt_y = current_y + omt.point[1] - extent.min_y;
            let omt_z = config.z + omt.point.get(2).copied().unwrap_or(0);

            // Create the bubble with default terrain
            let bubble = world_map.bubble_or_create(omt_x, omt_y, omt_z);
            // Fill with a placeholder terrain (index 1 = something visible, 0 = empty)
            bubble.fill_terrain(1);

            // Record placement metadata
            world_map.mark_placement(
                omt_x,
                omt_y,
                omt_z,
                bid.clone(),
                omt.overmap.clone(),
                (
                    omt.point[0],
                    omt.point[1],
                    omt.point.get(2).copied().unwrap_or(0),
                ),
            );
        }

        // Advance grid position
        current_x += extent.width + config.gap_x;
        max_row_height = max_row_height.max(extent.depth);
        col += 1;
        placed_count += 1;
    }

    placed_count
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::raw_types::DefId;

    fn make_def_id(s: &str) -> DefId<CityBuildingDef> {
        DefId::new(s.to_string())
    }

    fn make_building(id: &str, overmaps: Vec<CityBuildingOvermap>) -> CityBuildingDef {
        CityBuildingDef {
            id: make_def_id(id),
            locations: Some(vec!["land".to_string()]),
            overmaps: Some(overmaps),
            flags: None,
        }
    }

    fn make_omt(x: i32, y: i32, z: i32, omt_id: &str) -> CityBuildingOvermap {
        CityBuildingOvermap {
            point: vec![x, y, z],
            overmap: omt_id.to_string(),
        }
    }

    #[test]
    fn test_compute_extent_single_tile() {
        let overmaps = vec![make_omt(0, 0, 0, "house_1")];
        let extent = compute_extent(&overmaps);
        assert_eq!(extent.width, 1);
        assert_eq!(extent.depth, 1);
        assert_eq!(extent.height, 1);
    }

    #[test]
    fn test_compute_extent_multi_tile() {
        let overmaps = vec![
            make_omt(0, 0, 0, "north"),
            make_omt(0, 0, 1, "roof"),
            make_omt(1, 0, 0, "south"),
            make_omt(1, 0, 1, "roof_south"),
        ];
        let extent = compute_extent(&overmaps);
        assert_eq!(extent.width, 2);
        assert_eq!(extent.depth, 1);
        assert_eq!(extent.height, 2);
    }

    #[test]
    fn test_compute_extent_with_negative_offset() {
        let overmaps = vec![
            make_omt(-1, 0, 0, "west"),
            make_omt(0, 0, 0, "center"),
            make_omt(1, 0, 0, "east"),
        ];
        let extent = compute_extent(&overmaps);
        assert_eq!(extent.width, 3);
        assert_eq!(extent.min_x, -1);
    }

    #[test]
    fn test_empty_overmaps() {
        let extent = compute_extent(&[]);
        assert_eq!(extent.width, 1);
        assert_eq!(extent.depth, 1);
    }

    #[test]
    fn test_generate_empty_buildings() {
        let mut wm = WorldMap::new();
        let buildings: HashMap<DefId<CityBuildingDef>, std::sync::Arc<CityBuildingDef>> =
            HashMap::new();
        let config = DevWorldgenConfig::default();
        let count = generate_dev_worldmap(&mut wm, &buildings, &config);
        assert_eq!(count, 0);
        assert_eq!(wm.bubble_count(), 0);
    }

    #[test]
    fn test_generate_single_building() {
        let mut wm = WorldMap::new();
        let mut buildings = HashMap::new();
        let def = make_building("test_house", vec![make_omt(0, 0, 0, "house_north")]);
        let id = def.id.clone();
        buildings.insert(id, std::sync::Arc::new(def));

        let config = DevWorldgenConfig::default();
        let count = generate_dev_worldmap(&mut wm, &buildings, &config);
        assert_eq!(count, 1);
        assert_eq!(wm.bubble_count(), 1);
        assert!(wm.bubble(0, 0, 0).is_some());
        assert_eq!(wm.placements.len(), 1);
    }

    #[test]
    fn test_generate_multi_tile_building() {
        let mut wm = WorldMap::new();
        let mut buildings = HashMap::new();
        let def = make_building(
            "2story",
            vec![
                make_omt(0, 0, 0, "2story_1_north"),
                make_omt(0, 0, 1, "2story_2_north"),
            ],
        );
        let id = def.id.clone();
        buildings.insert(id, std::sync::Arc::new(def));

        let config = DevWorldgenConfig::default();
        let count = generate_dev_worldmap(&mut wm, &buildings, &config);
        assert_eq!(count, 1);
        // Two OMTs at different z-levels, same x,y
        assert_eq!(wm.bubble_count(), 2);
        assert!(wm.bubble(0, 0, 0).is_some());
        assert!(wm.bubble(0, 0, 1).is_some());
    }

    #[test]
    fn test_generate_multiple_buildings_grid_layout() {
        let mut wm = WorldMap::new();
        let mut buildings = HashMap::new();

        let def_a = make_building("a_house", vec![make_omt(0, 0, 0, "a_tile")]);
        buildings.insert(def_a.id.clone(), std::sync::Arc::new(def_a));

        let def_b = make_building(
            "b_wide",
            vec![make_omt(0, 0, 0, "b_left"), make_omt(1, 0, 0, "b_right")],
        );
        buildings.insert(def_b.id.clone(), std::sync::Arc::new(def_b));

        let config = DevWorldgenConfig::default();
        let count = generate_dev_worldmap(&mut wm, &buildings, &config);
        assert_eq!(count, 2);

        // Building "a" at (0, 0)
        assert!(wm.bubble(0, 0, 0).is_some());
        // Building "b" starts at (0 + 1 + gap(2), 0) = (3, 0)
        assert!(wm.bubble(3, 0, 0).is_some());
        assert!(wm.bubble(4, 0, 0).is_some());
    }

    #[test]
    fn test_row_wrapping() {
        let mut wm = WorldMap::new();
        let mut buildings = HashMap::new();

        for i in 0..5 {
            let def = make_building(&format!("b{i}"), vec![make_omt(0, 0, 0, "tile")]);
            buildings.insert(def.id.clone(), std::sync::Arc::new(def));
        }

        let config = DevWorldgenConfig {
            buildings_per_row: 3,
            gap_x: 2,
            gap_y: 4,
            ..Default::default()
        };
        let count = generate_dev_worldmap(&mut wm, &buildings, &config);
        assert_eq!(count, 5);

        // Row 0: b0 at (0,0), b1 at (3,0), b2 at (6,0)
        assert!(wm.bubble(0, 0, 0).is_some());
        assert!(wm.bubble(3, 0, 0).is_some());
        assert!(wm.bubble(6, 0, 0).is_some());
        // Row 1: b3 at (0, 5), b4 at (3, 5)
        assert!(wm.bubble(0, 5, 0).is_some());
        assert!(wm.bubble(3, 5, 0).is_some());
    }
}
