//! Generation pipeline — system sets, plugin, and state.
//!
//! ## Ordering — 1:1 with CDDA master `overmap::generate()`
//!
//! C++ generation order (overmap.cpp L932–1060):
//! 1. populate_connections_out_from_neighbors (if neighbor_connections)
//! 2. place_rivers → place_lakes → place_oceans → place_forests
//!    → place_swamps → place_ravines → polish_river(#1)
//! 3. place_highways
//! 4. place_cities
//! 5. place_highway_interchanges → build_cities
//! 6. place_forest_trails → place_roads/place_railroads (order per flag)
//! 7. place_specials
//! 8. finalize_highways → place_forest_trailheads → polish_river(#2)
//! 9. generate_sub → generate_over
//! 10. place_mongroups → place_radios
//!
//! Within each set, systems are **chained** so they execute sequentially.
//! This guarantees deterministic output matching C++. Each system uses
//! internal `par_iter()` parallelism over chunks for performance.
//!
//! Mods inject additional generation steps by adding systems to the
//! appropriate set without modifying this file.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

use crate::region_settings::OvermapRegionSettings;
use crate::steps;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Tracks the phase of overmap generation for the current overmap.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum OvermapGenPhase {
    /// No generation running.
    #[default]
    Idle,
    /// Generation systems are active.
    Generating,
    /// All steps complete.
    Complete,
}

/// Default noise seed for deterministic world generation.
/// Change this value to generate a different world layout.
pub const DEFAULT_NOISE_SEED: u32 = 1920237457;

/// Configuration for generation.
#[derive(Resource, Debug, Clone)]
pub struct OvermapGenConfig {
    /// Noise seed for terrain generation.
    pub noise_seed: u32,
    /// Which overmap position to generate.
    pub om_x: i32,
    pub om_y: i32,
    /// Region settings ID (references DefRegistry.region_settings).
    pub region_id: String,
}

impl Default for OvermapGenConfig {
    fn default() -> Self {
        Self {
            noise_seed: DEFAULT_NOISE_SEED,
            om_x: 0,
            om_y: 0,
            region_id: "default".into(),
        }
    }
}

/// Entity marker for the overmap entity being generated.
/// Spawned once per overmap; chunk entities are children.
#[derive(Component)]
pub struct OvermapEntity {
    pub om_x: i32,
    pub om_y: i32,
}

// ---------------------------------------------------------------------------
// System sets — exact match for C++ `overmap::generate()` order
// ---------------------------------------------------------------------------

/// Ordered phases of overmap generation.
///
/// Systems within a set are `.chain()`-ed to guarantee deterministic,
/// C++-compatible sequential execution.  Each system still uses internal
/// `par_iter()` parallelism for chunk-level throughput.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum OvermapGenSet {
    /// Fill all chunks with default terrain.
    InitBase,
    /// Populate cross-overmap connection exits (mirrored from neighbor edges).
    NeighborConnections,
    /// Natural terrain, chained: rivers → lakes → oceans → forests
    /// → swamps → ravines → polish_river(#1).
    NaturalTerrain,
    /// Highway path placement (before cities so cities avoid highways).
    Highways,
    /// City center placement.
    Cities,
    /// Post-city: highway interchanges (after city centers placed)
    /// then city street grids (after interchanges).
    PostCities,
    /// Roads, railroads, forest trails via pathfinding.
    /// Two sub-chains with run_if for place_railroads_before_roads flag.
    Connections,
    /// Buildings within cities, overmap specials.
    Structures,
    /// Pre-underground finalization: highways → trailheads → polish_river(#2).
    PreUnderground,
    /// Underground layers (z < 0).
    Underground,
    /// Elevated layers (z > 0).
    Elevated,
    /// Monster groups, radio towers (no chunk writes — safe to parallelize).
    Population,
    /// Finalize and fire completion.
    Finalize,
}

// ---------------------------------------------------------------------------
// Run conditions
// ---------------------------------------------------------------------------

/// True when region settings want railroads before roads.
fn railroads_before_roads(settings: Option<Res<OvermapRegionSettings>>) -> bool {
    settings.map_or(false, |s| s.place_railroads_before_roads)
}

