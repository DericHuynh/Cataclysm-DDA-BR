//! Overmap region settings — mirrors CDDA master's `region_settings` system.
//!
//! `OvermapRegionSettings` is a flat [`Resource`] that holds every generation
//! parameter referenced anywhere in the C++ overmap generation pipeline
//! (`overmap.cpp`, `overmap_water.cpp`, `overmap_highway.cpp`, `overmap_city.cpp`,
//! etc.).  When a boolean gate (e.g. `overmap_forest`) is `false`, the
//! corresponding sub-struct fields are ignored and that generation pass is
//! skipped entirely — exactly as the C++ code does via `std::optional`.
//!
//! # Field ordering
//!
//! Directional arrays use **N-E-S-W** order, matching the C++
//! `om_direction::type` discriminant: `North = 0`, `East = 1`, `South = 2`,
//! `West = 3`.

use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// Forest settings — `region_settings_forest`
// ---------------------------------------------------------------------------
/// Parameters controlling forest placement and swamp generation.
///
/// Only used when [`OvermapRegionSettings::overmap_forest`] is `true`.
/// C++ defaults are noted on each field.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsForest {
    /// Per-directional forest growth multiplier [N, E, S, W].
    /// C++ default: `[0.0, 0.0, 0.0, 0.0]`.
    pub forest_increase: [f32; 4],

    /// Perlin-noise threshold for basic forest coverage.
    /// C++ default: `0.25`.
    pub noise_threshold_forest: f32,

    /// Perlin-noise threshold for *thick* (dense) forest.
    /// C++ default: `0.30`.
    pub noise_threshold_forest_thick: f32,

    /// Hard cap on the forest-cover fraction.  Cities are difficult to
    /// generate above ~0.4.  Set from `max_forest` in JSON.
    /// No C++ default — `0.4` is the documented soft ceiling.
    pub max_forest: f32,

    /// Buffer distance from a river to the nearest swamp tile (min).
    /// C++ default: `3`.
    pub river_floodplain_buffer_distance_min: i32,

    /// Buffer distance from a river to the nearest swamp tile (max).
    /// C++ default: `15`.
    pub river_floodplain_buffer_distance_max: i32,

    /// Noise threshold for swamp tiles adjacent to an existing water body.
    /// C++ default: `0.30`.
    pub swamp_noise_threshold_adjacent: f32,

    /// Noise threshold for swamp tiles isolated from any water body.
    /// C++ default: `0.60`.
    pub swamp_noise_threshold_isolated: f32,
}

impl Default for RegionSettingsForest {
    fn default() -> Self {
        Self {
            forest_increase: [0.0; 4],
            noise_threshold_forest: 0.25,
            noise_threshold_forest_thick: 0.30,
            max_forest: 0.4,
            river_floodplain_buffer_distance_min: 3,
            river_floodplain_buffer_distance_max: 15,
            swamp_noise_threshold_adjacent: 0.30,
            swamp_noise_threshold_isolated: 0.60,
        }
    }
}

// ---------------------------------------------------------------------------
// Lake settings — `region_settings_lake`
// ---------------------------------------------------------------------------
/// Parameters controlling lake placement.
///
/// Only used when [`OvermapRegionSettings::overmap_lake`] is `true`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsLake {
    /// Perlin-noise threshold for lake generation.
    /// C++ default: `0.25`.
    pub noise_threshold_lake: f64,

    /// Minimum lake size in overmap tiles.
    /// C++ default: `20`.
    pub lake_size_min: usize,

    /// Z-level at which lake *beds* are placed (negative = below ground).
    /// C++ default: `-5`.
    pub lake_depth: i32,

    /// When `true`, inverts the lake mask so that what would be water
    /// becomes land and vice versa.
    /// C++ default: `false`.
    pub invert_lakes: bool,
}

