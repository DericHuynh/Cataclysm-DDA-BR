//! Generation pipeline — system sets, plugin, config, and state.
//!
//! Verbatim port of CDDA master's `overmap::generate()` ordering
//! (overmap.cpp L932-1060). Systems within each set are chained
//! to guarantee deterministic sequential execution matching C++.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

use crate::region_settings::OvermapRegionSettings;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Tracks the phase of overmap generation.
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

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Default noise seed for deterministic world generation.
pub const DEFAULT_NOISE_SEED: u32 = 1920237457;

/// Configuration for a single overmap generation run.
#[derive(Resource, Debug, Clone)]
pub struct OvermapGenConfig {
    /// Noise seed for terrain generation.
    pub noise_seed: u32,
    /// Which overmap position to generate (world-overmap coordinates).
    pub om_x: i32,
    pub om_y: i32,
}

impl Default for OvermapGenConfig {
    fn default() -> Self {
        Self {
            noise_seed: DEFAULT_NOISE_SEED,
            om_x: 0,
            om_y: 0,
        }
    }
}

/// Entity marker for the overmap entity being generated.
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
/// C++-compatible sequential execution.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum OvermapGenSet {
    /// Fill all chunks with default terrain.
    InitBase,
    /// Populate cross-overmap connection exits (mirrored from neighbor edges).
    NeighborConnections,
    /// Natural terrain: rivers → lakes → oceans → forests → swamps
    /// → ravines → polish_river(#1).
    NaturalTerrain,
    /// Highway path placement (before cities so cities avoid highways).
    Highways,
    /// City center placement.
    Cities,
    /// Post-city: highway interchanges (after city centers placed)
    /// then city street grids (after interchanges).
    PostCities,
    /// Roads, railroads, forest trails via pathfinding.
    /// Two sub-chains gated by `place_railroads_before_roads` flag.
    Connections,
    /// Overmap specials (fixed and mutable).
    Structures,
    /// Pre-underground finalization: highways → trailheads → polish_river(#2).
    PreUnderground,
    /// Underground layers (z < 0): sewers, subways.
    Underground,
    /// Elevated layers (z > 0): bridges.
    Elevated,
    /// Monster groups, radio towers.
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

        // Chunk index maintenance — observers, not systems
        app.add_observer(cdda_overmap::index::ChunkIndex::on_chunk_added);
        app.add_observer(cdda_overmap::index::ChunkIndex::on_chunk_removed);

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

        // ── InitBase ──────────────────────────────────────────────────
        app.add_systems(
            Update,
            crate::steps::init_base_terrain.in_set(OvermapGenSet::InitBase),
        );

        // ── NeighborConnections ───────────────────────────────────────
        app.add_systems(
            Update,
            crate::steps::populate_connections_out_from_neighbors
                .in_set(OvermapGenSet::NeighborConnections),
        );

        // ── NaturalTerrain — C++ order ────────────────────────────────
        app.add_systems(
            Update,
            (
                crate::steps::place_rivers,
                crate::steps::place_lakes,
                crate::steps::place_oceans,
                crate::steps::place_forests,
                crate::steps::place_swamps,
                crate::steps::place_ravines,
                crate::steps::polish_river, // #1
            )
                .chain()
                .in_set(OvermapGenSet::NaturalTerrain),
        );

        // ── Highways — before cities so cities avoid highways ─────────
        app.add_systems(
            Update,
            crate::steps::place_highways.in_set(OvermapGenSet::Highways),
        );

        // ── Cities — city center placement ────────────────────────────
        app.add_systems(
            Update,
            crate::steps::place_cities.in_set(OvermapGenSet::Cities),
        );

        // ── PostCities — interchange then city streets ────────────────
        app.add_systems(
            Update,
            (
                crate::steps::place_highway_interchanges,
                crate::steps::build_cities,
            )
                .chain()
                .in_set(OvermapGenSet::PostCities),
        );

        // ── Connections — two chains gated by flag ────────────────────
        app.add_systems(
            Update,
            (
                crate::steps::place_forest_trails,
                crate::steps::place_roads,
                crate::steps::place_railroads,
            )
                .chain()
                .run_if(roads_before_railroads)
                .in_set(OvermapGenSet::Connections),
        );
        app.add_systems(
            Update,
            (
                crate::steps::place_forest_trails,
                crate::steps::place_railroads,
                crate::steps::place_roads,
            )
                .chain()
                .run_if(railroads_before_roads)
                .in_set(OvermapGenSet::Connections),
        );

        // ── Structures ────────────────────────────────────────────────
        app.add_systems(
            Update,
            (
                crate::steps::place_specials,
                crate::steps::place_mutable_specials,
            )
                .chain()
                .in_set(OvermapGenSet::Structures),
        );

        // ── PreUnderground ────────────────────────────────────────────
        app.add_systems(
            Update,
            (
                crate::steps::finalize_highways,
                crate::steps::place_forest_trailheads,
                crate::steps::polish_river, // #2
            )
                .chain()
                .in_set(OvermapGenSet::PreUnderground),
        );

        // ── Underground / Elevated ────────────────────────────────────
        app.add_systems(
            Update,
            crate::steps::generate_sub.in_set(OvermapGenSet::Underground),
        );
        app.add_systems(
            Update,
            crate::steps::generate_over.in_set(OvermapGenSet::Elevated),
        );

        // ── Population ────────────────────────────────────────────────
        app.add_systems(
            Update,
            (crate::steps::place_mongroups, crate::steps::place_radios)
                .in_set(OvermapGenSet::Population),
        );

        // ── Finalize ──────────────────────────────────────────────────
        app.add_systems(
            Update,
            crate::steps::finalize_overmap.in_set(OvermapGenSet::Finalize),
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
