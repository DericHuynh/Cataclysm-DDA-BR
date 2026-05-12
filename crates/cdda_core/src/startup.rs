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
use crate::core::coords::{WorldPos, ZLevel};
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
            .get_resource::<cdda_worldgen::dev::DevWorldgenConfig>()
            .cloned()
            .unwrap_or_default();

        if let Some(cb) = city_buildings {
            let building_count = cb.0.len();
            info!(
                "Dev-worldgen: generating showcase with {} city buildings...",
                building_count
            );

            let mut world_map = world.resource_mut::<cdda_worldgen::setup::WorldMapResource>();
            let placed =
                cdda_worldgen::dev::generate_dev_worldmap(&mut world_map.0, &cb.0, &config);
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

// ===========================================================================
// Constants
// ===========================================================================

/// Maximum total item volume on a single tile (in mL) before blocking drops.
const FLOOR_CAP_ML: u32 = 4_000_000;

// ===========================================================================
// Examine overlay — item actions (drop, wield, resume craft)
// ===========================================================================

use cdda_components::actor::{ActionPoints, HandCount};
use cdda_components::dev::{DevCamera, DevPlayer, DevGroundItemName};
use cdda_components::input::{GameAction, InputAction};
use cdda_components::def::ItemVolume;
use cdda_components::item::{
    InsideContainer, Inventory, Invlet, ItemTypeId, MountedPockets,
    StackCount, WieldedBy, WieldedItems,
};
use cdda_components::sim::WorldPosition;
use cdda_context::{ContextStack, FocusedCommandIndex};
use cdda_context::ctx::Ctx;
use cdda_inventory::examine_resource::ExaminedItem;

use crate::actor::turn::AP_COST_WIELD;

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
                            wp.0.x.div_euclid(24) == camera.x
                                && wp.0.y.div_euclid(24) == camera.y
                                && wp.0.z.0 as i32 == camera.z
                        })
                        .filter_map(|(_, vol)| vol.map(|v| v.0))
                        .sum()
                };
                if floor_volume + item_vol > FLOOR_CAP_ML {
                    continue;
                }

                let drop_pos = WorldPos::new(
                    camera.x * 24, camera.y * 24,
                    ZLevel::new(camera.z as i8),
                );

                let invlet_char = world.get::<Invlet>(item_entity).map(|i| i.0);
                if let Some(c) = invlet_char {
                    if let Some(mut inv) = world.get_mut::<Inventory>(player_entity) {
                        inv.invlets.remove(&c);
                    }
                }

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
                use tracing::{info, warn};
                match crate::crafting::systems::resume_craft(
                    world, player_entity, item_entity,
                ) {
                    Ok(()) => info!("Resumed craft on {:?}", item_entity),
                    Err(e) => warn!("Cannot resume craft: {}", e),
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
// Dev-world — spawn player and test items
// ===========================================================================

pub fn spawn_dev_world(world: &mut World) {
    let pos = WorldPos::new(0, 0, ZLevel::new(0));

    let player = world.spawn((
        DevPlayer,
        HandCount(2),
        cdda_components::item::Inventory::default(),
        cdda_components::actor::ActionPoints { current: 100, speed: 100 },
        cdda_components::actor::IsAlive,
        cdda_components::actor::Health { current: 100, max: 100 },
        cdda_components::actor::PlayerData {
            name: "Dev Player".to_string(),
            gender: cdda_components::actor::Gender::Male,
            age: 30, height: 170,
            blood_type: "O+".to_string(),
            profession: None, scenario: None,
        },
        WorldPosition(pos),
        cdda_components::actor::Creature {
            def_id: "player".to_string(),
            name: "Dev".to_string(),
            species: crate::core::id::SpeciesId::from(0u32),
            symbol: '@',
        },
        cdda_components::sim::Solid,
    )).id();

    crate::inventory::pocket::spawn_body_pocket(world, player);
    world.insert_resource(DevCamera::default());
    world.insert_resource(cdda_worldgen::dev_spawn::DevSpawnFocus::default());
    world.insert_resource(cdda_worldgen::dev_spawn::DevSpawnQueue::default());
}