impl Default for RegionSettingsLake {
    fn default() -> Self {
        Self {
            noise_threshold_lake: 0.25,
            lake_size_min: 20,
            lake_depth: -5,
            invert_lakes: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Ocean settings — `region_settings_ocean`
// ---------------------------------------------------------------------------
/// Parameters controlling ocean placement.
///
/// Only used when [`OvermapRegionSettings::overmap_ocean`] is `true`.
/// `overmap_ocean` is automatically derived: if *any* `ocean_start_*` is
/// `Some`, oceans are enabled.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsOcean {
    /// Absolute OMT coordinate where ocean starts on the **north** edge.
    /// `None` means no ocean on that side.  C++ default: `None`.
    pub ocean_start_north: Option<i32>,

    /// Absolute OMT coordinate where ocean starts on the **east** edge.
    /// C++ default: `None`.
    pub ocean_start_east: Option<i32>,

    /// Absolute OMT coordinate where ocean starts on the **south** edge.
    /// C++ default: `None`.
    pub ocean_start_south: Option<i32>,

    /// Absolute OMT coordinate where ocean starts on the **west** edge.
    /// C++ default: `None`.
    pub ocean_start_west: Option<i32>,

    /// Perlin-noise threshold for ocean water tiles.
    /// C++ default: `0.25`.
    pub noise_threshold_ocean: f32,

    /// Minimum ocean size in overmap tiles.
    /// C++ default: `100`.
    pub ocean_size_min: usize,

    /// Z-level for the ocean floor.
    /// C++ default: `-9`.
    pub ocean_depth: i32,
}

impl RegionSettingsOcean {
    /// Returns `true` when at least one edge has an ocean start coordinate,
    /// matching the C++ convention where a present `std::optional` enables oceans.
    pub fn is_enabled(&self) -> bool {
        self.ocean_start_north.is_some()
            || self.ocean_start_east.is_some()
            || self.ocean_start_south.is_some()
            || self.ocean_start_west.is_some()
    }
}

impl Default for RegionSettingsOcean {
    fn default() -> Self {
        Self {
            ocean_start_north: None,
            ocean_start_east: None,
            ocean_start_south: None,
            ocean_start_west: None,
            noise_threshold_ocean: 0.25,
            ocean_size_min: 100,
            ocean_depth: -9,
        }
    }
}

// ---------------------------------------------------------------------------
// River settings — `region_settings_river`
// ---------------------------------------------------------------------------
/// Parameters controlling river placement.
///
/// Only used when [`OvermapRegionSettings::overmap_river`] is `true`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsRiver {
    /// River "scale" factor — higher values produce more intricate rivers.
    /// C++ default: `1`.
    pub river_scale: i32,

    /// Probability of river placement, expressed as an *x-in-y* frequency.
    /// C++ default: `1.5` (stored as `double`).
    pub river_frequency: f64,

    /// `one_in()` chance for a river branch to split off.
    /// C++ default: `64` (i.e. ~1.56% per opportunity).
    pub river_branch_chance: i32,

    /// `one_in()` chance for a river branch to re-merge.
    /// C++ default: `4` (i.e. 25% per opportunity).
    pub river_branch_remerge_chance: i32,

    /// Amount by which the branch's scale is decreased relative to the parent.
    /// C++ default: `1`.
    pub river_branch_scale_decrease: i32,
}

impl Default for RegionSettingsRiver {
    fn default() -> Self {
        Self {
            river_scale: 1,
            river_frequency: 1.5,
            river_branch_chance: 64,
            river_branch_remerge_chance: 4,
            river_branch_scale_decrease: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// City settings — `region_settings_city`
// ---------------------------------------------------------------------------
/// Parameters controlling city and road-network placement.
///
/// Only used when [`OvermapRegionSettings::city_spec`] is `true`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsCity {
    /// Overmap-cell spacing between city centres (`op_city_spacing`).
    /// C++ default: `4`.
    pub city_spacing: i32,

    /// Base city size in overmap tiles (`op_city_size`).
    /// C++ default: `8`.
    pub city_size: i32,

    /// When `true`, the urban sprawl is effectively boundless.
    /// C++ default: `false`.
    pub is_megacity: bool,

    /// Gaussian radius for shop placement within the city.
    /// CDDA default: `30` (≈ `city_size * 2` for the 8-tile default).
    pub shop_radius: i32,

    /// Gaussian sigma for shop placement spread.
    /// CDDA default: `20` (≈ `city_size` for the 8-tile default).
    pub shop_sigma: i32,

    /// Gaussian radius for park placement.
    /// CDDA default: identical to `shop_radius` (i.e. `30`).
    pub park_radius: i32,

    /// Gaussian sigma for park spread across the city.
    /// CDDA default: `100 - park_radius` (i.e. `70`).
    pub park_sigma: i32,
}

impl Default for RegionSettingsCity {
    fn default() -> Self {
        Self {
            city_spacing: 4,
            city_size: 8,
            is_megacity: false,
            shop_radius: 30,
            shop_sigma: 20,
            park_radius: 30,
            park_sigma: 70,
        }
    }
}

// ---------------------------------------------------------------------------
// Ravine settings — `region_settings_ravine`
// ---------------------------------------------------------------------------
/// Parameters controlling ravine (rift/crack) placement.
///
/// Only used when [`OvermapRegionSettings::overmap_ravine`] is `true`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsRavine {
    /// How many ravines to attempt (`num_ravines`).
    /// C++ default: `0`.
    pub ravine_num: i32,

    /// Maximum length a ravine can reach before it terminates.
    /// C++ default: `45`.
    pub ravine_range: i32,

    /// Width in overmap tiles.
    /// C++ default: `1`.
    pub ravine_width: i32,

    /// Z-level depth (negative = below ground).
    /// C++ default: `-3`.
    pub ravine_depth: i32,
}

impl Default for RegionSettingsRavine {
    fn default() -> Self {
        Self {
            ravine_num: 0,
            ravine_range: 45,
            ravine_width: 1,
            ravine_depth: -3,
        }
    }
}

// ---------------------------------------------------------------------------
// Forest trail settings — `region_settings_forest_trail`
// ---------------------------------------------------------------------------
/// Parameters controlling forest-trail placement.
///
/// Only used when [`OvermapRegionSettings::forest_trail`] is `true`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionSettingsForestTrail {
    /// `one_in(chance)` per qualifying forest to get a trail system.
    /// C++ default: `1` (≈ always).
    pub chance: i32,

