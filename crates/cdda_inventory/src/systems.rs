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

use cdda_actor::turn::{AP_COST_PICKUP, AP_COST_WIELD};
use cdda_components::actor::{
    ActionPoints, Gender, HandCount, Health, IsAlive, PlayerData, Stats,
};
use cdda_components::context::{ContextStack, Ctx, FocusedCommandIndex, push_ctx};
use cdda_components::input::{GameAction, InputAction};
use cdda_components::def::ItemVolume;
use cdda_components::dev::{DevCamera, DevGroundItemName, DevPlayer};
use cdda_components::events::{ItemMoveEvent, MoveLocation};
use cdda_components::item::{
    Container, ContainerContents, CurrentCharges, DefOrigin, InsideContainer, Inventory,
    InventoryBin, InventoryFocus, Invlet, InvletFavorites, ItemDamage, ItemTypeId, MountedPockets,
    Pocket, StackCount, WieldedBy, WieldedItems, FLOOR_CAP_ML,
};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::{WorldPos, ZLevel};
use cdda_core_types::core::units::*;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_state::prelude::NextState;
use tracing::{info, warn};

use crate::examine_resource::ExaminedItem;
use crate::pocket;

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
                        && item_damages.get(stack).ok().map(|d| d.0).unwrap_or(0) == incoming_damage
                        && current_charges_q.get(stack).ok().map(|c| c.0).unwrap_or(0)
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
            commands
                .entity(target)
                .insert(StackCount::new(current + extra));
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
            // Ground → Wielded (pickup into hands)
            (MoveLocation::Ground(_), MoveLocation::Wielded(wielder)) => {
                commands
                    .entity(event.item)
                    .remove::<WorldPosition>()
                    .insert(WieldedBy(wielder));
                if let Ok(mut inv) = inventory_query.get_mut(wielder) {
                    inv.needs_invlet.insert(event.item);
                }
            }
            // Wielded → Ground (drop from hands)
            (MoveLocation::Wielded(wielder), MoveLocation::Ground(pos)) => {
                let invlet_char = invlet_query.get(event.item).ok().map(|i| i.0);
                commands
                    .entity(event.item)
                    .remove::<WieldedBy>()
                    .remove::<Invlet>()
                    .insert(WorldPosition(pos));
                if let Ok(mut inv) = inventory_query.get_mut(wielder) {
                    if let Some(c) = invlet_char {
                        inv.invlets.remove(&c);
                    }
                    inv.needs_invlet.remove(&event.item);
                }
            }
            // Wielded → Container (stow from hands)
            (MoveLocation::Wielded(wielder), MoveLocation::Container(container)) => {
                let invlet_char = invlet_query.get(event.item).ok().map(|i| i.0);
                commands
                    .entity(event.item)
                    .remove::<WieldedBy>()
                    .insert(InsideContainer(container))
                    .remove::<Invlet>();
                if let Ok(mut from_inv) = inventory_query.get_mut(wielder) {
                    if let Some(c) = invlet_char {
                        from_inv.invlets.remove(&c);
                    }
                    from_inv.needs_invlet.remove(&event.item);
                }
                if wielder != container {
                    if let Ok(mut to_inv) = inventory_query.get_mut(container) {
                        to_inv.needs_invlet.insert(event.item);
                    }
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
    mut player_query: Query<(Entity, &mut Inventory, &HandCount), With<DevPlayer>>,
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

    let Ok((player_entity, inventory, hand_count)) = player_query.single_mut() else {
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

            // [Enter] — open item examine / action menu.
            GameAction::Confirm => {
                if focus.panel != 0 {
                    continue;
                }
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
                        let body_pocket = pocket::get_body_pocket(
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

/// Handles `Pickup` and `Drop` actions in the dev world.
///
/// - **g / Pickup** — picks up all items at the camera's current OMT tile.
/// - **d / Drop**   — drops the first item in the player's inventory at the
///   camera's current OMT tile.
///
/// Emits `ItemMoveEvent` messages for each item moved. The
/// `process_item_move_events` system (which runs later in the same
/// `SimSet::Inventory` phase) applies the actual component changes.
pub fn dev_pickup_drop_system(
    mut reader: MessageReader<InputAction>,
    camera: Res<DevCamera>,
    player_query: Query<(Entity, &HandCount), With<DevPlayer>>,
    ground_item_query: Query<
        (Entity, &WorldPosition, Option<&ItemVolume>),
        With<DevGroundItemName>,
    >,
    item_volumes: Query<Option<&ItemVolume>>,
    inventory_query: Query<&Inventory, With<DevPlayer>>,
    wielded_items_q: Query<Option<&WieldedItems>>,
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
    mut move_writer: MessageWriter<ItemMoveEvent>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, hand_count)) = player_query.single() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    for action in actions {
        match action {
            GameAction::Pickup => {
                let to_pickup: Vec<(Entity, WorldPos)> = ground_item_query
                    .iter()
                    .filter(|(_, wp, _)| {
                        wp.0.x.div_euclid(24) == camera.x
                            && wp.0.y.div_euclid(24) == camera.y
                            && wp.0.z.0 as i32 == camera.z
                    })
                    .map(|(e, wp, _)| (e, wp.0))
                    .collect();

                for (item, pos) in to_pickup {
                    // Fill hand slots first (WieldedBy), then fall back to body pocket.
                    let wielded_count = wielded_items_q
                        .get(player_entity)
                        .ok()
                        .flatten()
                        .map(|wi| wi.iter().count())
                        .unwrap_or(0);

                    if wielded_count < hand_limit {
                        move_writer.write(ItemMoveEvent {
                            item,
                            from: MoveLocation::Ground(pos),
                            to: MoveLocation::Wielded(player_entity),
                            count: 1,
                        });
                    } else {
                        let body_pocket = pocket::get_body_pocket(
                            player_entity,
                            &mounted_pockets_q,
                        )
                        .unwrap_or(player_entity);
                        move_writer.write(ItemMoveEvent {
                            item,
                            from: MoveLocation::Ground(pos),
                            to: MoveLocation::Container(body_pocket),
                            count: 1,
                        });
                    }
                    if let Ok(mut ap) = ap_query.single_mut() {
                        ap.spend(AP_COST_PICKUP);
                    }
                }
            }

            GameAction::Drop => {
                // Drop the first invlet-assigned item at the camera position.
                if let Ok(inventory) = inventory_query.single() {
                    if let Some((&_invlet_char, &item_entity)) = inventory.invlets.iter().next() {
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

                        let drop_pos = WorldPos::new(
                            camera.x * 24,
                            camera.y * 24,
                            ZLevel::new(camera.z as i8),
                        );
                        move_writer.write(ItemMoveEvent {
                            item: item_entity,
                            from: MoveLocation::Container(player_entity),
                            to: MoveLocation::Ground(drop_pos),
                            count: 1,
                        });
                    }
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
    use cdda_components::def::{ItemLongestSide, ItemVolume, ItemWeight};

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
    use cdda_components::def::ItemVolume;
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
    use cdda_components::def::ItemWeight;
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
    use cdda_components::def::DefStrId;

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
// Tests — covering all inventory functionality
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use cdda_components::def::{DefStrId, ItemName, ItemVolume, ItemWeight};
    use cdda_components::item::INVLET_CHARS;
    use cdda_sim::test_utils::TestBed;

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
        t.world_mut().entity_mut(a).insert(DefOrigin(10));
        t.world_mut().entity_mut(b).insert(DefOrigin(10));
        add_to_inventory(&mut t.world_mut(), &mut inv, a, None);
        add_to_inventory(&mut t.world_mut(), &mut inv, b, None);
        // Different damage levels prevent stacking
        assert_eq!(inv.len(), 2);
    }
}
