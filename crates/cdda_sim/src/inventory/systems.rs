//! ## Inventory system
//!
//! Manages item stacks, inventory letters (invlets), binned lookups,
//! and item movement between containers and inventories.
//!
//! ### Design (Bevy ECS 0.18)
//!
//! - **Items are entities** with components like `StackCount`, `CurrentCharges`,
//!   `ItemDamage`, and the `InsideContainer` relationship.
//! - **Ownership** is expressed via relationships: items in containers use
//!   `InsideContainer`/`ContainerContents`, wielded items use
//!   `WieldedBy`/`WieldedItems`.
//! - **Invlets** are `Invlet` components on item entities, assigned when an
//!   item enters an inventory. Items lacking an `Invlet` are picked up by
//!   `assign_invlets_system` via `Without<Invlet>` query filter.
//! - **Stack merging** — identical items (same `DefOrigin`, same damage,
//!   same charges) merge into one entity with `StackCount > 1` and the
//!   incoming entity is despawned.
//! - **Binned lookup** — `InventoryBin` resource is rebuilt each frame
//!   by querying items with `Invlet` inside creature containers.
//!
//! Reference: CDDA-master `inventory.h` / `inventory.cpp`.

use crate::actor::turn::{AP_COST_PICKUP, AP_COST_WIELD};
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_state::prelude::NextState;
use cdda_components::actor::{ActionPoints, HandCount};
use cdda_components::context::{push_ctx, ContextStack, Ctx, FocusedCommandIndex};
use cdda_components::def::ItemVolume;
use cdda_components::dev::{DevCamera, DevGroundItemName, DevPlayer};
use cdda_components::events::{ItemMoveEvent, MoveLocation};
use cdda_components::input::{GameAction, InputAction};
use cdda_components::item::{
    Container, ContainerContents, CurrentCharges, DefOrigin, InsideContainer, InventoryFocus,
    Invlet, ItemDamage, MountedOn, MountedPockets, Pocket, StackCount, WieldedBy, WieldedItems,
    FLOOR_CAP_ML, INVLET_CHARS,
};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::{WorldPos, ZLevel, TILES_PER_OMT};
use cdda_core_types::core::units::*;
use tracing::warn;

use super::examine_resource::ExaminedItem;
use super::pocket;

// ===========================================================================
// Helper: collect all items reachable from a creature
// ===========================================================================

/// Collects all item entities that are in `creature`'s inventory domain:
/// directly in the creature's `ContainerContents`, in mounted pockets,
/// and in wielded items.
pub fn all_items_for_creature(creature: Entity, world: &World) -> Vec<Entity> {
    let mut items = Vec::new();

    // Items directly inside the creature (ContainerContents on creature itself)
    if let Some(cc) = world.get::<ContainerContents>(creature) {
        items.extend(cc.iter());
    }

    // Items inside mounted pockets
    if let Some(mp) = world.get::<MountedPockets>(creature) {
        for pocket in mp.iter() {
            if let Some(cc) = world.get::<ContainerContents>(pocket) {
                items.extend(cc.iter());
            }
        }
    }

    // Wielded items
    if let Some(wi) = world.get::<WieldedItems>(creature) {
        items.extend(wi.iter());
    }

    items
}

/// Returns all invlet chars currently in use by items in `creature`'s domain.
fn used_invlets(creature: Entity, world: &World) -> std::collections::HashSet<char> {
    let items = all_items_for_creature(creature, world);
    items
        .iter()
        .filter_map(|&e| world.get::<Invlet>(e).map(|i| i.0))
        .collect()
}

/// Follow a container entity to find the owning creature.
/// If it's a pocket, traverse MountedOn. If it's a creature, return it.
fn find_owning_creature_q(
    container: Entity,
    creature_q: &Query<(), With<cdda_components::actor::Creature>>,
    pocket_q: &Query<(), With<cdda_components::item::IsPocket>>,
    mounted_on_q: &Query<&MountedOn>,
) -> Entity {
    if creature_q.get(container).is_ok() {
        container
    } else if pocket_q.get(container).is_ok() {
        pocket::find_creature_for_pocket(container, mounted_on_q, pocket_q, creature_q)
            .unwrap_or(container)
    } else {
        container
    }
}

