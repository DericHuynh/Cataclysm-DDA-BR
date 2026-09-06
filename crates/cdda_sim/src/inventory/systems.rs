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
//!   item enters an inventory. `assign_invlets_system` reserves unique letters
//!   for unlettered items without moving or merging them.
//! - **Stack merging** is explicit: compatible colocated stacks consolidate
//!   through `merge_stacks`, preserving native snapshot identity and checking
//!   arithmetic before despawning the incoming entity.
//! - **Binned lookup** — `InventoryBin` resource is rebuilt each frame
//!   by querying items with `Invlet` inside creature containers.
//!
//! Reference: CDDA-master `inventory.h` / `inventory.cpp`.

use bevy_ecs::message::MessageCursor;
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
#[cfg(test)]
use cdda_components::def::ItemVolume;
use cdda_components::events::ItemMoveEvent;
#[cfg(test)]
use cdda_components::item::ItemDamage;
use cdda_components::item::{
    ContainerContents, CurrentCharges, DefOrigin, InsideContainer, Invlet, MountedPockets,
    StackCount, WieldedItems, WornBy, INVLET_CHARS,
};
use cdda_components::sim::WorldPosition;
use cdda_core_types::core::coords::WorldPos;
use cdda_core_types::core::units::*;

// ===========================================================================
// Helper: collect all items reachable from a creature
// ===========================================================================

/// Collects all item entities that are in `creature`'s inventory domain:
/// directly in the creature's `ContainerContents`, in mounted pockets,
/// in wielded items, in worn clothing — **and recursively** inside every
/// nested container and pocket found along the way (a battery inside a
/// flashlight inside a backpack is the creature's item).
pub fn all_items_for_creature(creature: Entity, world: &World) -> Vec<Entity> {
    let mut items = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_inventory_domain(creature, world, &mut items, &mut visited);
    items
}

/// Recursive worker for [`all_items_for_creature`]: expands `holder`'s
/// direct item links (contents, pockets, wielded, worn), then recurses into
/// every item that can itself contain or mount further items. The `visited`
/// set guards against relationship cycles.
fn collect_inventory_domain(
    holder: Entity,
    world: &World,
    items: &mut Vec<Entity>,
    visited: &mut std::collections::HashSet<Entity>,
) {
    // Worn clothing/equipment (`WornOn` lives on the item and points at the
    // wearer; `WornBy` is the inverse collection on the wearer).
    if let Some(worn) = world.get::<WornBy>(holder) {
        for item in worn.iter() {
            push_item_recursively(item, world, items, visited);
        }
    }

    // Items directly inside the holder's container contents.
    if let Some(cc) = world.get::<ContainerContents>(holder) {
        for item in cc.iter() {
            push_item_recursively(item, world, items, visited);
        }
    }

    // Items inside mounted pockets (a pocket entity attached to the holder).
    if let Some(mp) = world.get::<MountedPockets>(holder) {
        for pocket in mp.iter() {
            if let Some(cc) = world.get::<ContainerContents>(pocket) {
                for item in cc.iter() {
                    push_item_recursively(item, world, items, visited);
                }
            }
        }
    }

    // Wielded items.
    if let Some(wi) = world.get::<WieldedItems>(holder) {
        for item in wi.iter() {
            push_item_recursively(item, world, items, visited);
        }
    }
}

/// Record one item and recurse into anything it contains or mounts.
fn push_item_recursively(
    item: Entity,
    world: &World,
    items: &mut Vec<Entity>,
    visited: &mut std::collections::HashSet<Entity>,
) {
    if !visited.insert(item) {
        return; // already collected (cycle or shared reference)
    }
    items.push(item);
    // The item itself may be a container (nested) or carry mounted pockets.
    collect_inventory_domain(item, world, items, visited);
}

/// Returns all invlet chars currently in use by items in `creature`'s domain.
#[cfg(test)]
fn used_invlets(creature: Entity, world: &World) -> std::collections::HashSet<char> {
    let items = all_items_for_creature(creature, world);
    items
        .iter()
        .filter_map(|&e| world.get::<Invlet>(e).map(|i| i.0))
        .collect()
}

/// Collects all items reachable from a creature using query references —
/// the query-based twin of [`all_items_for_creature`] with the same
/// recursive nested-container/pocket traversal and worn-equipment coverage.
pub fn all_items_for_creature_q(
    creature: Entity,
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
    worn_q: &Query<&WornBy>,
) -> Vec<Entity> {
    let mut items = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_inventory_domain_q(
        creature,
        contents_q,
        mounted_q,
        wielded_q,
        worn_q,
        &mut items,
        &mut visited,
    );
    items
}

