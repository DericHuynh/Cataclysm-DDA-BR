//! Startup systems — data loading and worldgen entry points.

use bevy_ecs::prelude::*;
use bevy_state::state::{NextState, State};

use cdda_data::def_world::{build_def_world, DefinitionWorld};

use cdda_core_types::core::coords::{WorldPos, ZLevel, TILES_PER_OMT};
use cdda_core_types::core::id::DefId;
use cdda_defs_raw::raw_defs::city_building::CityBuildingDef;
use cdda_sim::runtime::state::{AppState, GameTime};

use cdda_overmap::registry::{CoreTerrains, TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap_gen::pipeline::{OvermapGenConfig, OvermapGenPhase, DEFAULT_NOISE_SEED};

// ===========================================================================
// CityBuildings — resource wrapper for worldgen access
// ===========================================================================

/// Thin wrapper to store city_building definitions as a Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct CityBuildings(
    pub std::collections::HashMap<DefId<CityBuildingDef>, std::sync::Arc<CityBuildingDef>>,
);

// ===========================================================================
// Startup system — load JSON data and build DefinitionWorld
// ===========================================================================

/// Compatibility entry point; production uses begin_loading/poll_loading.
pub fn load_data_system(world: &mut World) {
    crate::loading::poll_loading(world);
}

// ===========================================================================
// Shared "build everything from a resolved registry into the World" path
// ===========================================================================

/// Builds every runtime resource and entity that depends on a fully-resolved
/// [`cdda_data::DefRegistry`]: the definition-world, flag registries, schemas,
/// terrain/city/region resources, overmap-gen config and game clock.
///
/// This is the shared tail used by both the legacy disk-backed load
/// ([`load_data_system`]) and the asset-driven hot-reload path (see
/// `crate::data_assets`), so a hot-reload rebuilds exactly what an initial load
/// built.
///
/// During hot reload the app is already `InGame`; in that case the state is
/// left untouched (only transition on first load, from `DataLoading`). Returns
/// false if terrain IDs were removed: the reload is rejected before rebuilding
/// definition entities, and existing handles/resources remain valid.
pub(crate) fn apply_registry_to_world(
    world: &mut World,
    registry: &cdda_data::DefRegistry,
    count: usize,
) -> bool {
    // Validate the terrain reload before mutating any definition-dependent world
    // state. Existing chunks hold process-local slots which must not be reused.
    let terrain_registry = match build_terrain_registry(
        registry,
        world.get_resource::<TerrainRegistry>(),
    ) {
        Ok(terrain_registry) => terrain_registry,
        Err(error) => {
            tracing::error!(%error, "Definition reload rejected; existing terrain IDs must remain available");
            crate::loading::publish_report(
                world,
                cdda_components::progress::ReportEvent::progress(
                    "Reload rejected",
                    error.to_string(),
                )
                .level(cdda_components::progress::ReportLevel::Error),
            );
            return false;
        }
    };

    crate::loading::publish_report(
        world,
        cdda_components::progress::ReportEvent::progress(
            "Building definition entities",
            "Publishing reloaded content",
        ),
    );
    let def_world = build_def_world(world, registry, true);
    finish_registry_publication(world, registry, count, terrain_registry, def_world);
    true
}

pub(crate) fn finish_registry_publication(
    world: &mut World,
    registry: &cdda_data::DefRegistry,
    count: usize,
    terrain_registry: TerrainRegistry,
    def_world: DefinitionWorld,
) {
    use tracing::info;
    world.insert_resource(cdda_data::def_registry_resource::DefRegistryResource(
        std::sync::Arc::new(registry.clone()),
    ));
    cdda_data::populate_flags::populate_def_flags(world, registry, &def_world);
    cdda_data::schema_gen::collect_and_generate_schemas(world);
    info!(
        "DefinitionWorld: {} items, {} terrain, {} furniture, {} monsters",
        registry.items.len(),
        registry.terrain.len(),
        registry.furniture.len(),
        registry.monsters.len(),
    );

    // --- Publish the validated, handle-preserving terrain registry ---
    world.insert_resource(CoreTerrains::from_registry(&terrain_registry));
    world.insert_resource(terrain_registry);

    world.insert_resource(CityBuildings(registry.city_buildings.clone()));

    // --- Build OvermapRegionSettings from RegionSettingsDef ---
    let region_settings = build_region_settings(registry);
    world.insert_resource(region_settings);

    world.insert_resource(def_world);
    // Seed the game clock only on first load; a hot reload must not reset it.
    if world.get_resource::<GameTime>().is_none() {
        world.insert_resource(GameTime::default());
    }

    // --- Configure overmap generation ---
    let gen_config = OvermapGenConfig {
        noise_seed: DEFAULT_NOISE_SEED,
        om_x: 0,
        om_y: 0,
    };
    world.insert_resource(gen_config);

    crate::loading::publish_report(
        world,
        cdda_components::progress::ReportEvent::progress(
            "Registries ready",
            format!("{count} definitions published"),
        ),
    );

    // Transition to worldgen only on the initial load; a hot reload must not
    // yank the player out of an in-progress game.
    let in_game = world
        .get_resource::<State<AppState>>()
        .map_or(false, |s| *s.get() == AppState::InGame);
    if !in_game {
        world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::WorldGen);
    }
}

