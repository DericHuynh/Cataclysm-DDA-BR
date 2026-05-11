//! Startup systems — data loading and worldgen entry points.
//!
//! These were extracted from `cdda_core::data::def_world` during the
//! `data/ → cdda_data` crate extraction.  They reference `crate::worldgen`
//! and `DefinitionWorld` directly, so they remain in `cdda_core` as the
//! integration glue between subsystems.

use bevy_ecs::prelude::*;
use bevy_state::state::NextState;

use cdda_data::def_world::{build_def_world, DefinitionWorld};
use cdda_data::loader::Loader;

use crate::sim::state::{AppState, GameTime, LoadingStatus, StartupConfig};
use crate::core::coords::WorldPos;
use crate::core::raw_defs::city_building::CityBuildingDef;
use crate::core::id::DefId;

// ===========================================================================
// CityBuildings — resource wrapper for worldgen access
// ===========================================================================

/// Thin wrapper to store city_building definitions as a Bevy resource.
/// Extracted from `DefRegistry` after loading so worldgen can access them.
#[derive(Resource, Debug, Clone)]
pub struct CityBuildings(
    pub std::collections::HashMap<
        DefId<CityBuildingDef>,
        std::sync::Arc<CityBuildingDef>,
    >,
);

// ===========================================================================
// Startup system — load JSON data and build DefinitionWorld
// ===========================================================================

pub fn load_data_system(world: &mut World) {
    use tracing::info;

    info!("Data loading deferred until player starts game");

    let data_dirs = world.resource::<StartupConfig>().data_dirs.clone();

    world.resource_mut::<LoadingStatus>().current_phase = "Scanning JSON files...".into();
    info!("Loading data from {:?}", data_dirs);

    let mut loader = Loader::new(data_dirs);

    world.resource_mut::<LoadingStatus>().current_phase = "Ingesting raw definitions...".into();
    let raw_map = loader.ingest_all();
    let total_raw: usize = raw_map.values().map(|v| v.len()).sum();
    world.resource_mut::<LoadingStatus>().total_defs = total_raw;
    info!("Ingested {} raw definitions", total_raw);

    world.resource_mut::<LoadingStatus>().current_phase =
        "Resolving copy-from inheritance...".into();
    match loader.load() {
        Ok(registry) => {
            let count = registry.total_count();
            info!("Data loading complete: {} resolved definitions", count);

            world.resource_mut::<LoadingStatus>().current_phase =
                "Building definition entities...".into();
            let def_world = build_def_world(world, &registry, true);
            cdda_data::populate_flags::populate_def_flags(world, &registry, &def_world);
            cdda_data::schema_gen::collect_and_generate_schemas(world);
            info!(
                "DefinitionWorld: {} items, {} terrain, {} furniture, {} monsters",
                registry.items.len(),
                registry.terrain.len(),
                registry.furniture.len(),
                registry.monsters.len(),
            );

            // Store the city_buildings for dev-worldgen access
            world.insert_resource(CityBuildings(registry.city_buildings.clone()));

            world.insert_resource(def_world);
            world.insert_resource(GameTime::default());

            world.resource_mut::<LoadingStatus>().current_phase = "Complete".into();
            world.resource_mut::<LoadingStatus>().total_defs = count;
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::WorldGen);
        }
        Err(errors) => {
            for err in &errors {
                tracing::warn!("Data loading error: {:?}", err);
            }
            info!(
                "Data loading finished with {} non-fatal errors, continuing...",
                errors.len()
            );
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::WorldGen);
        }
    }
}

// ===========================================================================
// Worldgen system - dev-worldgen: one of every building
// ===========================================================================

pub fn worldgen_system(world: &mut World) {
    use tracing::info;

    let has_defs = world.get_resource::<DefinitionWorld>().is_some();

    // --- Dev-worldgen: populate WorldMap with one of every city building ---
    if has_defs {
        let city_buildings = world.remove_resource::<CityBuildings>();
        let config = world
            .get_resource::<crate::worldgen::dev::DevWorldgenConfig>()
            .cloned()
            .unwrap_or_default();

        if let Some(cb) = city_buildings {
            let building_count = cb.0.len();
            info!(
                "Dev-worldgen: generating showcase with {} city buildings...",
                building_count
            );

            let mut world_map = world.resource_mut::<crate::worldgen::setup::WorldMapResource>();
            let placed =
                crate::worldgen::dev::generate_dev_worldmap(&mut world_map.0, &cb.0, &config);
            info!(
                "Dev-worldgen complete: {} buildings placed, {} bubbles created",
                placed,
                world_map.0.bubble_count(),
            );
        }
    }

    if has_defs {
        use cdda_components::actor::*;
        use cdda_components::sim::*;

        let pos = WorldPos::new(0, 0, crate::ZLevel::new(0));
        world.spawn((
            PlayerData {
                name: "Survivor".into(),
                gender: Gender::Male,
                age: 25,
                height: 175,
                blood_type: "O+".into(),
                profession: None,
                scenario: None,
            },
            IsAlive,
            WorldPosition(pos),
            Creature {
                def_id: "player".into(),
                name: "Survivor".into(),
                species: crate::SpeciesId::from(0u32),
                symbol: '@',
            },
            Health {
                current: 100,
                max: 100,
            },
            Faction {
                id: crate::FactionId::from(0u32),
            },
            Solid,
            ActionPoints {
                current: 100,
                speed: 100,
            },
        ));
        info!("Spawned player at origin (0,0). Use the map to explore all buildings.");
    }
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
}
