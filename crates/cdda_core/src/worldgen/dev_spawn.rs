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

use crate::core::coords::WorldPos;
use crate::input::context::{InputContextId, InputContextStack};
use crate::input::{GameAction, InputAction};
use crate::ZLevel;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input::ButtonState;

use crate::core::components::def::{DefStrId, IsDef, ItemName};
use crate::core::components::item::ItemTypeId;
use crate::worldgen::dev::DevGroundItemName;
use crate::worldgen::dev_move::DevCamera;

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
    /// Current filter string (case-insensitive substring match on name/id).
    pub filter: String,
    /// True while the text-input context is active for filter editing.
    pub filtering: bool,
}

impl DevSpawnFocus {
    /// Returns the subset of catalog entries matching the current filter.
    pub fn filtered_entries(&self) -> Vec<&DevCatalogEntry> {
        if self.filter.is_empty() {
            self.catalog.iter().collect()
        } else {
            let q = self.filter.to_lowercase();
            self.catalog
                .iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&q) || e.def_id.to_lowercase().contains(&q)
                })
                .collect()
        }
    }
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
/// Registered on `OnEnter(Ctx::DevSpawnPanel)` in cdda_app.  The guard
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

/// Handles navigation and spawn requests while `Ctx::DevSpawnPanel` is open.
///
/// - **/**              — open filter text input
/// - **j/k / ↑↓**     — move focus one row
/// - **PgUp/PgDn**    — move focus ten rows
/// - **Home/End**     — jump to first/last
/// - **Enter / e**    — enqueue selected item for spawning (panel stays open)
/// - **Esc / q**      — handled upstream by `handle_navigation_input`
///
/// Gated by `.run_if(in_state(Ctx::DevSpawnPanel))` at registration.
pub fn dev_spawn_panel_input(
    mut reader: MessageReader<InputAction>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut focus: ResMut<DevSpawnFocus>,
    mut spawn_queue: ResMut<DevSpawnQueue>,
    mut ctx_stack: ResMut<InputContextStack>,
) {
    if focus.filtering {
        // Drain InputAction so stale messages don't accumulate.
        for _ in reader.read() {}

        // Read keyboard directly — avoids the PreUpdate→Update timing gap
        // that prevents TextChar messages from handle_raw_input reaching here.
        for ev in keyboard.read() {
            if ev.state == ButtonState::Released || ev.repeat {
                continue;
            }
            match &ev.logical_key {
                Key::Character(ch) if !ch.chars().any(|c| c.is_control()) => {
                    // Skip '/' if filter just opened (it was the toggle key)
                    if ch == "/" && focus.filter.is_empty() {
                        continue;
                    }
                    focus.filter.push_str(ch.as_str());
                    focus.index = 0;
                }
                Key::Space => {
                    focus.filter.push(' ');
                    focus.index = 0;
                }
                Key::Backspace => {
                    focus.filter.pop();
                    focus.index = 0;
                }
                Key::Enter => {
                    focus.filtering = false;
                    ctx_stack.pop();
                }
                Key::Escape => {
                    focus.filtering = false;
                    focus.filter.clear();
                    focus.index = 0;
                    ctx_stack.pop();
                }
                _ => {}
            }
        }
        return;
    }

    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    for action in &actions {
        match action {
            // ── Filter text input ─────────────────────────────────────────
            GameAction::Filter => {
                if !focus.filtering {
                    focus.filtering = true;
                    ctx_stack.push(InputContextId::TextInput);
                }
            }
            // ── Navigation ───────────────────────────────────────────────
            GameAction::NavigateUp => {
                focus.index = focus.index.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                let len = focus.filtered_entries().len();
                if len > 0 {
                    focus.index = (focus.index + 1).min(len - 1);
                }
            }
            GameAction::NavigatePageUp => {
                focus.index = focus.index.saturating_sub(10);
            }
            GameAction::NavigatePageDown => {
                let len = focus.filtered_entries().len();
                if len > 0 {
                    focus.index = (focus.index + 10).min(len - 1);
                }
            }
            GameAction::NavigateHome => {
                focus.index = 0;
            }
            GameAction::NavigateEnd => {
                focus.index = focus.filtered_entries().len().saturating_sub(1);
            }
            GameAction::Confirm => {
                let def_entity = focus
                    .filtered_entries()
                    .get(focus.index)
                    .map(|e| e.def_entity);
                if let Some(entity) = def_entity {
                    spawn_queue.0.push(entity);
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

        let spawned = crate::worldgen::spawning::spawn_item(world, def_entity, pos, 1);

        // Make the item visible in ground rendering and interactable via pickup.
        world
            .entity_mut(spawned)
            .insert(DevGroundItemName(display_name))
            .insert(ItemTypeId(cdda_id));
    }
}
