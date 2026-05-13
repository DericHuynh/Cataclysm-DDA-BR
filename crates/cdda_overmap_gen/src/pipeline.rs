//! Generation pipeline — system sets, plugin, and state.
//!
//! Each phase is a `SystemSet` in the `Update` schedule. The sets are
//! chained so they execute in order. Within each set, multiple systems
//! may run in parallel if they access disjoint chunks (Bevy handles this
//! automatically via component access tracking).
//!
//! Mods inject additional generation steps by adding systems to the
//! appropriate set without modifying this file.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

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
            noise_seed: 1920237457,
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
// System sets
// ---------------------------------------------------------------------------

/// Ordered phases of overmap generation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum OvermapGenSet {
    /// Fill all chunks with default terrain.
    InitBase,
    /// Noise-driven natural terrain.
    NaturalTerrain,
    /// River placement and shore building.
    Rivers,
    /// City center placement.
    Cities,
    /// City street building (needs CityTiles from place_cities flush).
    CityBuilding,
    /// Roads, railroads, forest trails via pathfinding.
    Connections,
    /// Buildings within cities, overmap specials.
    Structures,
    /// Underground layers (z < 0).
    Underground,
    /// Elevated layers (z > 0).
    Elevated,
    /// Monster groups, NPCs.
    Population,
    /// Finalize and fire completion.
    Finalize,
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

        // Chain generation sets
        app.configure_sets(
            Update,
            (
                OvermapGenSet::InitBase,
                OvermapGenSet::NaturalTerrain,
                OvermapGenSet::Rivers,
                OvermapGenSet::Cities,
                OvermapGenSet::CityBuilding,
                OvermapGenSet::Connections,
                OvermapGenSet::Structures,
                OvermapGenSet::Underground,
                OvermapGenSet::Elevated,
                OvermapGenSet::Population,
                OvermapGenSet::Finalize,
            )
                .chain()
                .run_if(in_state(OvermapGenPhase::Generating)),
        );

        // Register step systems
        app.add_systems(Update, steps::init_base_terrain.in_set(OvermapGenSet::InitBase));
        app.add_systems(
            Update,
            (
                steps::place_forests,
                steps::place_lakes,
                steps::place_oceans,
                steps::place_swamps,
                steps::place_ravines,
            )
                .in_set(OvermapGenSet::NaturalTerrain),
        );
        app.add_systems(
            Update,
            (
                steps::place_rivers,
                steps::build_river_shores,
            )
                .in_set(OvermapGenSet::Rivers),
        );
        app.add_systems(
            Update,
            steps::place_cities.in_set(OvermapGenSet::Cities),
        );
        app.add_systems(
            Update,
            steps::build_cities.in_set(OvermapGenSet::CityBuilding),
        );
        app.add_systems(
            Update,
            (
                steps::place_roads,
                steps::place_railroads,
                steps::place_forest_trails,
                steps::place_highways,
                steps::place_highway_interchanges,
            )
                .in_set(OvermapGenSet::Connections),
        );
        app.add_systems(
            Update,
            (
                steps::place_city_buildings,
                steps::place_specials,
                steps::place_mutable_specials,
            )
                .in_set(OvermapGenSet::Structures),
        );
        app.add_systems(Update, steps::generate_sub.in_set(OvermapGenSet::Underground));
        app.add_systems(Update, steps::generate_over.in_set(OvermapGenSet::Elevated));
        app.add_systems(
            Update,
            (
                steps::place_mongroups,
                steps::place_radios,
            )
                .in_set(OvermapGenSet::Population),
        );
        app.add_systems(Update, steps::finalize_overmap.in_set(OvermapGenSet::Finalize));
        app.add_systems(Update, steps::polish_river.in_set(OvermapGenSet::Finalize));
        app.add_systems(
            Update,
            (
                steps::finalize_highways,
                steps::place_forest_trailheads,
            )
                .in_set(OvermapGenSet::Finalize),
        );

        // Transition to Complete after Finalize — use explicit ordering
        app.add_systems(
            Update,
            (
                |mut next: ResMut<NextState<OvermapGenPhase>>| {
                    next.set(OvermapGenPhase::Complete);
                }
            )
                .in_set(OvermapGenSet::Finalize),
        );
    }
}