    /// `one_in(border_point_chance)` per border point candidate.
    /// C++ default: `2` (≈ 50%).
    pub border_point_chance: i32,

    /// Minimum forest size (in OMT tiles) for a trail system to spawn.
    /// C++ default: `50`.
    pub minimum_forest_size: usize,

    /// Minimum number of random interior points to scatter.
    /// C++ default: `4`.
    pub random_point_min: i32,

    /// Maximum number of random interior points to scatter.
    /// C++ default: `50`.
    pub random_point_max: i32,

    /// Scalar applied to forest size to determine number of random points.
    /// C++ default: `100`.
    pub random_point_size_scalar: i32,

    /// `one_in()` chance for a trailhead to spawn near a road (`trailhead_chance`).
    /// C++ default: `1` (≈ always when conditions are met).
    pub trailhead_chance: i32,

    /// Maximum distance from a trail to a road for trailhead placement
    /// (`trailhead_road_distance`).
    /// C++ default: `6`.
    pub trailhead_road_distance: i32,
}

impl Default for RegionSettingsForestTrail {
    fn default() -> Self {
        Self {
            chance: 1,
            border_point_chance: 2,
            minimum_forest_size: 50,
            random_point_min: 4,
            random_point_max: 50,
            random_point_size_scalar: 100,
            trailhead_chance: 1,
            trailhead_road_distance: 6,
        }
    }
}

// ---------------------------------------------------------------------------
// OvermapRegionSettings — top-level resource
// ---------------------------------------------------------------------------

/// Master resource holding every overmap-generation parameter.
///
/// Insert this as a [`Resource`] before running the generation pipeline.
/// The `Default` impl returns CDDA's stock defaults (roughly equivalent to the
/// `"default"` region in `regional_map_settings.json`).
///
/// # Boolean gates
///
/// | Field | C++ check | When `false` |
/// |---|---|---|
/// | `overmap_forest` | `settings->overmap_forest` | Skips `place_forests()` and `place_swamps()`. |
/// | `overmap_lake` | `settings->overmap_lake` | Skips `place_lakes()`. |
/// | `overmap_ocean` | derived from `ocean_start_*` | Skips `place_oceans()`. |
/// | `overmap_river` | `settings->overmap_river` | Skips `place_rivers()`. |
/// | `overmap_highway` | `settings->overmap_highway` | Skips `place_highways()`. |
/// | `overmap_ravine` | `settings->overmap_ravine` | Skips `place_ravines()`. |
/// | `city_spec` | `settings->city_spec` | Skips `place_cities()` and `build_cities()`. |
/// | `forest_trail` | `settings->forest_trail` | Skips `place_forest_trails()` and `place_forest_trailheads()`. |
/// | `place_roads` | direct bool | Skips `place_roads()`. |
/// | `place_railroads` | direct bool | Skips `place_railroads()`. |
/// | `place_specials` | direct bool | Skips `place_specials()`. |
/// | `neighbor_connections` | direct bool | Skips `populate_connections_out_from_neighbors()`. |
#[derive(Resource, Debug, Clone)]
pub struct OvermapRegionSettings {
    // ---- Forest ----
    /// Master toggle: enable forest and swamp generation.
    /// C++: `settings->overmap_forest.has_value()`.
    pub overmap_forest: bool,
    /// Detailed forest parameters.  Ignored when `overmap_forest` is `false`.
    pub forest: RegionSettingsForest,
    /// When `true` (and `overmap_forest` is also `true`), runs `place_swamps()`.
    /// C++ default: `true`.
    pub place_swamps: bool,