/// Recursive worker for [`all_items_for_creature_q`].
#[allow(clippy::too_many_arguments)]
fn collect_inventory_domain_q(
    holder: Entity,
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
    worn_q: &Query<&WornBy>,
    items: &mut Vec<Entity>,
    visited: &mut std::collections::HashSet<Entity>,
) {
    if let Ok(worn) = worn_q.get(holder) {
        for item in worn.iter() {
            push_item_recursively_q(
                item, contents_q, mounted_q, wielded_q, worn_q, items, visited,
            );
        }
    }
    if let Ok(cc) = contents_q.get(holder) {
        for item in cc.iter() {
            push_item_recursively_q(
                item, contents_q, mounted_q, wielded_q, worn_q, items, visited,
            );
        }
    }
    if let Ok(mp) = mounted_q.get(holder) {
        for pocket in mp.iter() {
            if let Ok(cc) = contents_q.get(pocket) {
                for item in cc.iter() {
                    push_item_recursively_q(
                        item, contents_q, mounted_q, wielded_q, worn_q, items, visited,
                    );
                }
            }
        }
    }
    if let Ok(wi) = wielded_q.get(holder) {
        for item in wi.iter() {
            push_item_recursively_q(
                item, contents_q, mounted_q, wielded_q, worn_q, items, visited,
            );
        }
    }
}

/// Record one item and recurse into anything it contains or mounts (query form).
fn push_item_recursively_q(
    item: Entity,
    contents_q: &Query<&ContainerContents>,
    mounted_q: &Query<&MountedPockets>,
    wielded_q: &Query<&WieldedItems>,
    worn_q: &Query<&WornBy>,
    items: &mut Vec<Entity>,
    visited: &mut std::collections::HashSet<Entity>,
) {
    if !visited.insert(item) {
        return;
    }
    items.push(item);
    collect_inventory_domain_q(
        item, contents_q, mounted_q, wielded_q, worn_q, items, visited,
    );
}

/// Allocate from the live inventory so earlier assignments are visible.
#[cfg(test)]
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
#[cfg(test)]
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
    creatures: Query<
        Entity,
        Or<(
            With<cdda_components::actor::Creature>,
            With<cdda_components::actor::IsAlive>,
        )>,
    >,
    contents: Query<&ContainerContents>,
    mounted: Query<&MountedPockets>,
    wielded: Query<&WieldedItems>,
    worn: Query<&WornBy>,
    invlets: Query<&Invlet>,
) {
    // Reserve letters in a local set before issuing deferred writes. Assignment
    // owns letters only: no implicit transfers, count changes or despawns.
    let mut creatures: Vec<_> = creatures.iter().collect();
    creatures.sort_by_key(|e| e.to_bits());
    for creature in creatures {
        let mut items = all_items_for_creature_q(creature, &contents, &mounted, &wielded, &worn);
        items.sort_by_key(|e| e.to_bits());
        let mut used: std::collections::HashSet<_> = items
            .iter()
            .filter_map(|&e| invlets.get(e).ok().map(|v| v.0))
            .collect();
        for item in items {
            if invlets.get(item).is_ok() {
                continue;
            }
            if let Some(c) = INVLET_CHARS.iter().copied().find(|c| !used.contains(c)) {
                commands.entity(item).insert(Invlet(c));
                used.insert(c);
            }
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
    worn_q: Query<&WornBy>,
    invlet_q: Query<&Invlet>,
) {
    bin.bins.clear();
    for creature in &creatures {
        for item in all_items_for_creature_q(
            creature,
            &contents_q,
            &mounted_pockets_q,
            &wielded_q,
            &worn_q,
        ) {
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
/// Validates and commits synchronously; later requests observe prior commits.
/// Publishes a terminal ItemMoveResult for every request.
pub fn process_item_move_events(
    world: &mut World,
    mut reader: Local<MessageCursor<ItemMoveEvent>>,
) {
    let events: Vec<_> = reader
        .read(world.resource::<Messages<ItemMoveEvent>>())
        .cloned()
        .collect();
    for request in events {
        let result = super::transfer::apply_legacy_move(world, &request);
        world.write_message(cdda_components::events::ItemMoveResult {
            request,
            accepted: result.is_ok(),
            reason: result.err().map(|e| format!("{e:?}")),
        });
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
    super::capacity::validate_capacity(world, container, item).is_ok()
}

/// Recursive contents load, including stacks and flexible nested pockets.
/// Invalid graphs/overflow saturate display totals; validation returns an error.
pub fn total_container_volume(world: &World, container: Entity) -> Volume {
    Volume(super::capacity::contents_load(world, container).map_or(u64::MAX, |v| v.volume_ml))
}
pub fn total_container_weight(world: &World, container: Entity) -> Weight {
    Weight(super::capacity::contents_load(world, container).map_or(u64::MAX, |v| v.weight_g))
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
/// Both entities must have the same immediate location and compatible identity,
/// dimensions and damage. Stateful containers cannot merge. Legacy charge totals
/// are checked and summed; no relocation or AP charge occurs.
pub fn merge_or_stack(world: &mut World, target: Entity, incoming: Entity) -> bool {
    super::merge::merge_stacks(world, target, incoming)
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
    use cdda_components::item::{Container, Pocket};

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
        t.world_mut()
            .entity_mut(b)
            .insert(InsideContainer(creature));
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

        t.world_mut()
            .entity_mut(b)
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