// ===========================================================================
// Terrain registry builder
// ===========================================================================

pub(crate) fn build_terrain_registry(
    registry: &cdda_data::DefRegistry,
    existing: Option<&TerrainRegistry>,
) -> Result<TerrainRegistry, cdda_overmap::registry::TerrainRegistryReloadError> {
    use tracing::info;
    let mut treg = TerrainRegistry::empty();

    for (def_id, terrain) in &registry.overmap_terrains {
        // Determine flags from JSON flag strings.
        let mut flags = TerrainFlags::empty();
        let flag_strs: Vec<String> = match &terrain.flags {
            cdda_defs_raw::raw_defs::cdda_types::StringOrArray::Single(s) => {
                vec![s.clone()]
            }
            cdda_defs_raw::raw_defs::cdda_types::StringOrArray::Multi(v) => v.clone(),
        };
        for f in &flag_strs {
            let upper = f.to_uppercase();
            match upper.as_str() {
                "RIVER" => flags.set(TerrainFlags::RIVER),
                "LAKE" => flags.set(TerrainFlags::LAKE),
                "LAKE_SHORE" => flags.set(TerrainFlags::LAKE),
                "OCEAN" => flags.set(TerrainFlags::OCEAN),
                "OCEAN_SHORE" => flags.set(TerrainFlags::OCEAN),
                "ROAD" => flags.set(TerrainFlags::ROAD),
                "HIGHWAY" => flags.set(TerrainFlags::HIGHWAY),
                "LINE_DRAWING" | "LINEAR" => flags.set(TerrainFlags::LINE_DRAWING),
                "IMPASSABLE" => flags.set(TerrainFlags::IMPASSABLE),
                "UNDERGROUND" => flags.set(TerrainFlags::UNDERGROUND),
                "BRIDGE" => flags.set(TerrainFlags::BRIDGE),
                "SEWER" => flags.set(TerrainFlags::SEWER),
                "SUBWAY" => flags.set(TerrainFlags::SUBWAY),
                "RAILROAD" => flags.set(TerrainFlags::RAILROAD),
                "MANHOLE" => flags.set(TerrainFlags::MANHOLE),
                "FOREST" => flags.set(TerrainFlags::FOREST),
                _ => {}
            }
        }

        // Infer flags from terrain ID patterns (CDDA convention).
        let id_lower = def_id.as_str().to_lowercase();
        if id_lower.starts_with("forest") || id_lower.contains("_forest_") {
            flags.set(TerrainFlags::FOREST);
        }
        if id_lower.starts_with("road_") || id_lower == "road" {
            flags.set(TerrainFlags::ROAD);
            flags.set(TerrainFlags::LINE_DRAWING);
        }
        if id_lower.starts_with("highway_") || id_lower.starts_with("hiway_") {
            flags.set(TerrainFlags::HIGHWAY);
            flags.set(TerrainFlags::LINE_DRAWING);
        }
        if id_lower.starts_with("railroad_") || id_lower.starts_with("rail_") {
            flags.set(TerrainFlags::RAILROAD);
            flags.set(TerrainFlags::LINE_DRAWING);
        }
        if id_lower.starts_with("river_") || id_lower == "river_center" {
            flags.set(TerrainFlags::RIVER);
            flags.set(TerrainFlags::LINE_DRAWING);
        }
        if id_lower.starts_with("lake_") || id_lower.contains("_lake_") {
            flags.set(TerrainFlags::LAKE);
        }
        if id_lower.starts_with("ocean_") || id_lower.contains("_ocean_") {
            flags.set(TerrainFlags::OCEAN);
        }
        if id_lower.starts_with("sewer_") || id_lower.contains("_sewer_") {
            flags.set(TerrainFlags::SEWER);
        }
        if id_lower.starts_with("subway_") || id_lower.contains("_subway_") {
            flags.set(TerrainFlags::SUBWAY);
        }
        if id_lower.contains("_bridge") || id_lower.ends_with("_bridge") {
            flags.set(TerrainFlags::BRIDGE);
        }
        if id_lower.contains("manhole") {
            flags.set(TerrainFlags::MANHOLE);
        }
        if id_lower.starts_with("field") || id_lower == "open_air" {
            // field — default terrain
        }
        // Also tag well-known subtypes from the ID more broadly
        if id_lower.contains("house_")
            || id_lower.contains("_house")
            || id_lower.starts_with("house")
        {
            // building — no special flag needed
        }

        // Determine travel cost.
        let travel_cost: u8 = match &terrain.travel_cost_type {
            Some(cdda_defs_raw::raw_defs::cdda_types::RawValue::String(s)) => {
                match s.to_lowercase().as_str() {
                    "fast" => 1,
                    "slow" => 5,
                    "impassable" => 255,
                    _ => 2,
                }
            }
            Some(cdda_defs_raw::raw_defs::cdda_types::RawValue::Number(n)) => (*n as u8).max(1),
            _ => 2,
        };

        // Determine mapgen ID.
        let mapgen_id = terrain
            .mapgen
            .as_ref()
            .and_then(|mg| mg.first())
            .and_then(|raw| match raw {
                cdda_defs_raw::raw_defs::cdda_types::RawValue::String(s) => Some(s.clone()),
                cdda_defs_raw::raw_defs::cdda_types::RawValue::Object(obj) => obj
                    .get("builtin")
                    .or_else(|| obj.get("method"))
                    .and_then(|v| match v {
                        cdda_defs_raw::raw_defs::cdda_types::RawValue::String(s) => Some(s.clone()),
                        _ => None,
                    }),
                _ => None,
            })
            .unwrap_or_else(|| def_id.as_str().to_string());

        treg.register_no_entity(def_id.as_str(), flags, travel_cost, mapgen_id, 0);
    }

    // -- Generate directional variants for line-drawing terrains --
    let count_before = treg.len();
    let mut variants_to_create: Vec<(u32, String)> = Vec::new();

    for idx in 1..count_before as u32 {
        let handle = TerrainHandle::new(idx, 0);
        let flags = treg.flags_for(handle);
        if !flags.contains(TerrainFlags::LINE_DRAWING) {
            continue;
        }
        let base_id = treg.string_id_for(handle).unwrap_or("").to_string();
        let travel_cost = treg.travel_cost(handle);
        let mapgen_id = treg.mapgen_id(handle).to_string();

        // Create directional variants: _ns (vertical), _ew (horizontal), _nesw (intersection)
        for suffix in &["_ns", "_ew", "_nesw"] {
            let variant_id = format!("{}{}", base_id, suffix);
            if treg.index_by_id(&variant_id).is_some() {
                continue; // already exists
            }
            treg.register_no_entity(&variant_id, flags, travel_cost, mapgen_id.clone(), 0);
            variants_to_create.push((idx, variant_id));
        }
    }

    // Set up rotation: direction → variant
    for (base_idx, variant_id) in &variants_to_create {
        let variant_idx = treg.index_by_id(variant_id).unwrap();
        let suffix = variant_id.rsplit('_').next().unwrap_or("");
        match suffix {
            "ns" => {
                treg.register_rotation(*base_idx, 0, variant_idx); // north
                treg.register_rotation(*base_idx, 2, variant_idx); // south
            }
            "ew" => {
                treg.register_rotation(*base_idx, 1, variant_idx); // east
                treg.register_rotation(*base_idx, 3, variant_idx); // west
            }
            "nesw" => {
                // The _nesw variant is its own entry; look it up directly via handle_by_id.
                // Its default self-rotation is already correct (all dirs → self).
            }
            _ => {}
        }
    }

    // Tag well-known directional variants if they were generated.

    info!(
        "TerrainRegistry built: {} terrain types registered",
        treg.len()
    );
    if let Some(existing) = existing {
        let mut rebuilt = existing.clone();
        rebuilt.rebuild_from(&treg)?;
        Ok(rebuilt)
    } else {
        Ok(treg)
    }
}