    // ---- Lake ----
    /// Master toggle: enable lake generation.
    /// C++: `settings->overmap_lake.has_value()`.
    pub overmap_lake: bool,
    /// Detailed lake parameters.  Ignored when `overmap_lake` is `false`.
    pub lake: RegionSettingsLake,

    // ---- Ocean ----
    /// Master toggle: enable ocean generation.
    /// Derived: `true` when any `ocean.ocean_start_*` is `Some`.
    pub overmap_ocean: bool,
    /// Detailed ocean parameters.  Ignored when `overmap_ocean` is `false`.
    pub ocean: RegionSettingsOcean,

    // ---- River ----
    /// Master toggle: enable river generation.
    /// C++: `settings->overmap_river.has_value()`.
    pub overmap_river: bool,
    /// Detailed river parameters.  Ignored when `overmap_river` is `false`.
    pub river: RegionSettingsRiver,

    // ---- City ----
    /// Master toggle: enable city (and road) placement.
    /// C++: `settings->city_spec.has_value()`.
    pub city_spec: bool,
    /// Detailed city parameters.  Ignored when `city_spec` is `false`.
    pub city: RegionSettingsCity,

    // ---- Urban directional ----
    /// Maximum urbanity value cap (`max_urbanity`).
    /// C++ default: `8`.
    pub max_urban: i32,
    /// Per-directional urbanity increase [N, E, S, W].
    /// C++ default: `[0.0, 0.0, 0.0, 0.0]`.
    pub urban_increase: [f32; 4],

    // ---- Ravine ----
    /// Master toggle: enable ravine generation.
    /// C++: `settings->overmap_ravine.has_value()`.
    pub overmap_ravine: bool,
    /// Detailed ravine parameters.  Ignored when `overmap_ravine` is `false`.
    pub ravine: RegionSettingsRavine,

    // ---- Highway ----
    /// Master toggle: enable highway generation.
    /// C++: `settings->overmap_highway.has_value()`.
    pub overmap_highway: bool,

    // ---- Forest trail ----
    /// Master toggle: enable forest-trail generation.
    /// C++: `settings->forest_trail.has_value()`.
    pub forest_trail: bool,
    /// Detailed forest-trail parameters.  Ignored when `forest_trail` is `false`.
    pub forest_trail_settings: RegionSettingsForestTrail,

    // ---- Road / Rail ----
    /// Place roads (inter-city and intra-city).
    /// C++ default: `true`.
    pub place_roads: bool,
    /// Place railroads.
    /// C++ default: `false`.
    pub place_railroads: bool,
    /// When `true`, railroads are placed *before* roads (affects intersection
    /// resolution).  C++ default: `false`.
    pub place_railroads_before_roads: bool,

    // ---- Neighbor connections ----
    /// When `true`, stitch road/river/etc. connections across overmap boundaries.
    /// C++ default: `true`.
    pub neighbor_connections: bool,

