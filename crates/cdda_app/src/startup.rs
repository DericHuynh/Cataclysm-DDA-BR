//! Startup systems — data loading and worldgen entry points.

use bevy_ecs::prelude::*;
use bevy_state::state::{NextState, State};

use cdda_data::def_world::{build_def_world, DefinitionWorld};
use cdda_data::loader::Loader;

use cdda_core_types::core::coords::{WorldPos, ZLevel, TILES_PER_OMT};
use cdda_core_types::core::id::DefId;
use cdda_core_types::core::raw_defs::city_building::CityBuildingDef;
use cdda_sim::state::{AppState, GameTime, LoadingStatus, StartupConfig};

use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap_gen::pipeline::{OvermapGenConfig, OvermapGenPhase, DEFAULT_NOISE_SEED};
use cdda_overmap_gen::steps::city_buildings::CityBuildingCatalog;

use std::sync::Arc;

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

pub fn load_data_system(world: &mut World) {
    use tracing::info;

    info!("Data loading deferred until player starts game");

    let data_dirs = world.resource::<StartupConfig>().data_dirs.clone();
    info!("Loading data from {:?}", data_dirs);

    world.resource_mut::<LoadingStatus>().current_phase = "Scanning JSON files...".into();

    let mut loader = Loader::new(data_dirs);

    world.resource_mut::<LoadingStatus>().current_phase = "Ingesting raw definitions...".into();
    let raw_map = loader.ingest_all();
    let total_raw: usize = raw_map.values().map(|v| v.len()).sum();
    world.resource_mut::<LoadingStatus>().total_defs = total_raw;
    info!("Ingested {} raw definitions", total_raw);

    // Save raw JSON values for registry viewer comparison
    {
        let mut raw_values = cdda_data::raw_values::RawDefinitionValues::new();
        for (type_name, defs) in &raw_map {
            let entries = defs
                .iter()
                .filter_map(|raw| raw.id.as_ref().map(|id| (id.clone(), raw.value.clone())))
                .collect::<std::collections::HashMap<_, _>>();
            if !entries.is_empty() {
                raw_values.values.insert(type_name.clone(), entries);
            }
        }
        world.insert_resource(raw_values);
    }

    world.resource_mut::<LoadingStatus>().current_phase =
        "Resolving copy-from inheritance...".into();
    match loader.load() {
        Ok(registry) => {
            let count = registry.total_count();
            info!("Data loading complete: {} resolved definitions", count);

            world.resource_mut::<LoadingStatus>().current_phase =
                "Building definition entities...".into();
            let def_world = build_def_world(world, &registry, true);
            world.insert_resource(cdda_data::def_registry_resource::DefRegistryResource(
                std::sync::Arc::new(registry.clone()),
            ));
            cdda_data::populate_flags::populate_def_flags(world, &registry, &def_world);
            cdda_data::schema_gen::collect_and_generate_schemas(world);
            info!(
                "DefinitionWorld: {} items, {} terrain, {} furniture, {} monsters",
                registry.items.len(),
                registry.terrain.len(),
                registry.furniture.len(),
                registry.monsters.len(),
            );

            // --- Build TerrainRegistry from overmap_terrains ---
            build_terrain_registry(world, &registry, &def_world);

            // --- Build CityBuildingCatalog ---
            let catalog = CityBuildingCatalog {
                buildings: registry.city_buildings.values().cloned().collect(),
            };
            world.insert_resource(catalog);

            world.insert_resource(CityBuildings(registry.city_buildings.clone()));

            // --- Build SpecialCatalog ---
            let special_catalog =
                cdda_overmap_gen::special_catalog::SpecialCatalog::from_registry(&registry);
            world.insert_resource(special_catalog);

            // --- Build ConnectionCatalog ---
            let connection_catalog =
                cdda_overmap_gen::connection_catalog::ConnectionCatalog::from_registry(&registry);
            world.insert_resource(connection_catalog);

            // --- Build MongroupCatalog ---
            let mongroup_catalog =
                cdda_overmap_gen::mongroup_catalog::MongroupCatalog::from_registry(&registry);
            world.insert_resource(mongroup_catalog);

            // --- Build OvermapRegionSettings from RegionSettingsDef ---
            let region_settings = build_region_settings(&registry);
            world.insert_resource(region_settings);

            world.insert_resource(def_world);
            world.insert_resource(GameTime::default());

            // --- Configure overmap generation ---
            let gen_config = OvermapGenConfig {
                noise_seed: DEFAULT_NOISE_SEED,
                om_x: 0,
                om_y: 0,
                region_id: "default".into(),
            };
            world.insert_resource(gen_config);

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
// Terrain registry builder
// ===========================================================================

fn build_terrain_registry(
    world: &mut World,
    registry: &cdda_data::DefRegistry,
    def_world: &DefinitionWorld,
) {
    use tracing::info;
    let mut treg = TerrainRegistry::empty();

    for (def_id, terrain) in &registry.overmap_terrains {
        // Determine flags from JSON flag strings.
        let mut flags = TerrainFlags::empty();
        let flag_strs: Vec<String> = match &terrain.flags {
            cdda_core_types::core::raw_defs::cdda_types::StringOrArray::Single(s) => {
                vec![s.clone()]
            }
            cdda_core_types::core::raw_defs::cdda_types::StringOrArray::Multi(v) => v.clone(),
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
            Some(cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s)) => {
                match s.to_lowercase().as_str() {
                    "fast" => 1,
                    "slow" => 5,
                    "impassable" => 255,
                    _ => 2,
                }
            }
            Some(cdda_core_types::core::raw_defs::cdda_types::RawValue::Number(n)) => {
                (*n as u8).max(1)
            }
            _ => 2,
        };

        // Determine mapgen ID.
        let mapgen_id = terrain
            .mapgen
            .as_ref()
            .and_then(|mg| mg.first())
            .and_then(|raw| match raw {
                cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s) => Some(s.clone()),
                cdda_core_types::core::raw_defs::cdda_types::RawValue::Object(obj) => obj
                    .get("builtin")
                    .or_else(|| obj.get("method"))
                    .and_then(|v| match v {
                        cdda_core_types::core::raw_defs::cdda_types::RawValue::String(s) => {
                            Some(s.clone())
                        }
                        _ => None,
                    }),
                _ => None,
            })
            .unwrap_or_else(|| def_id.as_str().to_string());

        let idx = treg.register_no_entity(def_id.as_str(), flags, travel_cost, mapgen_id);

        // Tag well-known terrain types.
        match def_id.as_str() {
            "field" => treg.field_index = idx,
            "forest" => treg.forest_index = idx,
            "forest_thick" => treg.forest_thick_index = idx,
            "forest_water" => treg.forest_water_index = idx,
            "road_ns" => treg.road_ns_index = idx,
            "road_ew" => treg.road_ew_index = idx,
            "road_nesw" => treg.road_nesw_index = idx,
            "lake_surface" => treg.lake_surface_index = idx,
            "lake_shore" => treg.lake_shore_index = idx,
            "ocean" => treg.ocean_index = idx,
            "river_center" => treg.river_center_index = idx,
            _ => {}
        }
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
            treg.register_no_entity(&variant_id, flags, travel_cost, mapgen_id.clone());
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
    if let Some(idx) = treg.index_by_id("road_ns") {
        treg.road_ns_index = idx;
    }
    if let Some(idx) = treg.index_by_id("road_ew") {
        treg.road_ew_index = idx;
    }
    if let Some(idx) = treg.index_by_id("road_nesw") {
        treg.road_nesw_index = idx;
    }

    // Ensure field_index is set
    if treg.field_index == 0 {
        if let Some(idx) = treg.index_by_id("field") {
            treg.field_index = idx;
        } else {
            // Find the first terrain with no special flags as default field
            for idx in 1..treg.len() as u32 {
                let flags = treg.flags_for(TerrainHandle::new(idx, 0));
                // A basic field terrain has no line-drawing or biome flags
                if !flags.contains(TerrainFlags::ROAD)
                    && !flags.contains(TerrainFlags::RIVER)
                    && !flags.contains(TerrainFlags::LAKE)
                    && !flags.contains(TerrainFlags::OCEAN)
                    && !flags.contains(TerrainFlags::FOREST)
                    && !flags.contains(TerrainFlags::IMPASSABLE)
                    && !flags.contains(TerrainFlags::UNDERGROUND)
                {
                    treg.field_index = idx;
                    break;
                }
            }
        }
    }

    info!(
        "TerrainRegistry built: {} terrain types registered",
        treg.len()
    );
    world.insert_resource(treg);
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
        settings.ocean_start = [None, None, None, None];

        // Sensible forest thresholds for temperate regions.
        // These produce ~25-35% forest coverage.
        settings.forest_noise_threshold = 0.25;
        settings.forest_noise_threshold_thick = 0.30;

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
    use tracing::info;

    let has_defs = world.get_resource::<DefinitionWorld>().is_some();
    if !has_defs {
        info!("Worldgen: no definitions loaded, skipping to InGame");
        world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        return;
    }

    // Check if TerrainRegistry exists.
    let has_registry = world.get_resource::<TerrainRegistry>().is_some();
    if !has_registry {
        info!("Worldgen: no terrain registry, skipping to InGame");
        world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        return;
    }

    // Kick off overmap generation by transitioning state (only once).
    // The OvermapGenPlugin's chained system sets will execute in order.
    let phase = world.resource::<State<OvermapGenPhase>>().get().clone();
    if phase == OvermapGenPhase::Idle {
        info!("Worldgen: starting overmap generation pipeline");
        world
            .resource_mut::<NextState<OvermapGenPhase>>()
            .set(OvermapGenPhase::Generating);
    }

    // Wait for generation to complete, then transition to InGame.
    // This is polled each frame until Complete.
    let phase = world.resource::<State<OvermapGenPhase>>().get().clone();
    if phase == OvermapGenPhase::Complete {
        info!("Worldgen: overmap generation complete, transitioning to InGame");
        world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
    }
}

// ===========================================================================
// Constants
// ===========================================================================

const FLOOR_CAP_ML: u32 = 4_000_000;

// ===========================================================================
// Examine overlay — item actions (drop, wield, resume craft)
// ===========================================================================

use cdda_components::actor::{ActionPoints, HandCount};
use cdda_components::def::ItemVolume;
use cdda_components::dev::{DevCamera, DevGroundItemName, DevPlayer};
use cdda_components::input::{GameAction, InputAction};
use cdda_components::item::{InsideContainer, Invlet, MountedPockets, WieldedBy, WieldedItems};
use cdda_components::sim::WorldPosition;
use cdda_context::ctx::Ctx;
use cdda_context::{ContextStack, FocusedCommandIndex};
use cdda_inventory::examine_resource::ExaminedItem;

use cdda_actor::turn::AP_COST_WIELD;

pub fn examine_item_input(world: &mut World) {
    let actions: Vec<GameAction> = {
        let mut messages = world.resource_mut::<bevy_ecs::message::Messages<InputAction>>();
        messages.update();
        messages.drain().map(|e| e.action.clone()).collect()
    };
    if actions.is_empty() {
        return;
    }

    let item_entity = match world.resource::<ExaminedItem>().0 {
        Some(e) => e,
        None => return,
    };

    let player_entity = {
        let mut q = world.query_filtered::<Entity, With<DevPlayer>>();
        match q.iter(world).next() {
            Some(e) => e,
            None => return,
        }
    };

    let hand_limit = world
        .get::<HandCount>(player_entity)
        .map(|h| h.0 as usize)
        .unwrap_or(0);

    let camera = world.resource::<DevCamera>().clone();

    for action in actions {
        match action {
            GameAction::Cancel => {
                *world.resource_mut::<ExaminedItem>() = ExaminedItem(None);
                let parent = world.resource_mut::<ContextStack>().0.pop();
                if let Some(p) = parent {
                    world.resource_mut::<FocusedCommandIndex>().on_pop(p);
                    world.resource_mut::<NextState<Ctx>>().set(p);
                }
            }
            GameAction::Drop => {
                let item_vol = world
                    .get::<ItemVolume>(item_entity)
                    .map(|v| v.0)
                    .unwrap_or(0);

                let floor_volume: u32 = {
                    let mut q = world.query::<(&WorldPosition, Option<&ItemVolume>)>();
                    q.iter(world)
                        .filter(|(wp, _)| {
                            wp.0.x.div_euclid(TILES_PER_OMT) == camera.x
                                && wp.0.y.div_euclid(TILES_PER_OMT) == camera.y
                                && wp.0.z.0 as i32 == camera.z
                        })
                        .filter_map(|(_, vol)| vol.map(|v| v.0))
                        .sum()
                };
                if floor_volume + item_vol > FLOOR_CAP_ML {
                    continue;
                }

                let drop_pos = WorldPos::new(
                    camera.x * TILES_PER_OMT,
                    camera.y * TILES_PER_OMT,
                    ZLevel::new(camera.z as i8),
                );

                world
                    .entity_mut(item_entity)
                    .remove::<InsideContainer>()
                    .remove::<WieldedBy>()
                    .remove::<Invlet>()
                    .insert(WorldPosition(drop_pos));

                *world.resource_mut::<ExaminedItem>() = ExaminedItem(None);
                let parent = world.resource_mut::<ContextStack>().0.pop();
                if let Some(p) = parent {
                    world.resource_mut::<FocusedCommandIndex>().on_pop(p);
                    world.resource_mut::<NextState<Ctx>>().set(p);
                }
            }
            GameAction::UseItem => {
                let is_wielded = world.get::<WieldedBy>(item_entity).is_some();
                if is_wielded {
                    let body_pocket = {
                        let mp = world.get::<MountedPockets>(player_entity);
                        mp.and_then(|mp| mp.iter().next()).unwrap_or(player_entity)
                    };
                    world
                        .entity_mut(item_entity)
                        .remove::<WieldedBy>()
                        .insert(InsideContainer(body_pocket));
                } else {
                    let wielded_count = world
                        .get::<WieldedItems>(player_entity)
                        .map(|wi| wi.iter().count())
                        .unwrap_or(0);
                    if wielded_count < hand_limit {
                        world
                            .entity_mut(item_entity)
                            .remove::<InsideContainer>()
                            .insert(WieldedBy(player_entity));
                    } else {
                        continue;
                    }
                }
                if let Some(mut ap) = world.get_mut::<ActionPoints>(player_entity) {
                    ap.spend(AP_COST_WIELD);
                }
                *world.resource_mut::<ExaminedItem>() = ExaminedItem(None);
                let parent = world.resource_mut::<ContextStack>().0.pop();
                if let Some(p) = parent {
                    world.resource_mut::<FocusedCommandIndex>().on_pop(p);
                    world.resource_mut::<NextState<Ctx>>().set(p);
                }
            }
            GameAction::HotkeyPress('r') => {
                match cdda_crafting::systems::resume_craft(world, player_entity, item_entity) {
                    Ok(()) => {}
                    Err(_e) => {}
                }
                *world.resource_mut::<ExaminedItem>() = ExaminedItem(None);
                let parent = world.resource_mut::<ContextStack>().0.pop();
                if let Some(p) = parent {
                    world.resource_mut::<FocusedCommandIndex>().on_pop(p);
                    world.resource_mut::<NextState<Ctx>>().set(p);
                }
            }
            _ => {}
        }
    }
}

// ===========================================================================
// Dev-world — spawn player at origin
// ===========================================================================

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

    cdda_inventory::pocket::spawn_body_pocket(world, player);

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