// ===========================================================================
// Region settings builder
// ===========================================================================

/// Build an `OvermapRegionSettings` resource from the first `RegionSettingsDef`
/// found in the registry, falling back to defaults if none is present.
fn build_region_settings(
    registry: &cdda_data::DefRegistry,
) -> cdda_overmap_gen::region_settings::OvermapRegionSettings {
    use cdda_overmap_gen::region_settings::OvermapRegionSettings;
    use tracing::info;

    // Try to find a region_settings def and parse the fields we care about.
    // CDDA region_settings defs reference sub-settings by string ID;
    // those sub-settings (river_settings, forest_settings, etc.) may be
    // loaded as separate JSON types or inlined.  For now we use sensible
    // defaults that work on overmap (0,0) without oceans or extreme forests.
    if let Some(rs) = registry.region_settings.values().next() {
        let mut settings = OvermapRegionSettings::default();

        // If the region def has a city_size-like field, use it
        // (the CDDA RegionSettingsDef has references like "cities": "default_city"
        //  which resolve to city_settings objects; we don't resolve those yet)
        info!("build_region_settings: using region '{}'", rs.id.as_str());

        // Ocean off by default unless region explicitly enables it.
        // The ocean_start values come from a separate ocean_settings JSON
        // type which we don't resolve yet, so keep them disabled.

        // Sensible forest thresholds for temperate regions.
        // These produce ~25-35% forest coverage.
        settings.forest.noise_threshold_forest = 0.25;
        settings.forest.noise_threshold_forest_thick = 0.30;

        // Enable standard features.
        settings.place_roads = true;
        settings.place_specials = true;

        settings
    } else {
        OvermapRegionSettings::default()
    }
}