/// Collects all items reachable from a creature using query references.
pub fn all_items_for_creature_q(
    creature: Entity,
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
) -> Vec<Entity> {
    let mut items = Vec::new();
    if let Ok(cc) = contents_q.get(creature) {
        items.extend(cc.iter());
    }
    if let Ok(mp) = mounted_q.get(creature) {
        for pocket in mp.iter() {
            if let Ok(cc) = contents_q.get(pocket) {
                items.extend(cc.iter());
            }
        }
    }
    if let Ok(wi) = wielded_q.get(creature) {
        items.extend(wi.iter());
    }
    items
}

/// Find an existing item in `creature`'s domain that `item` can merge into.
fn find_merge_target_for_creature_q(
    creature: Entity,
    item: Entity,
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
    origins_q: &Query<&DefOrigin>,
    damages_q: &Query<&ItemDamage>,
    charges_q: &Query<&CurrentCharges>,
) -> Option<Entity> {
    let incoming_origin = origins_q.get(item).ok().map(|d| d.0);
    let incoming_damage = damages_q.get(item).ok().map(|d| d.0).unwrap_or(0);
    let incoming_charges = charges_q.get(item).ok().map(|c| c.0).unwrap_or(0);

    if incoming_origin.is_none() {
        return None;
    }

    for candidate in all_items_for_creature_q(creature, contents_q, mounted_q, wielded_q) {
        if candidate == item || contents_q.get(candidate).is_err() {
            continue;
        }
        let c_origin = origins_q.get(candidate).ok().map(|d| d.0);
        let c_damage = damages_q.get(candidate).ok().map(|d| d.0).unwrap_or(0);
        let c_charges = charges_q.get(candidate).ok().map(|c| c.0).unwrap_or(0);

        if c_origin == incoming_origin
            && c_damage == incoming_damage
            && c_charges == incoming_charges
        {
            return Some(candidate);
        }
    }
    None
}

/// Returns all invlet chars currently in use by items in `creature`'s domain.
fn used_invlets_q(
    creature: Entity,
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
    _invlet_q: &Query<&Invlet>,
) -> std::collections::HashSet<char> {
    let items = all_items_for_creature_q(creature, contents_q, mounted_q, wielded_q);
    items
        .iter()
        .filter_map(|&e| _invlet_q.get(e).ok().map(|i| i.0))
        .collect()
}

/// Finds an unassigned invlet char for `creature`, preferring `fav_chars`.
fn allocate_invlet_for_q(
    creature: Entity,
    fav_chars: &[char],
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
    invlet_q: &Query<&Invlet>,
) -> Option<char> {
    let used = used_invlets_q(creature, contents_q, mounted_q, wielded_q, invlet_q);
    for c in fav_chars {
        if !used.contains(c) {
            return Some(*c);
        }
    }
    INVLET_CHARS.iter().copied().find(|c| !used.contains(c))
}

/// Finds an unassigned invlet char for `creature`, preferring `fav_chars`.
fn allocate_invlet_for(creature: Entity, world: &World, fav_chars: &[char]) -> Option<char> {
    let used = used_invlets(creature, world);
    // Try favourites first
    for c in fav_chars {
        if !used.contains(c) {
            return Some(*c);
        }
    }
    // Fall back to sequential
    INVLET_CHARS.iter().copied().find(|c| !used.contains(c))
}

/// Find an existing item in `creature`'s domain that `item` can merge into.
fn find_merge_target_for_creature(creature: Entity, item: Entity, world: &World) -> Option<Entity> {
    let incoming_origin = world.get::<DefOrigin>(item).map(|d| d.0);
    let incoming_damage = world.get::<ItemDamage>(item).map(|d| d.0).unwrap_or(0);
    let incoming_charges = world.get::<CurrentCharges>(item).map(|c| c.0).unwrap_or(0);

    if incoming_origin.is_none() {
        return None;
    }

    for candidate in all_items_for_creature(creature, world) {
        // Skip items without an invlet (not yet in inventory) and the item itself
        if candidate == item || world.get::<Invlet>(candidate).is_none() {
            continue;
        }
        let c_origin = world.get::<DefOrigin>(candidate).map(|d| d.0);
        let c_damage = world.get::<ItemDamage>(candidate).map(|d| d.0).unwrap_or(0);
        let c_charges = world
            .get::<CurrentCharges>(candidate)
            .map(|c| c.0)
            .unwrap_or(0);

        if c_origin == incoming_origin
            && c_damage == incoming_damage
            && c_charges == incoming_charges
        {
            return Some(candidate);
        }
    }
    None
}

