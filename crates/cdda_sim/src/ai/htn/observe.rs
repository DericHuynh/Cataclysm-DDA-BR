//! Actor-scoped observations: the read-only bridge from Bevy relationships
//! to the planning models the HTN kernels read and write.
//!
//! The planner never sees `Related<R>` collections or raw entity handles —
//! it sees small, bounded **models** (needs, inventory, nearby ground items,
//! navigation). The observation adapter understands the relationship graph
//! (nested containers, mounted pockets, worn and wielded equipment — via the
//! inventory subsystem's recursive traversal) and derives selection facts
//! (definition ids, categories, counts) once per observation so overlapping
//! selectors answer from ONE consistent snapshot.
//!
//! Observations may be knowledge-limited (what the actor knows), not
//! omniscient — `view_radius` is the first slice of that contract.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use cdda_components::def::{IsDef, ItemCategory};
use cdda_components::item::{DefOrigin, InsideContainer, StackCount, WieldedBy};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::WorldPos;
use cdda_data::def_world::DefinitionWorld;
use cdda_data::interner::ItemTypeRegistry;

use super::model::{InventoryModel, NavigationModel, NearbyModel, NeedsModel, ObservedItem};

/// Definition-level facts the observation needs: `DefOrigin` token → item
/// definition id, and definition id → category. Owned data built once per
/// validated generation (reload) — observations never re-derive it.
#[derive(Debug, Clone, Default)]
pub struct ItemCatalog {
    def_of: HashMap<u32, String>,
    category_of: HashMap<String, String>,
}

impl ItemCatalog {
    /// Build a catalog from the world's internment registry and def entities.
    pub fn from_world(world: &mut World) -> Self {
        let mut def_of = HashMap::new();
        if let Some(reg) = world.get_resource::<ItemTypeRegistry>() {
            for (s, token) in reg.iter() {
                def_of.insert(token.0, s.to_string());
            }
        }
        let mut category_of = HashMap::new();
        let mut q = world.query::<(&DefOrigin, &ItemCategory)>();
        for (origin, cat) in q.iter(world) {
            if let Some(def) = def_of.get(&origin.0) {
                category_of.insert(def.clone(), cat.0.clone());
            }
        }
        // Fallback: category by definition id string (def entities indexed by
        // the definition world).
        if let Some(def_world) = world.get_resource::<DefinitionWorld>() {
            let pairs: Vec<(String, String)> = def_world
                .iter()
                .filter_map(|(category, id, entity)| {
                    if category != cdda_data::def_world::DefCategory::Item {
                        return None; // ItemCategory only exists on item defs
                    }
                    world
                        .get::<ItemCategory>(entity)
                        .map(|c| (id.to_string(), c.0.clone()))
                })
                .collect();
            for (id, cat) in pairs {
                category_of.entry(id).or_insert(cat);
            }
        }
        ItemCatalog {
            def_of,
            category_of,
        }
    }

    /// The definition id string behind a `DefOrigin` token.
    pub fn def_of(&self, origin: &DefOrigin) -> Option<String> {
        self.def_of.get(&origin.0).cloned()
    }

    /// The category of an item definition, if any.
    pub fn category_of(&self, def: &str) -> Option<String> {
        self.category_of.get(def).cloned()
    }
}

/// Everything an agent's planner may know this tick, divided by concern.
/// Each model is bounded (a radius, a carried inventory) — never a world
/// snapshot — so cloning/rollback during search stays cheap and per-component
/// look-ahead stays precise.
#[derive(Debug, Clone, Default)]
pub struct ActorObservation {
    pub needs: NeedsModel,
    pub inventory: InventoryModel,
    pub nearby: NearbyModel,
    pub navigation: NavigationModel,
}

/// Observe `actor` from its own point of view: carried inventory (recursive:
/// nested containers, pockets, worn, wielded) and nearby ground items within
/// `view_radius` (Manhattan tiles, nearest-first).
pub fn observe_actor(
    actor: Entity,
    world: &mut World,
    view_radius: i32,
    catalog: &ItemCatalog,
) -> ActorObservation {
    let inventory = InventoryModel {
        items: observe_items(
            crate::inventory::systems::all_items_for_creature(actor, world),
            world,
            catalog,
        ),
    };

    let actor_pos = world.get::<WorldPosition>(actor).map(|p| p.get());
    let mut nearby: Vec<ObservedItem> = Vec::new();
    {
        let mut q = world.query_filtered::<(Entity, &WorldPosition, &DefOrigin), (
            Without<InsideContainer>,
            Without<WieldedBy>,
            Without<IsDef>,
        )>();
        for (entity, pos, origin) in q.iter(world) {
            if entity == actor {
                continue;
            }
            let there = pos.get();
            if let Some(here) = actor_pos {
                let dist = (there.x - here.x).abs() + (there.y - here.y).abs();
                if dist > view_radius {
                    continue;
                }
            }
            let mut item = observe_item(entity, origin, world, catalog);
            item.pos = Some(there);
            nearby.push(item);
        }
    }
    nearby.sort_by_key(|i| {
        i.pos
            .map(|there| match actor_pos {
                Some(here) => (there.x - here.x).abs() + (there.y - here.y).abs(),
                None => i32::MAX,
            })
            .unwrap_or(i32::MAX)
    });

    let navigation = NavigationModel { pos: actor_pos };

    ActorObservation {
        needs: world
            .get::<super::model::Needs>(actor)
            .map_or_else(NeedsModel::default, |n| NeedsModel {
                hunger: n.hunger,
                thirst: n.thirst,
                fatigue: n.fatigue,
            }),
        inventory,
        nearby: NearbyModel { items: nearby },
        navigation,
    }
}

/// One observed item instance: identity, definition facts, and count.
fn observe_item(
    entity: Entity,
    origin: &DefOrigin,
    world: &World,
    catalog: &ItemCatalog,
) -> ObservedItem {
    let def = catalog
        .def_of(origin)
        .unwrap_or_else(|| format!("<interned:{}>", origin.0));
    let category = world
        .get::<ItemCategory>(entity)
        .map(|c| c.0.clone())
        .or_else(|| catalog.category_of(&def));
    let count = world
        .get::<StackCount>(entity)
        .map(|s| s.get() as i32)
        .unwrap_or(1);
    ObservedItem {
        entity: Some(entity),
        def,
        category,
        count,
        pos: None,
    }
}

fn observe_items(entities: Vec<Entity>, world: &World, catalog: &ItemCatalog) -> Vec<ObservedItem> {
    entities
        .into_iter()
        .filter_map(|e| {
            let origin = world.get::<DefOrigin>(e)?;
            Some(observe_item(e, origin, world, catalog))
        })
        .collect()
}

/// Manhattan tile distance (observation/kernel geometry helper).
pub fn manhattan(a: WorldPos, b: WorldPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
