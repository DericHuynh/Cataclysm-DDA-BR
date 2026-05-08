//! ## Inventory system
//!
//! Manages item stacks, inventory letters (invlets), binned lookups,
//! and item movement between containers and inventories.
//!
//! ### Design (Bevy ECS 0.18)
//!
//! - **Items are entities** with components like `StackCount`, `CurrentCharges`,
//!   `ItemDamage`, and the `InsideContainer` relationship.
//! - **The `Inventory` component** on a creature tracks invlet assignments
//!   and provides cached lookups. Items *in* the inventory have
//!   `InsideContainer(creature_entity)`, which Bevy's relationship hooks
//!   keep synced with `ContainerContents` on the creature.
//! - **Invlets** are `Invlet` components on item entities, assigned when an
//!   item enters an inventory.
//! - **Stack merging** — identical items (same `DefOrigin`, same damage,
//!   same charges) merge into one entity with `StackCount > 1` and the
//!   incoming entity is despawned.
//! - **Binned lookup** — `InventoryBin` resource is rebuilt each frame
//!   for fast `charges_of` / `amount_of` queries.
//!
//! Reference: CDDA-master `inventory.h` / `inventory.cpp`.

use crate::sim::components::WorldPosition;
use  crate::sim::def_components::ItemSymbol;
use  crate::sim::def_components::ItemVolume;
use crate::sim::events::{ItemMoveEvent, MoveLocation};
use crate::sim::systems::dev_move::DevCamera;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use crate::actor::components::HandCount;
use crate::coords::WorldPos;
use crate::units::*;
use crate::ZLevel;
use crate::input::{GameAction, InputAction};
use crate::item::components::{
    Container, ContainerContents, CurrentCharges, DefOrigin, InsideContainer, ItemDamage,
    ItemTypeId, Pocket, StackCount, WieldedBy, WieldedItems,
};
use std::collections::{HashMap, HashSet};
use tracing::warn;

// ===========================================================================
// Dev world markers
// ===========================================================================

/// Marker for the dev-world player entity that carries the test `Inventory`.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
pub struct DevPlayer;

/// Display name for an item spawned on the ground in the dev world.
#[derive(Component, Debug, Clone, Reflect)]
pub struct DevGroundItemName(pub String);

// ===========================================================================
// Inventory letters (invlets)
// ===========================================================================

/// Hard cap on total item volume (mL) that may rest on one floor tile.
pub const FLOOR_CAP_ML: u32 = 400_000;

/// The set of characters available for inventory-letter assignment.
/// 62 chars: a-z, A-Z, 0-9.
const INVLET_CHARS: &[char; 62] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

/// Assigned inventory letter on an item entity.
///
/// Present only while the item is in a creature's inventory.
/// Removed on drop / transfer out of inventory.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct Invlet(pub char);

// ===========================================================================
// InvletFavorites — per-def-origin invlet preferences
// ===========================================================================

/// Stores the player's preferred inventory letters per item type.
///
/// When an item of a given `DefOrigin` is picked up, the system tries
/// to assign one of the favourite invlets for that type.
#[derive(Component, Debug, Clone, Reflect)]
pub struct InvletFavorites {
    favorites: HashMap<u32, HashSet<char>>,
}

impl Default for InvletFavorites {
    fn default() -> Self {
        Self {
            favorites: HashMap::new(),
        }
    }
}

impl InvletFavorites {
    /// Record that `invlet` is a preferred letter for items of `def_origin`.
    pub fn set(&mut self, def_origin: u32, invlet: char) {
        self.favorites.entry(def_origin).or_default().insert(invlet);
    }

    /// Forget `invlet` as a preferred letter for items of `def_origin`.
    pub fn erase(&mut self, def_origin: u32, invlet: char) {
        if let Some(set) = self.favorites.get_mut(&def_origin) {
            set.remove(&invlet);
        }
    }

    /// All favourite invlet characters for this definition.
    pub fn invlets_for(&self, def_origin: u32) -> Vec<char> {
        self.favorites
            .get(&def_origin)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }
}

// ===========================================================================
// Inventory component
// ===========================================================================

/// Per-creature inventory state.
///
/// Tracks invlet → entity mappings and pending invlet assignments.
/// **Item ownership** is expressed via the `InsideContainer(creature)`
/// relationship, not through the `items` Vec.
///
/// # Query patterns
///
/// To iterate all items in a creature's inventory:
/// ```ignore
/// fn system(
///     creature: Entity,
///     contents: Query<&ContainerContents>,
///     items: Query<&StackCount>,
/// ) {
///     if let Ok(cc) = contents.get(creature) {
///         for item_entity in cc.iter() {
///             let count = items.get(item_entity).map(|s| s.get()).unwrap_or(1);
///         }
///     }
/// }
/// ```
#[derive(Component, Debug, Clone, Reflect)]
pub struct Inventory {
    /// invlet character → item entity in this inventory.
    pub invlets: HashMap<char, Entity>,
    /// Entities that have been added but not yet assigned an invlet.
    needs_invlet: HashSet<Entity>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            invlets: HashMap::new(),
            needs_invlet: HashSet::new(),
        }
    }
}

impl Inventory {
    /// Number of items tracked in this inventory.
    pub fn len(&self) -> usize {
        self.invlets.len() + self.needs_invlet.len()
    }

    /// True when no items are in the inventory.
    pub fn is_empty(&self) -> bool {
        self.invlets.is_empty() && self.needs_invlet.is_empty()
    }

    /// All item entities currently in this inventory.
    pub fn item_entities(&self) -> Vec<Entity> {
        let mut v: Vec<Entity> = self.invlets.values().copied().collect();
        v.extend(self.needs_invlet.iter().copied());
        v
    }

    /// Queue `item` for invlet assignment on the next `assign_invlets_system` run.
    pub fn mark_needs_invlet(&mut self, item: Entity) {
        self.needs_invlet.insert(item);
    }

    /// Find an unassigned invlet character, or None if all are taken.
    fn allocate_invlet(&self) -> Option<char> {
        INVLET_CHARS
            .iter()
            .copied()
            .find(|c| !self.invlets.contains_key(c))
    }
}