    // ---- Specials ----
    /// Place overmap specials (landmarks, labs, etc.).
    /// C++ default: `true`.
    pub place_specials: bool,
}

impl Default for OvermapRegionSettings {
    fn default() -> Self {
        let ocean = RegionSettingsOcean::default();
        Self {
            // Forest
            overmap_forest: false,
            forest: RegionSettingsForest::default(),
            place_swamps: true,

            // Lake
            overmap_lake: false,
            lake: RegionSettingsLake::default(),

            // Ocean — auto-derived
            overmap_ocean: ocean.is_enabled(),
            ocean,

            // River
            overmap_river: false,
            river: RegionSettingsRiver::default(),

            // City
            city_spec: false,
            city: RegionSettingsCity::default(),

            // Urban directional
            max_urban: 8,
            urban_increase: [0.0; 4],

            // Ravine
            overmap_ravine: false,
            ravine: RegionSettingsRavine::default(),

            // Highway
            overmap_highway: false,

            // Forest trail
            forest_trail: false,
            forest_trail_settings: RegionSettingsForestTrail::default(),

            // Road / Rail
            place_roads: true,
            place_railroads: false,
            place_railroads_before_roads: false,

            // Neighbor connections
            neighbor_connections: true,

            // Specials
            place_specials: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensure the `Default` impl matches the C++ defaults documented in
    /// `regional_settings.h` and `regional_settings.cpp`.
    #[test]
    fn defaults_match_cpp() {
        let s = OvermapRegionSettings::default();

        // Forest
        assert!(!s.overmap_forest);
        assert_eq!(s.forest.forest_increase, [0.0; 4]);
        assert_eq!(s.forest.noise_threshold_forest, 0.25);
        assert_eq!(s.forest.noise_threshold_forest_thick, 0.30);
        assert_eq!(s.forest.max_forest, 0.4);
        assert_eq!(s.forest.river_floodplain_buffer_distance_min, 3);
        assert_eq!(s.forest.river_floodplain_buffer_distance_max, 15);
        assert_eq!(s.forest.swamp_noise_threshold_adjacent, 0.30);
        assert_eq!(s.forest.swamp_noise_threshold_isolated, 0.60);
        assert!(s.place_swamps);

        // Lake
        assert!(!s.overmap_lake);
        assert_eq!(s.lake.noise_threshold_lake, 0.25);
        assert_eq!(s.lake.lake_size_min, 20);
        assert_eq!(s.lake.lake_depth, -5);
        assert!(!s.lake.invert_lakes);

        // Ocean
        assert!(!s.overmap_ocean);
        assert!(s.ocean.ocean_start_north.is_none());
        assert_eq!(s.ocean.noise_threshold_ocean, 0.25);
        assert_eq!(s.ocean.ocean_size_min, 100);
        assert_eq!(s.ocean.ocean_depth, -9);

        // River
        assert!(!s.overmap_river);
        assert_eq!(s.river.river_scale, 1);
        assert_eq!(s.river.river_frequency, 1.5);
        assert_eq!(s.river.river_branch_chance, 64);
        assert_eq!(s.river.river_branch_remerge_chance, 4);
        assert_eq!(s.river.river_branch_scale_decrease, 1);

        // City
        assert!(!s.city_spec);
        assert_eq!(s.city.city_spacing, 4);
        assert_eq!(s.city.city_size, 8);
        assert!(!s.city.is_megacity);
        assert_eq!(s.city.shop_radius, 30);
        assert_eq!(s.city.shop_sigma, 20);
        assert_eq!(s.city.park_radius, 30);
        assert_eq!(s.city.park_sigma, 70);

        // Urban directional
        assert_eq!(s.max_urban, 8);
        assert_eq!(s.urban_increase, [0.0; 4]);

        // Ravine
        assert!(!s.overmap_ravine);
        assert_eq!(s.ravine.ravine_num, 0);
        assert_eq!(s.ravine.ravine_range, 45);
        assert_eq!(s.ravine.ravine_width, 1);
        assert_eq!(s.ravine.ravine_depth, -3);

        // Highway
        assert!(!s.overmap_highway);

        // Forest trail
        assert!(!s.forest_trail);
        assert_eq!(s.forest_trail_settings.chance, 1);
        assert_eq!(s.forest_trail_settings.border_point_chance, 2);
        assert_eq!(s.forest_trail_settings.minimum_forest_size, 50);
        assert_eq!(s.forest_trail_settings.random_point_min, 4);
        assert_eq!(s.forest_trail_settings.random_point_max, 50);
        assert_eq!(s.forest_trail_settings.random_point_size_scalar, 100);
        assert_eq!(s.forest_trail_settings.trailhead_chance, 1);
        assert_eq!(s.forest_trail_settings.trailhead_road_distance, 6);

        // Road / Rail
        assert!(s.place_roads);
        assert!(!s.place_railroads);
        assert!(!s.place_railroads_before_roads);
        assert!(s.neighbor_connections);
        assert!(s.place_specials);
    }

    /// `RegionSettingsOcean::is_enabled()` must be `true` exactly when at
    /// least one edge has a start coordinate.
    #[test]
    fn ocean_enabled_derivation() {
        let mut ocean = RegionSettingsOcean::default();
        assert!(!ocean.is_enabled());

        ocean.ocean_start_east = Some(0);
        assert!(ocean.is_enabled());
        assert!(
            OvermapRegionSettings {
                ocean: ocean.clone(),
                overmap_ocean: ocean.is_enabled(),
                ..Default::default()
            }
            .overmap_ocean
        );
    }
}
