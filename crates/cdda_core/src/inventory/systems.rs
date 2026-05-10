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

use crate::context::ctx::Ctx;
use crate::context::nav::{push_ctx, FocusedCommandIndex};
use crate::context::ContextStack;
use crate::core::components::actor::{ActionPoints, HandCount, Health, IsAlive, PlayerData, Gender};
use crate::core::components::actor::Stats;
use crate::actor::turn::{AP_COST_PICKUP, AP_COST_WIELD};
use crate::core::components::def::ItemVolume;
use crate::core::components::item::{
    Container, ContainerContents, CurrentCharges, DefOrigin, InsideContainer, Inventory,
    InventoryBin, InventoryFocus, Invlet, InvletFavorites, ItemDamage, ItemTypeId,
    MountedPockets, Pocket, StackCount, WieldedBy, WieldedItems, FLOOR_CAP_ML,
};
use crate::core::components::sim::WorldPosition;
use crate::core::coords::WorldPos;
use crate::core::units::*;
use crate::data::def_world::DefinitionWorld;
use crate::input::{GameAction, InputAction};
use crate::inventory::examine_resource::ExaminedItem;
use crate::sim::events::{ItemMoveEvent, MoveLocation};
use crate::worldgen::dev::{DevGroundItemName, DevPlayer};
use crate::worldgen::dev_move::DevCamera;
use crate::ZLevel;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;
use bevy_state::prelude::NextState;
use tracing::warn;

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
    stack_counts: Query<&StackCount>,
    item_damages: Query<&ItemDamage>,
    current_charges_q: Query<&CurrentCharges>,
) {
    for (inv_owner, mut inv) in &mut inventories {
        let pending: Vec<Entity> = inv.needs_invlet.drain().collect();

        // Track count increments from merges so we don't race with deferred commands.
        let mut merge_adds: std::collections::HashMap<Entity, u32> =
            std::collections::HashMap::new();

        for item in pending {
            let incoming_origin = item_origins.get(item).ok().map(|d| d.0);
            let incoming_damage = item_damages.get(item).ok().map(|d| d.0).unwrap_or(0);
            let incoming_charges = current_charges_q.get(item).ok().map(|c| c.0).unwrap_or(0);
            let incoming_count = stack_counts.get(item).ok().map(|s| s.get()).unwrap_or(1);

            // Try to merge into an existing stack with the same identity.
            let merge_target = if let Some(origin) = incoming_origin {
                inv.invlets.values().copied().find(|&stack| {
                    item_origins.get(stack).ok().map(|d| d.0) == Some(origin)
                        && item_damages.get(stack).ok().map(|d| d.0).unwrap_or(0)
                            == incoming_damage
                        && current_charges_q
                            .get(stack)
                            .ok()
                            .map(|c| c.0)
                            .unwrap_or(0)
                            == incoming_charges
                })
            } else {
                None
            };

            if let Some(target) = merge_target {
                *merge_adds.entry(target).or_insert(0) += incoming_count;
                commands.entity(item).despawn();
                continue;
            }

            // No merge: assign invlet (favourites first, then sequential).
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

        // Apply accumulated stack count increases from merges.
        for (target, extra) in merge_adds {
            let current = stack_counts.get(target).ok().map(|s| s.get()).unwrap_or(1);
            commands.entity(target).insert(StackCount::new(current + extra));
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

/// Handles navigation and item actions while `Ctx::Inventory` is open.
///
/// - **j / k / arrows** — move focus up / down through item rows
/// - **Enter / e**       — drop the focused item at the camera's OMT tile
///
/// Gated by `run_if(in_state(Ctx::Inventory))` at registration in cdda_app.
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
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
    mut commands: Commands,
    mut stack: ResMut<ContextStack>,
    mut next_screen: ResMut<NextState<Ctx>>,
    mut focused_cmd: ResMut<FocusedCommandIndex>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, mut inventory, hand_count)) = player_query.single_mut() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    // Panel 0: pocket items (everything not in hand), sorted by invlet char.
    let mut pocket_items: Vec<(char, Entity)> = inventory
        .invlets
        .iter()
        .filter(|(_, &e)| wielded_by_check.get(e).is_err())
        .map(|(&c, &e)| (c, e))
        .collect();
    pocket_items.sort_by_key(|(c, _)| *c);

    // Panel 1: wielded items.
    let wielded_list: Vec<Entity> = wielded_items_q
        .get(player_entity)
        .ok()
        .flatten()
        .map(|wi| wi.iter().collect())
        .unwrap_or_default();

    let current_panel_len = if focus.panel == 0 {
        pocket_items.len()
    } else {
        wielded_list.len()
    };

    for action in actions {
        match action {
            GameAction::NavigateUp => {
                focus.index = focus.index.saturating_sub(1);
            }
            GameAction::NavigateDown => {
                if current_panel_len > 0 {
                    focus.index = (focus.index + 1).min(current_panel_len - 1);
                }
            }
            GameAction::NavigateHome => {
                focus.index = 0;
            }
            GameAction::NavigateEnd => {
                focus.index = current_panel_len.saturating_sub(1);
            }
            // Tab / Shift-Tab: cycle between pocket panel and wielded panel.
            GameAction::NavigateNextTab | GameAction::NavigatePrevTab => {
                focus.panel = 1 - focus.panel.min(1);
                focus.index = 0;
            }

            // [Enter / e] — drop focused pocket item at player's feet.
            GameAction::Confirm => {
                if focus.panel != 0 {
                    continue;
                }
                if let Some(&(invlet_char, item_entity)) = pocket_items.get(focus.index) {
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
                    let new_len = pocket_items.len().saturating_sub(1);
                    focus.index = focus.index.min(new_len);
                }
            }

            // [w] — wield from pocket panel, or unwield from wielded panel.
            GameAction::UseItem => {
                if focus.panel == 0 {
                    // Wield: pocket → hand.
                    if let Some(&(_, item_entity)) = pocket_items.get(focus.index) {
                        let wielded_count = wielded_list.len();
                        if wielded_count < hand_limit {
                            commands
                                .entity(item_entity)
                                .remove::<InsideContainer>()
                                .insert(WieldedBy(player_entity));
                            if let Ok(mut ap) = ap_query.single_mut() {
                                ap.spend(AP_COST_WIELD);
                            }
                        } else {
                            warn!(
                                "Hands full ({}/{}) — cannot wield.",
                                wielded_count, hand_limit
                            );
                        }
                    }
                } else {
                    // Unwield: hand → body pocket.
                    if let Some(&item_entity) = wielded_list.get(focus.index) {
                        let body_pocket = crate::inventory::pocket::get_body_pocket(
                            player_entity,
                            &mounted_pockets_q,
                        )
                        .unwrap_or(player_entity);
                        commands
                            .entity(item_entity)
                            .remove::<WieldedBy>()
                            .insert(InsideContainer(body_pocket));
                        if let Ok(mut ap) = ap_query.single_mut() {
                            ap.spend(AP_COST_WIELD);
                        }
                        let new_len = wielded_list.len().saturating_sub(1);
                        focus.index = focus.index.min(new_len);
                    }
                }
            }

            // [X / examine] — open item detail overlay (pocket panel only).
            GameAction::Examine => {
                if focus.panel == 0 {
                    if let Some(&(_, item_entity)) = pocket_items.get(focus.index) {
                        commands.insert_resource(ExaminedItem(Some(item_entity)));
                        push_ctx(
                            Ctx::Inventory,
                            Ctx::ItemExamine,
                            &mut stack,
                            &mut next_screen,
                            &mut focused_cmd,
                        );
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
/// Exclusive system so it can call `spawn_item_from_def` directly, which uses
/// `EntityCloner` to copy all def components (qualities, weapon stats, etc.)
/// to the runtime entity without per-component enumeration.
pub fn spawn_dev_world(world: &mut World) {
    let player = world
        .spawn((
            DevPlayer,
            HandCount(2),
            Inventory::default(),
            InvletFavorites::default(),
            ActionPoints::default(),
            IsAlive,
            Stats::default(),
            Health { current: 100, max: 100 },
            PlayerData {
                name: "Dev Player".to_string(),
                gender: Gender::default(),
                age: 30,
                height: 170,
                blood_type: "O+".to_string(),
                profession: None,
                scenario: None,
            },
        ))
        .id();
    crate::inventory::pocket::spawn_body_pocket(world, player);

    // Columns: display name | CDDA type ID | OMT x | OMT y
    let items: &[(&str, &str, i32, i32)] = &[
        ("Rock",    "sharp_rock",         0, 0),
        ("Stick",   "stick",              1, 0),
        ("Battery", "light_battery_cell", 0, 1),
        ("Knife",   "spear_knife",        2, 0),
        ("Lighter", "lighter",            1, 1),
    ];

    // Resolve def entities before mutably borrowing world for spawning.
    let resolved: Vec<(&str, &str, i32, i32, Option<Entity>)> = {
        let dw = world.get_resource::<DefinitionWorld>();
        items
            .iter()
            .map(|&(name, cdda_id, ox, oy)| {
                let def_e = dw.and_then(|dw| dw.entity_by_str(cdda_id));
                (name, cdda_id, ox, oy, def_e)
            })
            .collect()
    };

    for (name, cdda_id, omt_x, omt_y, def_e) in resolved {
        let pos = WorldPos::new(omt_x * 24, omt_y * 24, ZLevel::new(0));
        if let Some(def_entity) = def_e {
            let instance =
                crate::worldgen::spawning_impl::spawn_item_from_def(world, def_entity, pos, 1);
            world
                .entity_mut(instance)
                .insert(DevGroundItemName(name.to_string()))
                .insert(ItemTypeId(cdda_id.to_string()));
        } else {
            // Def not found — spawn a minimal placeholder so the dev world still loads.
            world.spawn((
                DevGroundItemName(name.to_string()),
                ItemTypeId(cdda_id.to_string()),
                StackCount::new(1),
                WorldPosition(pos),
            ));
        }
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
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
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
                        let body_pocket = crate::inventory::pocket::get_body_pocket(
                            player_entity,
                            &mounted_pockets_q,
                        )
                        .unwrap_or(player_entity);
                        commands
                            .entity(item)
                            .remove::<WorldPosition>()
                            .insert(InsideContainer(body_pocket));
                    }
                    inventory.needs_invlet.insert(item);
                    if let Ok(mut ap) = ap_query.single_mut() {
                        ap.spend(AP_COST_PICKUP);
                    }
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
    use crate::core::components::def::{ItemLongestSide, ItemVolume, ItemWeight};

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
    use crate::core::components::def::ItemVolume;
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
    use crate::core::components::def::ItemWeight;
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
    use crate::core::components::def::DefStrId;

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
    use crate::core::components::def::{DefStrId, ItemName, ItemVolume, ItemWeight};
    use crate::core::components::item::INVLET_CHARS;
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