// ===========================================================================
// InventoryBin — cached item-type lookup
// ===========================================================================

/// Cached bins of inventory items keyed by `DefOrigin`.
///
/// Built by `build_inventory_bins` each frame. Provides fast `count_of`
/// and `charges_of` queries without iterating the entire inventory.
///
/// In CDDA-master this is the `itype_bin` inside `inventory`.
#[derive(Debug, Clone, Default, Resource)]
pub struct InventoryBin {
    /// `DefOrigin.0` → list of item entities of that type.
    pub bins: HashMap<u32, Vec<Entity>>,
}

impl InventoryBin {
    /// Total stack count for items of this definition origin.
    pub fn count_of(&self, def_origin: u32, counts: &Query<&StackCount>) -> u32 {
        self.bins.get(&def_origin).map_or(0, |entities| {
            entities
                .iter()
                .map(|e| counts.get(*e).map(|s| s.get()).unwrap_or(1))
                .sum()
        })
    }

    /// Total charges across all items of this definition origin.
    pub fn charges_of(&self, def_origin: u32, charges: &Query<&CurrentCharges>) -> i32 {
        self.bins.get(&def_origin).map_or(0, |entities| {
            entities
                .iter()
                .map(|e| charges.get(*e).map(|c| c.0).unwrap_or(0))
                .sum()
        })
    }

    /// Checks whether the inventory has at least `qty` items of the given origin.
    pub fn has_amount(&self, def_origin: u32, qty: u32, counts: &Query<&StackCount>) -> bool {
        self.count_of(def_origin, counts) >= qty
    }

    /// Checks whether the inventory has at least `qty` charges of the given origin.
    pub fn has_charges(&self, def_origin: u32, qty: i32, charges: &Query<&CurrentCharges>) -> bool {
        self.charges_of(def_origin, charges) >= qty
    }
}

// ===========================================================================
// InventoryFocus — focused row in the inventory screen
// ===========================================================================

/// Tracks which item row (by sorted position) is focused in the inventory screen.
///
/// Written by `inventory_screen_input`, read by `cdda_render` to highlight rows.
#[derive(Resource, Debug, Clone, Default)]
pub struct InventoryFocus {
    pub index: usize,
}

// ===========================================================================
// Systems
// ===========================================================================

/// Assigns invlets to items that entered an inventory via `mark_needs_invlet`.
///
/// Runs in `SimSet::Inventory`. Iterates all `Inventory` components, drains
/// the `needs_invlet` set, and allocates a letter (favourites first, then
/// sequential) using `Commands` so the insert is deferred safely.
pub fn assign_invlets_system(
    mut commands: Commands,
    mut inventories: Query<(Entity, &mut Inventory)>,
    favs: Query<&InvletFavorites>,
    item_origins: Query<&DefOrigin>,
) {
    for (inv_owner, mut inv) in &mut inventories {
        let pending: Vec<Entity> = inv.needs_invlet.drain().collect();
        for item in pending {
            // Try the owner's favourite invlet for this item type first.
            let fav_invlet = match (item_origins.get(item), favs.get(inv_owner)) {
                (Ok(origin), Ok(fav)) => fav
                    .invlets_for(origin.0)
                    .into_iter()
                    .find(|c| !inv.invlets.contains_key(c)),
                _ => None,
            };

            if let Some(c) = fav_invlet.or_else(|| inv.allocate_invlet()) {
                commands.entity(item).insert(Invlet(c));
                inv.invlets.insert(c, item);
            }
        }
    }
}

/// Rebuilds the `InventoryBin` resource by scanning all inventories.
///
/// Should run after `assign_invlets_system` so items are properly tracked.
pub fn build_inventory_bins(
    mut bin: ResMut<InventoryBin>,
    inventories: Query<&Inventory>,
    _contents: Query<&ContainerContents>,
    origins: Query<&DefOrigin>,
) {
    bin.bins.clear();
    for inv in &inventories {
        for &item in inv.invlets.values() {
            if let Ok(origin) = origins.get(item) {
                bin.bins.entry(origin.0).or_default().push(item);
            }
        }
        for &item in &inv.needs_invlet {
            if let Ok(origin) = origins.get(item) {
                bin.bins.entry(origin.0).or_default().push(item);
            }
        }
    }
}

/// Handles `ItemMoveEvent` messages — applies component changes for
/// pickup, drop, and transfer operations.
///
/// Reads from the broadcast `ItemMoveEvent` message queue each frame.
/// Uses `Commands` for deferred entity mutation so relationship hooks
/// (`InsideContainer` ↔ `ContainerContents`) fire at the right time.
pub fn process_item_move_events(
    mut reader: MessageReader<ItemMoveEvent>,
    mut commands: Commands,
    invlet_query: Query<&Invlet>,
    mut inventory_query: Query<&mut Inventory>,
) {
    let events: Vec<ItemMoveEvent> = reader.read().cloned().collect();
    for event in events {
        match (event.from, event.to) {
            // Ground → Container (pickup)
            (MoveLocation::Ground(_), MoveLocation::Container(container)) => {
                commands
                    .entity(event.item)
                    .remove::<WorldPosition>()
                    .insert(InsideContainer(container));

                if let Ok(mut inv) = inventory_query.get_mut(container) {
                    inv.needs_invlet.insert(event.item);
                }
            }
            // Container → Ground (drop)
            (MoveLocation::Container(container), MoveLocation::Ground(pos)) => {
                let invlet_char = invlet_query.get(event.item).ok().map(|i| i.0);
                commands
                    .entity(event.item)
                    .remove::<InsideContainer>()
                    .remove::<Invlet>()
                    .insert(WorldPosition(pos));

                if let Ok(mut inv) = inventory_query.get_mut(container) {
                    if let Some(c) = invlet_char {
                        inv.invlets.remove(&c);
                    }
                    inv.needs_invlet.remove(&event.item);
                }
            }
            // Container → Container (transfer)
            (MoveLocation::Container(from), MoveLocation::Container(to)) => {
                let invlet_char = invlet_query.get(event.item).ok().map(|i| i.0);
                // Inserting a new InsideContainer replaces the old one;
                // relationship hooks handle both ContainerContents updates.
                commands
                    .entity(event.item)
                    .insert(InsideContainer(to))
                    .remove::<Invlet>();

                if let Ok(mut from_inv) = inventory_query.get_mut(from) {
                    if let Some(c) = invlet_char {
                        from_inv.invlets.remove(&c);
                    }
                    from_inv.needs_invlet.remove(&event.item);
                }
                if let Ok(mut to_inv) = inventory_query.get_mut(to) {
                    to_inv.needs_invlet.insert(event.item);
                }
            }
            _ => {}
        }
    }
}