/// True when region settings want roads before railroads (the default).
fn roads_before_railroads(settings: Option<Res<OvermapRegionSettings>>) -> bool {
    settings.map_or(true, |s| !s.place_railroads_before_roads)
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct OvermapGenPlugin;

impl Plugin for OvermapGenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<OvermapGenPhase>();
        app.init_resource::<OvermapGenConfig>();
        app.init_resource::<cdda_overmap::index::ChunkIndex>();

        // Chunk index maintenance — keeps the O(1) lookup up to date
        app.add_systems(Update, cdda_overmap::index::index_added_chunks);
        app.add_systems(Update, cdda_overmap::index::index_removed_chunks);

        // Chain generation sets — overall order matches C++ overmap::generate()
        app.configure_sets(
            Update,
            (
                OvermapGenSet::InitBase,
                OvermapGenSet::NeighborConnections,
                OvermapGenSet::NaturalTerrain,
                OvermapGenSet::Highways,
                OvermapGenSet::Cities,
                OvermapGenSet::PostCities,
                OvermapGenSet::Connections,
                OvermapGenSet::Structures,
                OvermapGenSet::PreUnderground,
                OvermapGenSet::Underground,
                OvermapGenSet::Elevated,
                OvermapGenSet::Population,
                OvermapGenSet::Finalize,
            )
                .chain()
                .run_if(in_state(OvermapGenPhase::Generating)),
        );

        // ── InitBase ───────────────────────────────────────────────────
        app.add_systems(
            Update,
            steps::init_base_terrain.in_set(OvermapGenSet::InitBase),
        );

        // ── NeighborConnections — mirrors exit points from adjacent
        //    overmaps to ensure road/rail continuity across boundaries.
        app.add_systems(
            Update,
            steps::populate_connections_out_from_neighbors
                .in_set(OvermapGenSet::NeighborConnections),
        );

        // ── NaturalTerrain — chained C++ order: rivers → lakes → oceans
        //    → forests → swamps → ravines → polish_river(#1)
        app.add_systems(
            Update,
            (
                steps::place_rivers,
                steps::place_lakes,
                steps::place_oceans,
                steps::place_forests,
                steps::place_swamps,
                steps::place_ravines,
                steps::polish_river, // #1 — shore tiles for highway predecessors
            )
                .chain()
                .in_set(OvermapGenSet::NaturalTerrain),
        );

        // ── Highways — before cities so cities avoid highways ──────────
        app.add_systems(
            Update,
            steps::place_highways.in_set(OvermapGenSet::Highways),
        );

        // ── Cities — city center placement ─────────────────────────────
        app.add_systems(Update, steps::place_cities.in_set(OvermapGenSet::Cities));

        // ── PostCities — highway interchanges then city street grids ───
        app.add_systems(
            Update,
            (steps::place_highway_interchanges, steps::build_cities)
                .chain()
                .in_set(OvermapGenSet::PostCities),
        );

        // ── Connections — two chains gated by place_railroads_before_roads ─
        //    Default: forest_trails → roads → railroads
        app.add_systems(
            Update,
            (
                steps::place_forest_trails,
                steps::place_roads,
                steps::place_railroads,
            )
                .chain()
                .run_if(roads_before_railroads)
                .in_set(OvermapGenSet::Connections),
        );
        //    Alternative: forest_trails → railroads → roads
        app.add_systems(
            Update,
            (
                steps::place_forest_trails,
                steps::place_railroads,
                steps::place_roads,
            )
                .chain()
                .run_if(railroads_before_roads)
                .in_set(OvermapGenSet::Connections),
        );

        // ── Structures — city buildings → specials → mutable specials ──
        app.add_systems(
            Update,
            (
                steps::place_city_buildings,
                steps::place_specials,
                steps::place_mutable_specials,
            )
                .chain()
                .in_set(OvermapGenSet::Structures),
        );

        // ── PreUnderground — finalize highways, trailheads, polish(#2) ─
        app.add_systems(
            Update,
            (
                steps::finalize_highways,
                steps::place_forest_trailheads,
                steps::polish_river, // #2 — fix shores after specials
            )
                .chain()
                .in_set(OvermapGenSet::PreUnderground),
        );

        // ── Underground / Elevated ─────────────────────────────────────
        app.add_systems(
            Update,
            steps::generate_sub.in_set(OvermapGenSet::Underground),
        );
        app.add_systems(Update, steps::generate_over.in_set(OvermapGenSet::Elevated));

        // ── Population — mongroups + radios (no chunk writes, safe parallel) ─
        app.add_systems(
            Update,
            (steps::place_mongroups, steps::place_radios).in_set(OvermapGenSet::Population),
        );

        // ── Finalize ───────────────────────────────────────────────────
        app.add_systems(
            Update,
            steps::finalize_overmap.in_set(OvermapGenSet::Finalize),
        );

        // Transition to Complete after Finalize
        app.add_systems(
            Update,
            (|mut next: ResMut<NextState<OvermapGenPhase>>| {
                next.set(OvermapGenPhase::Complete);
            })
            .in_set(OvermapGenSet::Finalize),
        );
    }
}