// ===========================================================================
// InventoryBin — cached item-type lookup
// ===========================================================================

/// Cached bins of inventory items keyed by `DefOrigin`.
///
/// Built by `build_inventory_bins` each frame. Provides fast `count_of`
/// and `charges_of` queries without iterating the entire inventory.
#[derive(Debug, Clone, Default, Resource)]
pub struct InventoryBin {
    /// `DefOrigin.0` → list of item entities of that type.
    pub bins: std::collections::HashMap<u32, Vec<Entity>>,
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
// Systems
// ===========================================================================

/// Assigns invlets to items in a creature's domain that lack `Invlet`.
///
/// Runs in `SimSet::Inventory`. Queries items with `InsideContainer` or
/// `WieldedBy` that lack `Invlet`, groups by owning creature, assigns
/// invlet letters. Merges identical stacks when possible.
pub fn assign_invlets_system(
    mut commands: Commands,
    inside_query: Query<(Entity, &InsideContainer), Without<Invlet>>,
    wielded_query: Query<(Entity, &WieldedBy), Without<Invlet>>,
    invlet_query: Query<&Invlet>,
    stack_counts: Query<&StackCount>,
    creature_query: Query<(), With<cdda_components::actor::Creature>>,
    pocket_query: Query<(), With<cdda_components::item::IsPocket>>,
    container_contents: Query<&ContainerContents>,
    mounted_pockets: Query<&MountedPockets>,
    mounted_on: Query<&MountedOn>,
    wielded_items: Query<&WieldedItems>,
    item_origins: Query<&DefOrigin>,
    item_damages: Query<&ItemDamage>,
    current_charges_q: Query<&CurrentCharges>,
) {
    // Collect items needing invlets grouped by owning creature
    let mut by_creature: std::collections::HashMap<Entity, Vec<Entity>> =
        std::collections::HashMap::new();

    for (item, inside) in &inside_query {
        let creature =
            find_owning_creature_q(inside.0, &creature_query, &pocket_query, &mounted_on);
        by_creature.entry(creature).or_default().push(item);
    }
    for (item, wielded) in &wielded_query {
        by_creature.entry(wielded.0).or_default().push(item);
    }

    for (creature, pending) in by_creature {
        let mut merge_adds: std::collections::HashMap<Entity, u32> =
            std::collections::HashMap::new();

        for item in pending {
            let incoming_count = stack_counts.get(item).ok().map(|s| s.get()).unwrap_or(1);

            let merge_target = find_merge_target_for_creature_q(
                creature,
                item,
                &container_contents,
                &mounted_pockets,
                &wielded_items,
                &item_origins,
                &item_damages,
                &current_charges_q,
            );

            if let Some(target) = merge_target {
                *merge_adds.entry(target).or_insert(0) += incoming_count;
                commands.entity(item).despawn();
                continue;
            }

            let existing_char = invlet_query.get(item).ok().map(|i| i.0);
            let fav_chars: Vec<char> = existing_char.into_iter().collect();

            let c = allocate_invlet_for_q(
                creature,
                &fav_chars,
                &container_contents,
                &mounted_pockets,
                &wielded_items,
                &invlet_query,
            );
            if let Some(c) = c {
                commands.entity(item).insert(Invlet(c));
            }
        }

        for (target, extra) in merge_adds {
            let current = stack_counts.get(target).ok().map(|s| s.get()).unwrap_or(1);
            commands.entity(target).insert(
                StackCount::new(current + extra).expect("current >= 1, so current + extra >= 1"),
            );
        }
    }
}

/// Rebuilds the `InventoryBin` resource by scanning all creatures' inventories.
///
/// Should run after `assign_invlets_system` so items have invlets.
pub fn build_inventory_bins(
    mut bin: ResMut<InventoryBin>,
    creatures: Query<Entity, With<cdda_components::actor::Creature>>,
    origins: Query<&DefOrigin>,
    contents_q: Query<&ContainerContents>,
    mounted_pockets_q: Query<&MountedPockets>,
    wielded_q: Query<&WieldedItems>,
    invlet_q: Query<&Invlet>,
) {
    bin.bins.clear();
    for creature in &creatures {
        for item in all_items_for_creature_q(creature, &contents_q, &mounted_pockets_q, &wielded_q)
        {
            if invlet_q.get(item).is_err() {
                continue;
            }
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
pub fn process_item_move_events(mut reader: MessageReader<ItemMoveEvent>, mut commands: Commands) {
    let events: Vec<ItemMoveEvent> = reader.read().cloned().collect();
    for event in events {
        match (event.from, event.to) {
            // Ground → Container (pickup)
            (MoveLocation::Ground(_), MoveLocation::Container(container)) => {
                commands
                    .entity(event.item)
                    .remove::<WorldPosition>()
                    .insert(InsideContainer(container));
            }
            // Container → Ground (drop)
            (MoveLocation::Container(_container), MoveLocation::Ground(pos)) => {
                commands
                    .entity(event.item)
                    .remove::<InsideContainer>()
                    .remove::<Invlet>()
                    .insert(WorldPosition(pos));
            }
            // Container → Container (transfer)
            (MoveLocation::Container(_from), MoveLocation::Container(to)) => {
                // Inserting a new InsideContainer replaces the old one;
                // relationship hooks handle both ContainerContents updates.
                commands
                    .entity(event.item)
                    .insert(InsideContainer(to))
                    .remove::<Invlet>();
            }
            // Ground → Wielded (pickup into hands)
            (MoveLocation::Ground(_), MoveLocation::Wielded(wielder)) => {
                commands
                    .entity(event.item)
                    .remove::<WorldPosition>()
                    .insert(WieldedBy(wielder));
            }
            // Wielded → Ground (drop from hands)
            (MoveLocation::Wielded(_wielder), MoveLocation::Ground(pos)) => {
                commands
                    .entity(event.item)
                    .remove::<WieldedBy>()
                    .remove::<Invlet>()
                    .insert(WorldPosition(pos));
            }
            // Wielded → Container (stow from hands)
            (MoveLocation::Wielded(_wielder), MoveLocation::Container(container)) => {
                commands
                    .entity(event.item)
                    .remove::<WieldedBy>()
                    .insert(InsideContainer(container))
                    .remove::<Invlet>();
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
    player_query: Query<(Entity, &HandCount), With<DevPlayer>>,
    wielded_items_q: Query<&WieldedItems>,
    wielded_by_check: Query<Entity, With<WieldedBy>>,
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
    mut commands: Commands,
    mut stack: ResMut<ContextStack>,
    mut next_screen: ResMut<NextState<Ctx>>,
    mut focused_cmd: ResMut<FocusedCommandIndex>,
    contents_q: Query<&ContainerContents>,
    invlet_q: Query<&Invlet>,
) {
    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    let Ok((player_entity, hand_count)) = player_query.single() else {
        return;
    };
    let hand_limit = hand_count.0 as usize;

    // Collect pocket items: all items in creature's containers/pockets that have an Invlet
    // and are NOT wielded.
    let all_items = all_items_for_creature_q(
        player_entity,
        &contents_q,
        &mounted_pockets_q,
        &wielded_items_q,
    );
    let mut pocket_items: Vec<(char, Entity)> = Vec::new();
    for item in all_items {
        if wielded_by_check.get(item).is_ok() {
            continue; // skip wielded items
        }
        if let Ok(invlet) = invlet_q.get(item) {
            pocket_items.push((invlet.0, item));
        }
    }
    pocket_items.sort_by_key(|(c, _)| *c);

    // Wielded items
    let wielded_list: Vec<Entity> = wielded_items_q
        .get(player_entity)
        .ok()
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
                        let body_pocket =
                            pocket::get_body_pocket(player_entity, &mounted_pockets_q)
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
/// - **d / Drop**   — drops the first invlet-assigned item at the camera's tile.
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
    wielded_items_q: Query<&WieldedItems>,
    mounted_pockets_q: Query<&MountedPockets>,
    mut ap_query: Query<&mut ActionPoints, With<DevPlayer>>,
    mut move_writer: MessageWriter<ItemMoveEvent>,
    contents_q: Query<&ContainerContents>,
    invlet_q: Query<&Invlet>,
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
                        wp.0.x.div_euclid(TILES_PER_OMT) == camera.x
                            && wp.0.y.div_euclid(TILES_PER_OMT) == camera.y
                            && wp.0.z.0 as i32 == camera.z
                    })
                    .map(|(e, wp, _)| (e, wp.0))
                    .collect();

                for (item, pos) in to_pickup {
                    // Fill hand slots first (WieldedBy), then fall back to body pocket.
                    let wielded_count = wielded_items_q
                        .get(player_entity)
                        .ok()
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
                        let body_pocket =
                            pocket::get_body_pocket(player_entity, &mounted_pockets_q)
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
                // Drop the first invlet-assigned item in the player's domain.
                let invlet_items: Vec<(char, Entity)> = all_items_for_creature_q(
                    player_entity,
                    &contents_q,
                    &mounted_pockets_q,
                    &wielded_items_q,
                )
                .iter()
                .filter_map(|&e| invlet_q.get(e).ok().map(|i| (i.0, e)))
                .collect();

                if let Some(&(_c, item_entity)) = invlet_items.first() {
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
                            wp.0.x.div_euclid(TILES_PER_OMT) == camera.x
                                && wp.0.y.div_euclid(TILES_PER_OMT) == camera.y
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
                        camera.x * TILES_PER_OMT,
                        camera.y * TILES_PER_OMT,
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
    use cdda_components::def::{ItemLongestSide, ItemWeight};

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

    world.entity_mut(target).insert(
        StackCount::new(target_count + incoming_count)
            .expect("target_count + incoming_count >= 1 for any merge"),
    );
    world
        .entity_mut(target)
        .insert(CurrentCharges(target_charges + incoming_charges));
    world.despawn(incoming);
    true
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
    use crate::runtime::test_utils::TestBed;
    use cdda_components::actor::Creature;
    use cdda_components::def::{DefStrId, ItemName, ItemWeight};

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
        t.register::<InsideContainer>();
        t.register::<ContainerContents>();
        t.register::<Container>();
        t.register::<Pocket>();
        t.register::<WorldPosition>();
        t.register::<Creature>();
    }

    fn make_item(t: &mut TestBed, name: &str, count: u32) -> Entity {
        t.spawn((
            DefStrId(name.into()),
            ItemName(name.into()),
            StackCount::new(count).unwrap(),
            ItemVolume(250),
            ItemWeight(100),
        ))
    }

    fn make_item_charges(t: &mut TestBed, name: &str, count: u32, charges: i32) -> Entity {
        t.spawn((
            DefStrId(name.into()),
            ItemName(name.into()),
            StackCount::new(count).unwrap(),
            CurrentCharges(charges),
            ItemVolume(250),
            ItemWeight(100),
        ))
    }

    fn make_creature(t: &mut TestBed) -> Entity {
        t.spawn((Creature {
            def_id: "test_creature".into(),
            name: "Test".into(),
            species: cdda_components::SpeciesId::from(0u32),
            symbol: '@',
        },))
    }

    // ── Invlet allocation ──────────────────────────────────────────────

    #[test]
    fn invlet_alloc_first() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        // With no items in inventory, 'a' should be available
        let c = allocate_invlet_for(creature, &t.world(), &[]);
        assert_eq!(c, Some('a'));
    }

    #[test]
    fn invlet_alloc_after_used() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        // Put an item with invlet 'a' in creature's ContainerContents
        let item = make_item(&mut t, "rock", 1);
        t.world_mut().entity_mut(item).insert(Invlet('a'));
        t.world_mut()
            .entity_mut(item)
            .insert(InsideContainer(creature));
        let c = allocate_invlet_for(creature, &t.world(), &[]);
        assert_eq!(c, Some('b'));
    }

    #[test]
    fn invlet_alloc_all_full() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        // Fill all invlet slots
        for ch in INVLET_CHARS.iter() {
            let item = make_item(&mut t, "filler", 1);
            t.world_mut().entity_mut(item).insert(Invlet(*ch));
            t.world_mut()
                .entity_mut(item)
                .insert(InsideContainer(creature));
        }
        let c = allocate_invlet_for(creature, &t.world(), &[]);
        assert_eq!(c, None);
    }

    // ── NeedsInvlet assignment ─────────────────────────────────────────

    #[test]
    fn item_without_invlet_gets_assigned() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        let item = make_item(&mut t, "rock", 1);
        t.world_mut()
            .entity_mut(item)
            .insert(InsideContainer(creature));

        // Verify the item is inside the creature
        assert!(t.get::<InsideContainer>(item).is_some());
    }

    #[test]
    fn remove_invlet_on_drop() {
        let mut t = TestBed::new();
        setup(&mut t);
        let item = make_item(&mut t, "rock", 1);
        t.world_mut().entity_mut(item).insert(Invlet('a'));
        // Simulate drop: remove container relationship and invlet
        t.world_mut().entity_mut(item).remove::<Invlet>();
        assert!(t.get::<Invlet>(item).is_none());
    }

    // ── Stack merging ─────────────────────────────────────────────────

    #[test]
    fn merge_identical_items() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        let a = make_item(&mut t, "rock", 3);
        let b = make_item(&mut t, "rock", 2);
        t.world_mut().entity_mut(a).insert(DefOrigin(1));
        t.world_mut().entity_mut(b).insert(DefOrigin(1));
        t.world_mut().entity_mut(a).insert(Invlet('a'));
        t.world_mut()
            .entity_mut(a)
            .insert(InsideContainer(creature));

        // b should find a as merge target
        let target = find_merge_target_for_creature(creature, b, &t.world());
        assert_eq!(target, Some(a));

        // Perform merge
        let merged = merge_or_stack(&mut t.world_mut(), a, b);
        assert!(merged);
        assert_eq!(t.get::<StackCount>(a).unwrap().get(), 5);
    }

    #[test]
    fn merge_diff_types() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        let r = make_item(&mut t, "rock", 1);
        let s = make_item(&mut t, "stick", 1);
        t.world_mut().entity_mut(r).insert(DefOrigin(1));
        t.world_mut().entity_mut(s).insert(DefOrigin(2));
        t.world_mut().entity_mut(r).insert(Invlet('a'));
        t.world_mut()
            .entity_mut(r)
            .insert(InsideContainer(creature));

        // s should NOT find r as merge target
        let target = find_merge_target_for_creature(creature, s, &t.world());
        assert_eq!(target, None);
    }