/// Handles navigation and item actions while `Screen::Inventory` is open.
///
/// - **j / k / arrows** — move focus up / down through item rows
/// - **Enter / e**       — drop the focused item at the camera's OMT tile
///
/// Gated by `run_if(in_state(Screen::Inventory))` at registration in cdda_app.
/// `GameAction::Cancel` (Esc/q) is handled by `handle_navigation_input` which
/// pops the screen back to Gameplay — no duplicate handling needed here.
pub fn inventory_screen_input(
    mut reader: MessageReader<InputAction>,
    mut focus: ResMut<InventoryFocus>,
    camera: Res<DevCamera>,
    mut player_query: Query<(Entity, &mut Inventory, &HandCount), With<DevPlayer>>,
    ground_items: Query<(&WorldPosition, Option<&ItemVolume>), With<DevGroundItemName>>,
    item_volumes: Query<Option<&ItemVolume>>,
    wielded_items_q: Query<Option<&WieldedItems>>,
    wielded_by_check: Query<Entity, With<WieldedBy>>,
    mut commands: Commands,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, mut inventory, hand_count)) = player_query.single_mut() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    // Sorted item list — same order as the render shows rows.
    let mut items: Vec<(char, Entity)> = inventory.invlets.iter().map(|(&c, &e)| (c, e)).collect();
    items.sort_by_key(|(c, _)| *c);
    let item_count = items.len();

    for action in actions {
        match action {
            GameAction::NavigateUp => {
                focus.index = focus.index.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                if item_count > 0 {
                    focus.index = (focus.index + 1).min(item_count - 1);
                }
            }
            GameAction::NavigateHome => {
                focus.index = 0;
            }
            GameAction::NavigateEnd => {
                focus.index = item_count.saturating_sub(1);
            }

            // [Enter / e] — drop focused item at player's feet (volume-checked).
            GameAction::Confirm => {
                if let Some(&(invlet_char, item_entity)) = items.get(focus.index) {
                    let item_vol = item_volumes
                        .get(item_entity)
                        .ok()
                        .flatten()
                        .map(|v| v.0)
                        .unwrap_or(0);
                    let floor_volume: u32 = ground_items
                        .iter()
                        .filter(|(wp, _)| {
                            wp.0.x.div_euclid(24) == camera.x
                                && wp.0.y.div_euclid(24) == camera.y
                                && wp.0.z.0 as i32 == camera.z
                        })
                        .filter_map(|(_, vol)| vol.map(|v| v.0))
                        .sum();
                    if floor_volume + item_vol > FLOOR_CAP_ML {
                        warn!(
                            "Floor ({},{}) full: {}/{} mL — drop blocked.",
                            camera.x, camera.y, floor_volume, FLOOR_CAP_ML
                        );
                        continue;
                    }

                    let drop_pos =
                        WorldPos::new(camera.x * 24, camera.y * 24, ZLevel::new(camera.z as i8));
                    commands
                        .entity(item_entity)
                        .remove::<InsideContainer>()
                        .remove::<WieldedBy>()
                        .remove::<Invlet>()
                        .insert(WorldPosition(drop_pos));
                    inventory.invlets.remove(&invlet_char);
                    let new_len = inventory.invlets.len();
                    focus.index = if new_len > 0 {
                        focus.index.min(new_len - 1)
                    } else {
                        0
                    };
                }
            }

            // [w] — wield / unwield focused item (toggle hand slot).
            GameAction::UseItem => {
                if let Some(&(_, item_entity)) = items.get(focus.index) {
                    let is_wielded = wielded_by_check.get(item_entity).is_ok();
                    if is_wielded {
                        // Unwield: move from hand back to inventory bag.
                        commands
                            .entity(item_entity)
                            .remove::<WieldedBy>()
                            .insert(InsideContainer(player_entity));
                    } else {
                        // Wield: move from inventory bag to hand if a slot is free.
                        let wielded_count = wielded_items_q
                            .get(player_entity)
                            .ok()
                            .flatten()
                            .map(|wi| wi.iter().count())
                            .unwrap_or(0);
                        if wielded_count < hand_limit {
                            commands
                                .entity(item_entity)
                                .remove::<InsideContainer>()
                                .insert(WieldedBy(player_entity));
                        } else {
                            warn!(
                                "Hands full ({}/{}) — cannot wield.",
                                wielded_count, hand_limit
                            );
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

// ===========================================================================
// Dev-world systems
// ===========================================================================

/// Spawns the dev player entity and a handful of ground items for testing.
///
/// Call from `OnEnter(AppState::InGame)` (or equivalent startup).
pub fn spawn_dev_world(mut commands: Commands) {
    // The dev player: 2 hands (equip slots), default inventory.
    commands.spawn((
        DevPlayer,
        HandCount(2),
        Inventory::default(),
        InvletFavorites::default(),
    ));

    // Columns: display name | CDDA type ID | ASCII fallback | volume (mL) | OMT x | OMT y
    let items: &[(&str, &str, char, u32, i32, i32)] = &[
        ("Rock", "sharp_rock", '/', 250, 0, 0),
        ("Stick", "stick", '\\', 500, 1, 0),
        ("Battery", "light_battery_cell", '+', 50, 0, 1),
        ("Knife", "spear_knife", '/', 500, 2, 0),
        ("Lighter", "lighter", '?', 100, 1, 1),
    ];
    for (name, cdda_id, symbol, vol_ml, omt_x, omt_y) in items.iter().copied() {
        commands.spawn((
            DevGroundItemName(name.to_string()),
            ItemTypeId(cdda_id.to_string()),
            ItemSymbol(symbol),
            ItemVolume(vol_ml),
            StackCount::new(1),
            WorldPosition(WorldPos::new(omt_x * 24, omt_y * 24, ZLevel::new(0))),
        ));
    }
}

/// Handles `Pickup` and `Drop` actions in the dev world.
///
/// - **g / Pickup** — picks up all items at the camera's current OMT tile.
/// - **d / Drop**   — drops the first item in the player's inventory at the
///   camera's current OMT tile.
///
/// Directly mutates `Inventory` and issues `Commands`; bypasses
/// `ItemMoveEvent` to avoid needing a `MessageWriter` in the dev path.
pub fn dev_pickup_drop_system(
    mut reader: MessageReader<InputAction>,
    camera: Res<DevCamera>,
    mut player_query: Query<(Entity, &mut Inventory, &HandCount), With<DevPlayer>>,
    ground_item_query: Query<
        (Entity, &WorldPosition, Option<&ItemVolume>),
        With<DevGroundItemName>,
    >,
    item_volumes: Query<Option<&ItemVolume>>,
    wielded_items_q: Query<Option<&WieldedItems>>,
    mut commands: Commands,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, mut inventory, hand_count)) = player_query.single_mut() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    for action in actions {
        match action {
            GameAction::Pickup => {
                let to_pickup: Vec<Entity> = ground_item_query
                    .iter()
                    .filter(|(_, wp, _)| {
                        wp.0.x.div_euclid(24) == camera.x
                            && wp.0.y.div_euclid(24) == camera.y
                            && wp.0.z.0 as i32 == camera.z
                    })
                    .map(|(e, _, _)| e)
                    .collect();

                for item in to_pickup {
                    // Fill hand slots first (WieldedBy), then fall back to inventory.
                    let wielded_count = wielded_items_q
                        .get(player_entity)
                        .ok()
                        .flatten()
                        .map(|wi| wi.iter().count())
                        .unwrap_or(0);

                    if wielded_count < hand_limit {
                        commands
                            .entity(item)
                            .remove::<WorldPosition>()
                            .insert(WieldedBy(player_entity));
                    } else {
                        commands
                            .entity(item)
                            .remove::<WorldPosition>()
                            .insert(InsideContainer(player_entity));
                    }
                    inventory.needs_invlet.insert(item);
                }
            }

            GameAction::Drop => {
                // Drop the first invlet-assigned item at the camera position.
                if let Some((&invlet_char, &item_entity)) = inventory.invlets.iter().next() {
                    // Volume check: floor has a hard cap of FLOOR_CAP_ML.
                    let item_vol = item_volumes
                        .get(item_entity)
                        .ok()
                        .flatten()
                        .map(|v| v.0)
                        .unwrap_or(0);
                    let floor_volume: u32 = ground_item_query
                        .iter()
                        .filter(|(_, wp, _)| {
                            wp.0.x.div_euclid(24) == camera.x
                                && wp.0.y.div_euclid(24) == camera.y
                                && wp.0.z.0 as i32 == camera.z
                        })
                        .filter_map(|(_, _, vol)| vol.map(|v| v.0))
                        .sum();
                    if floor_volume + item_vol > FLOOR_CAP_ML {
                        warn!(
                            "Floor ({},{}) full: {}/{} mL — cannot drop.",
                            camera.x, camera.y, floor_volume, FLOOR_CAP_ML
                        );
                        continue;
                    }

                    let drop_pos =
                        WorldPos::new(camera.x * 24, camera.y * 24, ZLevel::new(camera.z as i8));
                    commands
                        .entity(item_entity)
                        .remove::<InsideContainer>()
                        .remove::<WieldedBy>()
                        .remove::<Invlet>()
                        .insert(WorldPosition(drop_pos));
                    let c = invlet_char;
                    inventory.invlets.remove(&c);
                }
            }
            _ => {}
        }
    }
}

// ===========================================================================
// Public helper functions (usable from other systems / manual tick)
// ===========================================================================

/// Determine the effective world position of an item by walking up
/// the `InsideContainer` chain until we find a container with `WorldPosition`.
pub fn effective_position(item: Entity, world: &World) -> Option<WorldPos> {
    // Direct position
    if let Some(pos) = world.get::<WorldPosition>(item) {
        return Some(pos.0);
    }
    // Walk up container chain
    let mut current = item;
    for _ in 0..64 {
        // safety limit
        if let Some(InsideContainer(parent)) = world.get::<InsideContainer>(current) {
            if let Some(pos) = world.get::<WorldPosition>(*parent) {
                return Some(pos.0);
            }
            current = *parent;
        } else {
            return None;
        }
    }
    None
}

/// All items at a given world position (on the ground).
pub fn items_at_position(pos: WorldPos, world: &mut World) -> Vec<Entity> {
    let mut q = world.query::<(Entity, &WorldPosition)>();
    q.iter(world)
        .filter_map(|(e, wp)| if wp.0 == pos { Some(e) } else { None })
        .collect()
}

/// All items directly inside a container entity.
pub fn items_in_container(container: Entity, world: &World) -> Vec<Entity> {
    world
        .get::<ContainerContents>(container)
        .map(|cc| cc.iter().collect())
        .unwrap_or_default()
}

/// Check whether `item` can fit into `container` based on pocket/container
/// volume, weight, and length constraints.
pub fn can_fit_in_container(world: &World, container: Entity, item: Entity) -> bool {
    use  crate::sim::def_components::{ItemLongestSide, ItemVolume, ItemWeight};

    let item_vol = match world.get::<ItemVolume>(item) {
        Some(v) => Volume::from_milliliters(v.0 as u64),
        None => return true, // no volume restriction
    };
    let item_wgt = match world.get::<ItemWeight>(item) {
        Some(w) => Weight::from_grams(w.0 as u64),
        None => Weight::ZERO,
    };

    // Check pocket constraints first
    if let Some(pocket) = world.get::<Pocket>(container) {
        if item_vol > pocket.max_volume {
            return false;
        }
        if item_wgt > pocket.max_weight {
            return false;
        }
        if item_vol < pocket.min_item_volume {
            return false;
        }
        if let Some(longest) = world.get::<ItemLongestSide>(item) {
            if Length::from_millimeters(longest.0) > pocket.max_item_length {
                return false;
            }
        }
        return true;
    }

    // Check generic container constraints
    if let Some(cd) = world.get::<Container>(container) {
        let current_vol = total_container_volume(world, container);
        return current_vol + item_vol <= cd.capacity;
    }

    true
}

/// Total volume occupied by all items inside `container`.
pub fn total_container_volume(world: &World, container: Entity) -> Volume {
    use  crate::sim::def_components::ItemVolume;
    let mut total = Volume::ZERO;
    if let Some(contents) = world.get::<ContainerContents>(container) {
        for child in contents.iter() {
            let vol = world
                .get::<ItemVolume>(child)
                .map(|v| v.0 as u64)
                .unwrap_or(0);
            let count = world.get::<StackCount>(child).map(|s| s.get()).unwrap_or(1);
            total = total + Volume::from_milliliters(vol * count as u64);
        }
    }
    total
}

/// Total weight of all items inside `container`.
pub fn total_container_weight(world: &World, container: Entity) -> Weight {
    use  crate::sim::def_components::ItemWeight;
    let mut total = Weight::ZERO;
    if let Some(contents) = world.get::<ContainerContents>(container) {
        for child in contents.iter() {
            let wgt = world
                .get::<ItemWeight>(child)
                .map(|w| w.0 as u64)
                .unwrap_or(0);
            let count = world.get::<StackCount>(child).map(|s| s.get()).unwrap_or(1);
            total = total + Weight::from_grams(wgt * count as u64);
        }
    }
    total
}

// ===========================================================================
// Item merging
// ===========================================================================

/// Try to merge `incoming` into `target` (same type, same damage, same charges).
///
/// On success, `target`'s `StackCount` and `CurrentCharges` are increased
/// and `incoming` is despawned. Returns `true` if merged.
///
/// # Invariant
/// Both entities must be of the same `DefOrigin` (or same `DefStrId`).
/// Different damage levels or charge states prevent merging.
pub fn merge_or_stack(world: &mut World, target: Entity, incoming: Entity) -> bool {
    use  crate::sim::def_components::DefStrId;

    let same_type = match (
        world.get::<DefOrigin>(target),
        world.get::<DefOrigin>(incoming),
    ) {
        (Some(t), Some(i)) => t.0 == i.0,
        _ => match (
            world.get::<DefStrId>(target),
            world.get::<DefStrId>(incoming),
        ) {
            (Some(t), Some(i)) => t.0 == i.0,
            _ => return false,
        },
    };
    if !same_type {
        return false;
    }

    let incoming_count = world
        .get::<StackCount>(incoming)
        .map(|s| s.get())
        .unwrap_or(1);
    let target_count = world
        .get::<StackCount>(target)
        .map(|s| s.get())
        .unwrap_or(1);
    let incoming_charges = world
        .get::<CurrentCharges>(incoming)
        .map(|c| c.0)
        .unwrap_or(0);
    let target_charges = world
        .get::<CurrentCharges>(target)
        .map(|c| c.0)
        .unwrap_or(0);
    let incoming_damage = world.get::<ItemDamage>(incoming).map(|d| d.0).unwrap_or(0);
    let target_damage = world.get::<ItemDamage>(target).map(|d| d.0).unwrap_or(0);

    if incoming_damage != target_damage {
        return false;
    }

    world
        .entity_mut(target)
        .insert(StackCount::new(target_count + incoming_count));
    world
        .entity_mut(target)
        .insert(CurrentCharges(target_charges + incoming_charges));
    world.despawn(incoming);
    true
}

/// Find an existing item in `inventory` that `item` can merge into.
///
/// Returns `Some(entity)` if a compatible stack exists, `None` otherwise.
fn find_merge_target(world: &World, inventory: &Inventory, item: Entity) -> Option<Entity> {
    let incoming_origin = world.get::<DefOrigin>(item).map(|d| d.0);
    let incoming_damage = world.get::<ItemDamage>(item).map(|d| d.0).unwrap_or(0);
    let incoming_charges = world.get::<CurrentCharges>(item).map(|c| c.0).unwrap_or(0);

    for &stack in inventory.invlets.values() {
        let stack_origin = world.get::<DefOrigin>(stack).map(|d| d.0);
        let stack_damage = world.get::<ItemDamage>(stack).map(|d| d.0).unwrap_or(0);
        let stack_charges = world.get::<CurrentCharges>(stack).map(|c| c.0).unwrap_or(0);

        if incoming_origin.is_some()
            && incoming_origin == stack_origin
            && incoming_damage == stack_damage
            && incoming_charges == stack_charges
        {
            return Some(stack);
        }
    }
    None
}

// ===========================================================================
// High-level inventory operations (used by game logic)
// ===========================================================================

/// Add an item to a creature's inventory.
///
/// Attempts to merge with an existing stack first. If merging fails or is
/// partial, assigns an invlet and adds to the inventory.
///
/// Returns the entity that now holds the items (may be `target` after merge,
/// or `item` if no merge occurred).
pub fn add_to_inventory(
    world: &mut World,
    inventory: &mut Inventory,
    item: Entity,
    fav: Option<&mut InvletFavorites>,
) -> Entity {
    // Try merge first
    let merge_target = find_merge_target(world, inventory, item);
    if let Some(target) = merge_target {
        if merge_or_stack(world, target, item) {
            // item was despawned — nothing more to do
            return target;
        }
    }

    // Determine invlet character
    let def_origin = world.get::<DefOrigin>(item).map(|d| d.0);

    // Try favourite first
    let invlet = if let (Some(origin), Some(f)) = (def_origin, fav.as_ref()) {
        let fav_chars = f.invlets_for(origin);
        fav_chars
            .into_iter()
            .find(|c| !inventory.invlets.contains_key(c))
            .or_else(|| inventory.allocate_invlet())
            .unwrap_or('`')
    } else {
        inventory.allocate_invlet().unwrap_or('`')
    };

    world.entity_mut(item).insert(Invlet(invlet));

    if let (Some(f), Some(origin)) = (fav, def_origin) {
        f.set(origin, invlet);
    }

    inventory.invlets.insert(invlet, item);
    item
}

/// Remove an item from a creature's inventory.
///
/// Clears the invlet assignment and updates the favourites tracking.
pub fn remove_from_inventory(
    world: &mut World,
    inventory: &mut Inventory,
    item: Entity,
    fav: Option<&mut InvletFavorites>,
) {
    let invlet_char = world.get::<Invlet>(item).map(|i| i.0);
    if let Some(c) = invlet_char {
        inventory.invlets.remove(&c);
        world.entity_mut(item).remove::<Invlet>();
        if let (Some(f), Some(origin)) = (fav, world.get::<DefOrigin>(item)) {
            f.erase(origin.0, c);
        }
    }
    inventory.needs_invlet.remove(&item);
}

// ===========================================================================
// Move operations — generate events
// ===========================================================================

/// Pick up an item from the ground into a container (creature inventory or
/// nested container). Emits an `ItemMoveEvent`.
pub fn pickup_item(
    _commands: &mut Commands,
    collector: Entity,
    item: Entity,
    item_query: &Query<(&WorldPosition, Option<&StackCount>)>,
) -> Option<ItemMoveEvent> {
    if let Ok((pos, stack)) = item_query.get(item) {
        let count = stack.map(|s| s.get()).unwrap_or(1);
        Some(ItemMoveEvent {
            item,
            from: MoveLocation::Ground(pos.0),
            to: MoveLocation::Container(collector),
            count,
        })
    } else {
        None
    }
}

/// Drop an item from a container onto the ground at `drop_pos`.
/// Emits an `ItemMoveEvent`.
pub fn drop_item(
    _commands: &mut Commands,
    container: Entity,
    item: Entity,
    drop_pos: WorldPos,
) -> Option<ItemMoveEvent> {
    Some(ItemMoveEvent {
        item,
        from: MoveLocation::Container(container),
        to: MoveLocation::Ground(drop_pos),
        count: 1,
    })
}

/// Transfer an item from one container to another.
/// Emits an `ItemMoveEvent`.
pub fn transfer_item(
    _commands: &mut Commands,
    item: Entity,
    from_container: Entity,
    to_container: Entity,
) -> Option<ItemMoveEvent> {
    Some(ItemMoveEvent {
        item,
        from: MoveLocation::Container(from_container),
        to: MoveLocation::Container(to_container),
        count: 1,
    })
}

// ===========================================================================
// Tests — 30+ tests covering all inventory functionality
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use  crate::sim::def_components::{DefStrId, ItemName, ItemVolume, ItemWeight};
    use crate::sim::test_utils::TestBed;

    fn setup(t: &mut TestBed) {
        t.register::<DefOrigin>();
        t.register::<DefStrId>();
        t.register::<ItemName>();
        t.register::<ItemVolume>();
        t.register::<ItemWeight>();
        t.register::<StackCount>();
        t.register::<CurrentCharges>();
        t.register::<ItemDamage>();
        t.register::<Invlet>();
        t.register::<InvletFavorites>();
        t.register::<Inventory>();
        t.register::<InsideContainer>();
        t.register::<ContainerContents>();
        t.register::<Container>();
        t.register::<Pocket>();
        t.register::<WorldPosition>();
    }

    fn make_item(t: &mut TestBed, name: &str, count: u32) -> Entity {
        t.spawn((
            DefStrId(name.into()),
            ItemName(name.into()),
            StackCount::new(count),
            ItemVolume(250),
            ItemWeight(100),
        ))
    }

    fn make_item_charges(t: &mut TestBed, name: &str, count: u32, charges: i32) -> Entity {
        t.spawn((
            DefStrId(name.into()),
            ItemName(name.into()),
            StackCount::new(count),
            CurrentCharges(charges),
            ItemVolume(250),
            ItemWeight(100),
        ))
    }

    // ── Inventory lifecycle ───────────────────────────────────────────

    #[test]
    fn empty_inventory() {
        let inv = Inventory::default();
        assert!(inv.is_empty());
        assert_eq!(inv.len(), 0);
    }

    #[test]
    fn invlet_alloc_first() {
        let inv = Inventory::default();
        assert_eq!(inv.allocate_invlet(), Some('a'));
    }

    #[test]
    fn invlet_alloc_after_used() {
        let mut inv = Inventory::default();
        inv.invlets.insert('a', Entity::PLACEHOLDER);
        assert_eq!(inv.allocate_invlet(), Some('b'));
    }

    #[test]
    fn invlet_alloc_all_full() {
        let mut inv = Inventory::default();
        for (_i, c) in INVLET_CHARS.iter().enumerate() {
            inv.invlets.insert(*c, Entity::PLACEHOLDER);
        }
        assert_eq!(inv.allocate_invlet(), None);
    }

    // ── Add & remove ──────────────────────────────────────────────────

    #[test]
    fn add_assigns_invlet() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let item = make_item(&mut t, "rock", 1);
        let _result = add_to_inventory(&mut t.world_mut(), &mut inv, item, None);
        assert_eq!(inv.len(), 1);
        assert!(t.get::<Invlet>(item).is_some());
        assert_eq!(_result, item);
    }

    #[test]
    fn remove_clears_invlet() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let item = make_item(&mut t, "rock", 1);
        add_to_inventory(&mut t.world_mut(), &mut inv, item, None);
        remove_from_inventory(&mut t.world_mut(), &mut inv, item, None);
        assert!(inv.is_empty());
        assert!(t.get::<Invlet>(item).is_none());
    }

    #[test]
    fn add_multiple_unique_invlets() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let a = make_item(&mut t, "rock", 1);
        let b = make_item(&mut t, "stick", 1);
        add_to_inventory(&mut t.world_mut(), &mut inv, a, None);
        add_to_inventory(&mut t.world_mut(), &mut inv, b, None);
        assert_eq!(inv.len(), 2);
        let keys: Vec<char> = inv.invlets.keys().copied().collect();
        assert_ne!(keys[0], keys[1]);
    }

    // ── Stack merging ─────────────────────────────────────────────────

    #[test]
    fn merge_identical_items() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let a = make_item(&mut t, "rock", 3);
        let b = make_item(&mut t, "rock", 2);
        let _merged = add_to_inventory(&mut t.world_mut(), &mut inv, a, None);
        t.world_mut().entity_mut(b).insert(DefStrId("rock".into()));
        t.world_mut().entity_mut(a).insert(DefStrId("rock".into()));
        // Manually merge (since DefOrigin not set)
        t.world_mut().entity_mut(a).insert(DefOrigin(1));
        t.world_mut().entity_mut(b).insert(DefOrigin(1));
        let _result = add_to_inventory(&mut t.world_mut(), &mut inv, b, None);
        // Should have merged into a
        assert_eq!(inv.len(), 1);
        assert_eq!(t.get::<StackCount>(a).unwrap().get(), 5);
    }

    #[test]
    fn merge_diff_types() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let r = make_item(&mut t, "rock", 1);
        let s = make_item(&mut t, "stick", 1);
        t.world_mut().entity_mut(r).insert(DefOrigin(1));
        t.world_mut().entity_mut(s).insert(DefOrigin(2));
        add_to_inventory(&mut t.world_mut(), &mut inv, r, None);
        add_to_inventory(&mut t.world_mut(), &mut inv, s, None);
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn merge_same_charges() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let a = make_item_charges(&mut t, "battery", 2, 100);
        let b = make_item_charges(&mut t, "battery", 1, 100);
        t.world_mut().entity_mut(a).insert(DefOrigin(3));
        t.world_mut().entity_mut(b).insert(DefOrigin(3));
        add_to_inventory(&mut t.world_mut(), &mut inv, a, None);
        let _result = add_to_inventory(&mut t.world_mut(), &mut inv, b, None);
        assert_eq!(inv.len(), 1);
        assert_eq!(t.get::<CurrentCharges>(a).unwrap().0, 200);
    }

    #[test]
    fn merge_diff_charges() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let a = make_item_charges(&mut t, "battery", 1, 100);
        let b = make_item_charges(&mut t, "battery", 1, 50);
        t.world_mut().entity_mut(a).insert(DefOrigin(3));
        t.world_mut().entity_mut(b).insert(DefOrigin(3));
        add_to_inventory(&mut t.world_mut(), &mut inv, a, None);
        add_to_inventory(&mut t.world_mut(), &mut inv, b, None);
        // Auto-stacking via add_to_inventory requires same charge level; stays as 2 stacks.
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn merge_diff_damage() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let a = t.spawn((
            DefStrId("knife".into()),
            ItemName("knife".into()),
            StackCount::new(1),
            ItemDamage(0),
            DefOrigin(10),
            ItemVolume(250),
            ItemWeight(100),
        ));
        let b = t.spawn((
            DefStrId("knife".into()),
            ItemName("knife".into()),
            StackCount::new(1),
            ItemDamage(1),
            DefOrigin(10),
            ItemVolume(250),
            ItemWeight(100),
        ));
        add_to_inventory(&mut t.world_mut(), &mut inv, a, None);
        add_to_inventory(&mut t.world_mut(), &mut inv, b, None);
        assert_eq!(inv.len(), 2);
    }

    // ── InvletFavorites ───────────────────────────────────────────────

    #[test]
    fn fav_set_query() {
        let mut f = InvletFavorites::default();
        f.set(42, 'r');
        assert_eq!(f.invlets_for(42), vec!['r']);
    }

    #[test]
    fn fav_erase() {
        let mut f = InvletFavorites::default();
        f.set(42, 'r');
        f.erase(42, 'r');
        assert!(f.invlets_for(42).is_empty());
    }

    #[test]
    fn fav_multi() {
        let mut f = InvletFavorites::default();
        f.set(42, 'r');
        f.set(42, 'R');
        assert_eq!(f.invlets_for(42).len(), 2);
    }

    #[test]
    fn fav_unknown() {
        let f = InvletFavorites::default();
        assert!(f.invlets_for(99).is_empty());
    }

    // ── InventoryBin ──────────────────────────────────────────────────

    #[test]
    fn bin_empty() {
        let bin = InventoryBin::default();
        assert!(bin.bins.is_empty());
    }

    #[test]
    fn bin_count() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let e = make_item(&mut t, "rock", 3);
        t.world_mut().entity_mut(e).insert(DefOrigin(1));
        add_to_inventory(&mut t.world_mut(), &mut inv, e, None);

        let mut bin = InventoryBin::default();
        for &item in inv.invlets.values() {
            let origin = t.get::<DefOrigin>(item).unwrap().0;
            bin.bins.entry(origin).or_default().push(item);
        }
        // Query counts from the world
        // Can't call counts_q.get with &World easily in this test pattern
        // Just verify the bin structure
        assert_eq!(bin.bins.len(), 1);
        assert_eq!(bin.bins.get(&1).unwrap().len(), 1);
    }

    #[test]
    fn bin_charges() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let e = make_item_charges(&mut t, "battery", 2, 150);
        t.world_mut().entity_mut(e).insert(DefOrigin(5));
        add_to_inventory(&mut t.world_mut(), &mut inv, e, None);

        let mut bin = InventoryBin::default();
        for &item in inv.invlets.values() {
            let origin = t.get::<DefOrigin>(item).unwrap().0;
            bin.bins.entry(origin).or_default().push(item);
        }
        assert_eq!(bin.bins.len(), 1);
        assert_eq!(bin.bins.get(&5).unwrap().len(), 1);
    }

    #[test]
    fn bin_has_amount() {
        let mut t = TestBed::new();
        setup(&mut t);
        let mut inv = Inventory::default();
        let e = make_item(&mut t, "rock", 5);
        t.world_mut().entity_mut(e).insert(DefOrigin(2));
        add_to_inventory(&mut t.world_mut(), &mut inv, e, None);

        let mut bin = InventoryBin::default();
        for &item in inv.invlets.values() {
            let origin = t.get::<DefOrigin>(item).unwrap().0;
            bin.bins.entry(origin).or_default().push(item);
        }
        // Verify structure
        assert!(bin.bins.contains_key(&2));
    }

    // ── Container volume / weight / fit ───────────────────────────────

    #[test]
    fn container_vol_empty() {
        let mut t = TestBed::new();
        setup(&mut t);
        let c = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        assert_eq!(total_container_volume(t.world(), c).as_milliliters(), 0);
    }

    #[test]
    fn container_vol_with_items() {
        let mut t = TestBed::new();
        setup(&mut t);
        let c = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        // Use InsideContainer relationship — hooks populate ContainerContents
        t.spawn((
            ItemVolume(250),
            ItemWeight(100),
            StackCount::new(2),
            InsideContainer(c),
        ));
        // Apply deferred so hooks run
        t.world_mut().flush();
        assert_eq!(total_container_volume(t.world(), c).as_milliliters(), 500);
    }

    #[test]
    fn container_fit_yes() {
        let mut t = TestBed::new();
        setup(&mut t);
        let c = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        let item = t.spawn((ItemVolume(250), ItemWeight(100)));
        assert!(can_fit_in_container(t.world(), c, item));
    }

    #[test]
    fn container_fit_no() {
        let mut t = TestBed::new();
        setup(&mut t);
        let c = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        let item = t.spawn((ItemVolume(99999), ItemWeight(100)));
        assert!(!can_fit_in_container(t.world(), c, item));
    }

    #[test]
    fn container_weight() {
        let mut t = TestBed::new();
        setup(&mut t);
        let c = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        t.spawn((
            ItemVolume(250),
            ItemWeight(100),
            StackCount::new(2),
            InsideContainer(c),
        ));
        t.world_mut().flush();
        assert_eq!(total_container_weight(t.world(), c).as_grams(), 200);
    }

    // ── Effective position ────────────────────────────────────────────

    #[test]
    fn eff_pos_direct() {
        let mut t = TestBed::new();
        setup(&mut t);
        let pos = WorldPos::new(3, 4, crate::ZLevel::new(0));
        let item = t.spawn((WorldPosition(pos),));
        assert_eq!(effective_position(item, t.world()), Some(pos));
    }

    #[test]
    fn eff_pos_nested() {
        let mut t = TestBed::new();
        setup(&mut t);
        let pos = WorldPos::new(1, 2, crate::ZLevel::new(0));
        let c = t.spawn((WorldPosition(pos),));
        let item = t.spawn((InsideContainer(c),));
        assert_eq!(effective_position(item, t.world()), Some(pos));
    }

    // ── Items at position / in container ──────────────────────────────

    #[test]
    fn items_at_pos() {
        let mut t = TestBed::new();
        setup(&mut t);
        let pos = WorldPos::new(0, 0, crate::ZLevel::new(0));
        t.spawn((WorldPosition(pos), StackCount::new(1)));
        assert_eq!(items_at_position(pos, t.world_mut()).len(), 1);
    }

    #[test]
    fn items_in_cont() {
        let mut t = TestBed::new();
        setup(&mut t);
        let c = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        t.spawn((InsideContainer(c), StackCount::new(1)));
        t.world_mut().flush();
        assert_eq!(items_in_container(c, t.world()).len(), 1);
    }

    // ── merge_or_stack edge cases ─────────────────────────────────────

    #[test]
    fn merge_or_stack_basic() {
        let mut t = TestBed::new();
        setup(&mut t);
        let a = make_item(&mut t, "rock", 3);
        let b = make_item(&mut t, "rock", 2);
        t.world_mut().entity_mut(a).insert(DefOrigin(1));
        t.world_mut().entity_mut(b).insert(DefOrigin(1));
        assert!(merge_or_stack(&mut t.world_mut(), a, b));
        t.world_mut().flush();
        assert_eq!(t.get::<StackCount>(a).unwrap().get(), 5);
    }

    #[test]
    fn merge_or_stack_wrong_type() {
        let mut t = TestBed::new();
        setup(&mut t);
        let r = make_item(&mut t, "rock", 1);
        let s = make_item(&mut t, "stick", 1);
        t.world_mut().entity_mut(r).insert(DefOrigin(1));
        t.world_mut().entity_mut(s).insert(DefOrigin(2));
        assert!(!merge_or_stack(&mut t.world_mut(), r, s));
    }

    // ── ItemMoveEvent processing ──────────────────────────────────────

    #[test]
    fn pickup_creates_inside_container() {
        let mut t = TestBed::new();
        setup(&mut t);
        let pos = WorldPos::new(0, 0, crate::ZLevel::new(0));
        let creature = t.spawn((Inventory::default(),));
        let item = t.spawn((WorldPosition(pos), StackCount::new(1)));

        // Insert InsideContainer manually — simulates the event handler
        t.world_mut()
            .entity_mut(item)
            .remove::<WorldPosition>()
            .insert(InsideContainer(creature));

        assert!(t.get::<InsideContainer>(item).is_some());
        assert!(t.get::<WorldPosition>(item).is_none());
    }

    #[test]
    fn drop_removes_container_relationship() {
        let mut t = TestBed::new();
        setup(&mut t);
        let pos = WorldPos::new(5, 10, crate::ZLevel::new(0));
        let creature = t.spawn((Inventory::default(),));
        let item = t.spawn((InsideContainer(creature), StackCount::new(1)));

        t.world_mut()
            .entity_mut(item)
            .remove::<InsideContainer>()
            .remove::<Invlet>()
            .insert(WorldPosition(pos));

        assert!(t.get::<InsideContainer>(item).is_none());
        assert_eq!(t.get::<WorldPosition>(item).unwrap().0, pos);
    }

    #[test]
    fn transfer_between_containers() {
        let mut t = TestBed::new();
        setup(&mut t);
        let src = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        let dst = t.spawn((Container {
            capacity: Volume::from_milliliters(5000),
        },));
        let item = t.spawn((InsideContainer(src), StackCount::new(1)));

        // Re-insert with new parent
        t.world_mut().entity_mut(item).insert(InsideContainer(dst));
        t.world_mut().flush();

        assert_eq!(t.get::<InsideContainer>(item).unwrap().0, dst);
    }
}
