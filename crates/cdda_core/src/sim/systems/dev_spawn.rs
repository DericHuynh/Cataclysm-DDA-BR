//! Debug spawn panel — sim-side logic.
//!
//! The item catalog is built dynamically from all entities carrying `IsDef` +
//! `ItemName` + `DefStrId` in the main world, so every item imported from JSON
//! appears in the panel automatically.
//!
//! Spawning uses `spawn_item` (EntityCloner), which needs `&mut World`.  A
//! lightweight `DevSpawnQueue` resource bridges the gap: the normal input system
//! enqueues a def-entity, and the exclusive `dev_spawn_flush` system drains the
//! queue and calls `spawn_item` with full world access.

use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use crate::coords::WorldPos;
use crate::ZLevel;
use crate::input::{GameAction, InputAction};

use  crate::sim::def_components::{DefStrId, IsDef, ItemName};
use crate::sim::systems::dev_move::DevCamera;
use crate::sim::systems::inventory::DevGroundItemName;
use crate::item::components::ItemTypeId;

// ---------------------------------------------------------------------------
// Catalog entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DevCatalogEntry {
    /// The definition entity in the world — passed to `spawn_item`.
    pub def_entity: Entity,
    /// Display name (from `ItemName`).
    pub name: String,
    /// JSON string ID (from `DefStrId`), shown alongside the name.
    pub def_id: String,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Tracks focus index and the sorted item catalog for the debug spawn panel.
#[derive(Resource, Debug, Default)]
pub struct DevSpawnFocus {
    pub index: usize,
    /// Sorted alphabetically by display name.  Empty until first panel open.
    pub catalog: Vec<DevCatalogEntry>,
}

/// Queue of def-entities to spawn at the current camera tile.
///
/// Written by `dev_spawn_panel_input`, drained by `dev_spawn_flush`.
#[derive(Resource, Debug, Default)]
pub struct DevSpawnQueue(pub Vec<Entity>);

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Build (or rebuild) the sorted item catalog from live `IsDef` entities.
///
/// Registered on `OnEnter(Screen::DevSpawnPanel)` in cdda_app.  The guard
/// (`!catalog.is_empty()`) makes repeated panel opens free after the first.
pub fn build_dev_spawn_catalog(
    mut focus: ResMut<DevSpawnFocus>,
    query: Query<(Entity, &ItemName, &DefStrId), With<IsDef>>,
) {
    if !focus.catalog.is_empty() {
        return;
    }
    let mut entries: Vec<DevCatalogEntry> = query
        .iter()
        .map(|(entity, name, def_id)| DevCatalogEntry {
            def_entity: entity,
            name: name.0.clone(),
            def_id: def_id.0.clone(),
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name).then(a.def_id.cmp(&b.def_id)));
    focus.catalog = entries;
}

/// Handles navigation and spawn requests while `Screen::DevSpawnPanel` is open.
///
/// - **j/k / ↑↓**     — move focus one row
/// - **PgUp/PgDn**    — move focus ten rows
/// - **Home/End**     — jump to first/last
/// - **Enter / e**    — enqueue selected item for spawning (panel stays open)
/// - **Esc / q**      — handled upstream by `handle_navigation_input`
///
/// Gated by `.run_if(in_state(Screen::DevSpawnPanel))` at registration.
pub fn dev_spawn_panel_input(
    mut reader: MessageReader<InputAction>,
    mut focus: ResMut<DevSpawnFocus>,
    mut spawn_queue: ResMut<DevSpawnQueue>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let len = focus.catalog.len();

    for action in actions {
        match action {
            GameAction::NavigateUp => {
                focus.index = focus.index.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                if len > 0 {
                    focus.index = (focus.index + 1).min(len - 1);
                }
            }
            GameAction::NavigatePageUp => {
                focus.index = focus.index.saturating_sub(10);
            }
            GameAction::NavigatePageDown => {
                if len > 0 {
                    focus.index = (focus.index + 10).min(len - 1);
                }
            }
            GameAction::NavigateHome => {
                focus.index = 0;
            }
            GameAction::NavigateEnd => {
                focus.index = len.saturating_sub(1);
            }
            GameAction::Confirm => {
                if let Some(entry) = focus.catalog.get(focus.index) {
                    spawn_queue.0.push(entry.def_entity);
                }
            }
            _ => {}
        }
    }
}

/// Drains `DevSpawnQueue` and spawns each item at the camera's current tile.
///
/// Exclusive system — requires `&mut World` for `EntityCloner` inside
/// `spawn_item`.  Runs after `dev_spawn_panel_input` in the same set.
pub fn dev_spawn_flush(world: &mut World) {
    let camera = world.resource::<DevCamera>().clone();
    let queue: Vec<Entity> = world.resource_mut::<DevSpawnQueue>().0.drain(..).collect();

    if queue.is_empty() {
        return;
    }

    let pos = WorldPos::new(camera.x * 24, camera.y * 24, ZLevel::new(camera.z as i8));

    for def_entity in queue {
        // Capture display name and CDDA string ID from the def before cloning.
        // (EntityCloner denies DefStrId on the clone, so we read it here.)
        let display_name = world
            .get::<ItemName>(def_entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "?".to_string());
        let cdda_id = world
            .get::<DefStrId>(def_entity)
            .map(|d| d.0.clone())
            .unwrap_or_default();

        let spawned = crate::sim::systems::spawning::spawn_item(world, def_entity, pos, 1);

        // Make the item visible in ground rendering and interactable via pickup.
        world
            .entity_mut(spawned)
            .insert(DevGroundItemName(display_name))
            .insert(ItemTypeId(cdda_id));
    }
}