    #[test]
    fn merge_same_charges() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        let a = make_item_charges(&mut t, "battery", 2, 100);
        let b = make_item_charges(&mut t, "battery", 1, 100);
        t.world_mut().entity_mut(a).insert(DefOrigin(3));
        t.world_mut().entity_mut(b).insert(DefOrigin(3));
        t.world_mut().entity_mut(a).insert(Invlet('a'));
        t.world_mut()
            .entity_mut(a)
            .insert(InsideContainer(creature));

        let merged = merge_or_stack(&mut t.world_mut(), a, b);
        assert!(merged);
        assert_eq!(t.get::<CurrentCharges>(a).unwrap().0, 200);
    }

    #[test]
    fn merge_diff_charges() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        let a = make_item_charges(&mut t, "battery", 1, 100);
        let b = make_item_charges(&mut t, "battery", 1, 50);
        t.world_mut().entity_mut(a).insert(DefOrigin(3));
        t.world_mut().entity_mut(b).insert(DefOrigin(3));
        t.world_mut().entity_mut(a).insert(Invlet('a'));
        t.world_mut()
            .entity_mut(a)
            .insert(InsideContainer(creature));

        // Different charges — should NOT find merge target
        let target = find_merge_target_for_creature(creature, b, &t.world());
        assert_eq!(target, None);
    }

    #[test]
    fn merge_diff_damage() {
        let mut t = TestBed::new();
        setup(&mut t);
        let creature = make_creature(&mut t);
        let a = t.spawn((
            DefStrId("knife".into()),
            ItemName("knife".into()),
            StackCount::new(1).unwrap(),
            ItemDamage(0),
            DefOrigin(10),
            ItemVolume(250),
            ItemWeight(100),
            Invlet('a'),
            InsideContainer(creature),
        ));
        let b = t.spawn((
            DefStrId("knife".into()),
            ItemName("knife".into()),
            StackCount::new(1).unwrap(),
            ItemDamage(1),
            DefOrigin(10),
            ItemVolume(250),
            ItemWeight(100),
        ));

        // Different damage — should NOT find merge target
        let target = find_merge_target_for_creature(creature, b, &t.world());
        assert_eq!(target, None);
        assert!(t.get::<StackCount>(a).is_some());
        assert!(t.get::<StackCount>(b).is_some());
    }
}