// ===========================================================================
// Worldgen system — triggers overmap generation
// ===========================================================================

pub fn worldgen_system(world: &mut World) {
    if world
        .resource::<cdda_components::progress::OperationReport>()
        .failed()
        || world
            .resource::<cdda_components::progress::OperationReport>()
            .cancelled
    {
        return;
    }
    let has_defs = world.get_resource::<DefinitionWorld>().is_some();
    if !has_defs {
        crate::loading::publish_report(
            world,
            cdda_components::progress::ReportEvent::progress(
                "World generation failed",
                "No definitions were published",
            )
            .level(cdda_components::progress::ReportLevel::Error),
        );
        return;
    }

    // Check if TerrainRegistry exists.
    let has_registry = world.get_resource::<TerrainRegistry>().is_some();
    if !has_registry {
        crate::loading::publish_report(
            world,
            cdda_components::progress::ReportEvent::progress(
                "World generation failed",
                "Terrain registry is missing",
            )
            .level(cdda_components::progress::ReportLevel::Error),
        );
        return;
    }

    // Kick off overmap generation by transitioning state (only once).
    // The OvermapGenPlugin's chained system sets will execute in order.
    let phase = world.resource::<State<OvermapGenPhase>>().get().clone();
    if phase == OvermapGenPhase::Idle {
        crate::loading::publish_report(
            world,
            cdda_components::progress::ReportEvent::progress(
                "Generating world",
                "Terrain, rivers, forests, roads and settlements",
            ),
        );
        world
            .resource_mut::<NextState<OvermapGenPhase>>()
            .set(OvermapGenPhase::Generating);
    }

    // Wait for generation to complete, then transition to InGame.
    // This is polled each frame until Complete.
    let phase = world.resource::<State<OvermapGenPhase>>().get().clone();
    if phase == OvermapGenPhase::Complete {
        crate::loading::publish_report(
            world,
            cdda_components::progress::ReportEvent::progress("Ready", "World generation complete")
                .level(cdda_components::progress::ReportLevel::Complete),
        );
        world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
    }
}

use cdda_components::actor::HandCount;
use cdda_components::dev::{DevCamera, DevPlayer};
use cdda_components::sim::WorldPosition;

pub fn spawn_dev_world(world: &mut World) {
    use tracing::info;

    let pos = WorldPos::new(0, 0, ZLevel::new(0));

    let player = world
        .spawn((
            DevPlayer,
            HandCount(2),
            cdda_components::actor::ActionPoints {
                current: 100,
                speed: 100,
            },
            cdda_components::actor::IsAlive,
            cdda_components::actor::Health {
                current: 100,
                max: 100,
            },
            cdda_components::actor::PlayerData {
                name: "Dev Player".to_string(),
                gender: cdda_components::actor::Gender::Male,
                age: 30,
                height: 170,
                blood_type: "O+".to_string(),
                profession: None,
                scenario: None,
            },
            WorldPosition(pos),
            cdda_components::actor::Creature {
                def_id: "player".to_string(),
                name: "Dev".to_string(),
                species: cdda_components::SpeciesId::new("human"),
                symbol: '@',
            },
            cdda_components::sim::Solid,
        ))
        .id();

    cdda_sim::inventory::pocket::spawn_body_pocket(world, player);

    let camera = DevCamera {
        x: pos.x.div_euclid(TILES_PER_OMT),
        y: pos.y.div_euclid(TILES_PER_OMT),
        z: pos.z.0 as i32,
    };
    let cx = camera.x;
    let cy = camera.y;
    let cz = camera.z;
    world.insert_resource(camera);
    world.insert_resource(cdda_overmap_gen::pipeline::OvermapGenConfig::default());

    // Dev-spawn resources from old worldgen — migrated here.
    // These should eventually move to a dev_spawn crate.
    info!(
        "Dev world spawned: player at ({}, {}), camera at ({}, {}, {})",
        pos.x, pos.y, cx, cy, cz
    );
}
