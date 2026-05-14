//! Overmap generation region settings.
//!
//! Port of the C++ `region_settings` structure that controls terrain generation
//! parameters: river density, lake/ocean sizes, forest coverage, city placement,
//! road networks, and special placement.
//!
//! This is a Bevy [`Resource`] — insert it once during setup and read it from
//! generation step systems via `Res<OvermapRegionSettings>`.

use bevy_ecs::prelude::*;

/// Configuration controlling overmap terrain generation.
///
/// Every field corresponds to a C++ `region_settings` member that tunes how
/// the procedural generation places features on the overmap. The [`Default`]
/// implementation provides values matching CDDA's standard region.
///
/// # Cardinal directions
///
/// Several fields use 4-element arrays indexed by direction:
/// `[North, East, South, West]`. North is positive-y in overmap coordinates,
/// East is positive-x.
#[derive(Resource, Debug, Clone)]
pub struct OvermapRegionSettings {
    /// Controls how many rivers spawn. Range 0–4.
    pub river_scale: u32,

    /// Minimum cluster size (in overmap tiles) for lake placement.
    pub lake_size_min: usize,

    /// Noise threshold for lake placement. Higher = fewer lakes.
    pub lake_noise_threshold: f32,

    /// How many z-levels down lakes extend (typically negative).
    pub lake_depth: i32,

    /// If true, invert the lake noise to place lakes where noise is *low*.
    pub invert_lakes: bool,

    /// Noise threshold for ocean placement.
    pub ocean_noise_threshold: f32,

    /// Minimum cluster size for ocean placement.
    pub ocean_size_min: usize,

    /// How many z-levels down oceans extend.
    pub ocean_depth: i32,

    /// Where oceans start from each cardinal edge, in overmap coordinates.
    /// `None` means no ocean on that edge. `Some(-3)` means ocean starts
    /// 3 OMTs past the overmap boundary in that direction.
    pub ocean_start: [Option<i32>; 4],

    /// Noise threshold for standard forest placement.
    pub forest_noise_threshold: f32,

    /// Noise threshold for thick forest placement.
    pub forest_noise_threshold_thick: f32,

    /// Maximum forest noise value before terrain becomes something else.
    pub forest_max: f32,

    /// Per-direction forest density increase. Higher values push forest
    /// further from the edge. `[North, East, South, West]`.
    pub forest_increase: [f32; 4],

    /// Whether forest generation is enabled on this overmap.
    pub overmap_forest: bool,

    /// Target city size. Larger = bigger cities.
    pub city_size: i32,

    /// Distance between city centers, in overmap tiles.
    pub city_spacing: i32,

    /// Maximum urban density level (used for road density around cities).
    pub max_urban: i32,

    /// Per-direction urbanity increase. `[North, East, South, West]`.
    pub urban_increase: [i32; 4],

    /// If true, use megacity mode: 5 equidistant large cities per overmap.
    pub is_megacity: bool,

    /// Whether swamps are placed at all.
    pub place_swamps: bool,

    /// Noise threshold for swamp placement adjacent to water features.
    pub swamp_noise_threshold_adjacent: f32,

    /// Noise threshold for isolated swamp placement.
    pub swamp_noise_threshold_isolated: f32,

    /// Number of ravines to attempt to place.
    pub ravine_num: usize,

    /// Maximum length of a ravine, in overmap tiles.
    pub ravine_range: i32,

    /// Width of ravines in overmap tiles.
    pub ravine_width: i32,

    /// Depth ravines cut down (negative z-levels).
    pub ravine_depth: i32,

    /// Whether roads are placed.
    pub place_roads: bool,

    /// Whether railroads are placed.
    pub place_railroads: bool,

    /// If true, railroads are placed before roads in the pipeline.
    pub place_railroads_before_roads: bool,

    /// Whether overmap specials (labs, military bases, etc.) are placed.
    pub place_specials: bool,

    /// Minimum overmap tile count for a forest to get trails.
    pub forest_trail_min_size: usize,

    /// Chance (1-in-N) of a forest trail being placed through a forest.
    pub forest_trail_chance: i32,

    /// Minimum random interior points for forest trail pathfinding.
    pub forest_trail_random_point_min: i32,

    /// Maximum random interior points for forest trail pathfinding.
    pub forest_trail_random_point_max: i32,

    /// Scalar applied to forest size to determine random point count.
    pub forest_trail_random_point_size_scalar: i32,

    /// Chance (1-in-N) of a forest trail connecting at a border point.
    pub forest_trail_border_point_chance: i32,

    /// Minimum distance from a river where floodplain buffering starts.
    pub river_floodplain_buffer_dist_min: i32,

    /// Maximum distance from a river where floodplain buffering ends.
    pub river_floodplain_buffer_dist_max: i32,

    /// Maximum Chebyshev distance from a trail end to look for a road.
    pub trailhead_road_distance: i32,

    /// Chance (1-in-N) of a trailhead being placed at a trail end.
    pub trailhead_chance: i32,
}

impl Default for OvermapRegionSettings {
    fn default() -> Self {
        Self {
            river_scale: 1,
            lake_size_min: 20,
            lake_noise_threshold: 0.25,
            lake_depth: -4,
            invert_lakes: false,
            ocean_noise_threshold: 0.25,
            ocean_size_min: 200,
            ocean_depth: -4,
            ocean_start: [None, None, None, None],
            forest_noise_threshold: 0.2,
            forest_noise_threshold_thick: 0.25,
            forest_max: 0.395,
            forest_increase: [0.04, 0.0, 0.0, 0.02],
            overmap_forest: true,
            city_size: 8,
            city_spacing: 4,
            max_urban: 3,
            urban_increase: [0, 0, 0, 0],
            place_swamps: true,
            swamp_noise_threshold_adjacent: 0.3,
            swamp_noise_threshold_isolated: 0.25,
            is_megacity: false,
            ravine_num: 2,
            ravine_range: 30,
            ravine_width: 3,
            ravine_depth: -4,
            place_roads: true,
            place_railroads: true,
            place_railroads_before_roads: false,
            place_specials: true,
            forest_trail_min_size: 20,
            forest_trail_chance: 4,
            forest_trail_random_point_min: 2,
            forest_trail_random_point_max: 6,
            forest_trail_random_point_size_scalar: 50,
            forest_trail_border_point_chance: 2,
            river_floodplain_buffer_dist_min: 2,
            river_floodplain_buffer_dist_max: 6,
            trailhead_road_distance: 8,
            trailhead_chance: 8,
        }
    }
}
