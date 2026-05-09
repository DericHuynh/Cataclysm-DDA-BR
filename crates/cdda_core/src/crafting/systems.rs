//! Crafting system — recipe validation, component consumption, and craft execution.

use bevy_ecs::prelude::*;

use crate::actor::turn::AP_COST_CRAFT_TICK;
use crate::core::components::actor::ActionPoints;
use crate::core::components::def::{
    ItemName, RecipeComponents, RecipeQualities, RecipeResult, RecipeResultCount, RecipeTime,
};
use crate::core::components::item::{
    ContainerContents, InProgressCraft, InsideContainer, Inventory, Invlet, ItemQualities,
    ItemTypeId, StackCount,
};
use crate::core::components::sim::WorldPosition;
use crate::data::def_world::DefinitionWorld;
use crate::worldgen::dev::DevPlayer;
use crate::worldgen::spawning_impl::spawn_item_from_def;
use crate::{WorldPos, ZLevel};

// ---------------------------------------------------------------------------
// RecipeIndex — all recipe def entities
// ---------------------------------------------------------------------------

/// All recipe definition entities, built during `build_def_world`.
/// Used by the crafting menu to enumerate available recipes.
#[derive(Resource, Default, Clone)]
pub struct RecipeIndex(pub Vec<Entity>);

// ---------------------------------------------------------------------------
// CategoryIndex — tabbed category navigation for the crafting menu
// ---------------------------------------------------------------------------

/// Maps recipe category → subcategory → recipe entity list.
/// Built in `build_craft_state` by iterating recipe entities and reading
/// `RecipeCategory` / `RecipeSubcategory` / `IsRecipeDef` components.
#[derive(Resource, Default, Debug, Clone)]
pub struct CategoryIndex {
    /// Ordered list of top-level category display names (e.g. "FOOD", "WEAPON").
    pub top_categories: Vec<String>,
    /// (top_category_display_name, subcategory_display_name) → list of recipe entities.
    pub sub_recipes: std::collections::BTreeMap<(String, String), Vec<Entity>>,
    /// Which top-level category is currently selected.
    pub selected_top: usize,
    /// Which subcategory within the selected top category is selected.
    pub selected_sub: usize,
    /// Which zone has keyboard focus: 0=recipe list, 1=category tabs, 2=subcategory tabs.
    pub focus_zone: usize,
}

/// Strip the "CC_" prefix from a category string for display.
pub fn display_category(raw: &str) -> String {
    raw.strip_prefix("CC_")
        .map(|s| s.to_string())
        .unwrap_or_else(|| raw.to_string())
}

/// Given the raw category (e.g. "CC_FOOD") and raw subcategory (e.g. "CSC_FOOD_BREAD"),
/// return a display name for the subcategory ("BREAD").
pub fn display_subcategory(raw_category: &str, raw_subcategory: &str) -> String {
    // Strip "CSC_" prefix, then strip the category short name + "_".
    let cat_short = raw_category.strip_prefix("CC_").unwrap_or(raw_category);
    let without_csc = raw_subcategory
        .strip_prefix("CSC_")
        .unwrap_or(raw_subcategory);
    // Remove the category short name prefix if present ("FOOD_BREAD" → "BREAD")
    without_csc
        .strip_prefix(&format!("{}_", cat_short))
        .map(|s| s.to_string())
        .unwrap_or_else(|| without_csc.to_string())
}

// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

/// Collect all item entities available for crafting:
/// items in the player's inventory (via ContainerContents relationship + Inventory fallback)
/// + items on the ground in the same OMT tile.
pub fn collect_available_items(world: &mut World, player: Entity) -> Vec<Entity> {
    let player_pos = world.get::<WorldPosition>(player).map(|wp| wp.0);

    let mut items: Vec<Entity> = Vec::new();

    // Items in inventory via ContainerContents relationship (primary source)
    if let Some(cc) = world.get::<ContainerContents>(player) {
        items.extend(cc.iter());
    }

    // Fallback: also collect from Inventory component.
    // ContainerContents can be out of sync if commands haven't flushed yet,
    // or if the player entity was spawned without ContainerContents (Bevy's
    // relationship hooks auto-add it, but only after the first insert).
    if let Some(inv) = world.get::<Inventory>(player) {
        for e in inv.item_entities() {
            if !items.contains(&e) {
                items.push(e);
            }
        }
    }

    // Items on the ground within the same 24×24 OMT tile as the player
    if let Some(pos) = player_pos {
        let px = pos.x.div_euclid(24);
        let py = pos.y.div_euclid(24);
        let pz = pos.z;

        let mut q = world.query::<(Entity, &WorldPosition)>();
        let ground: Vec<Entity> = q
            .iter(world)
            .filter(|(e, wp)| {
                *e != player
                    && wp.0.x.div_euclid(24) == px
                    && wp.0.y.div_euclid(24) == py
                    && wp.0.z == pz
            })
            .map(|(e, _)| e)
            .collect();
        items.extend(ground);
    }

    items
}

// ---------------------------------------------------------------------------
// Availability helpers
// ---------------------------------------------------------------------------

/// Sum `StackCount` across all items in `available` whose `ItemTypeId` matches.
pub fn count_available(world: &World, available: &[Entity], type_id: &str) -> u32 {
    available
        .iter()
        .filter_map(|&e| {
            let matches = world
                .get::<ItemTypeId>(e)
                .map(|t| t.0.as_str() == type_id)
                .unwrap_or(false);
            matches.then(|| world.get::<StackCount>(e).map(|s| s.get()).unwrap_or(1))
        })
        .sum()
}

/// Return `true` if any item in `available` has `quality_id` at `>= min_level`.
pub fn has_quality(world: &World, available: &[Entity], quality_id: &str, min_level: u32) -> bool {
    available.iter().any(|&e| {
        world
            .get::<ItemQualities>(e)
            .map(|iq| {
                iq.0.iter()
                    .any(|(qid, lvl)| qid.as_str() == quality_id && *lvl >= min_level as i32)
            })
            .unwrap_or(false)
    })
}

/// Check whether `available` satisfies all requirements of `recipe_entity`.
///
/// Returns `Ok(())` if craftable, `Err(reason)` otherwise.
pub fn check_can_craft(
    world: &World,
    recipe_entity: Entity,
    available: &[Entity],
) -> Result<(), String> {
    // Quality requirements
    if let Some(quals) = world.get::<RecipeQualities>(recipe_entity) {
        for (quality_id, min_level) in &quals.0 {
            if !has_quality(world, available, quality_id, *min_level) {
                return Err(format!("Need quality {} level {}", quality_id, min_level));
            }
        }
    }

    // Component requirements — each slot must be met by at least one alternative
    if let Some(comps) = world.get::<RecipeComponents>(recipe_entity) {
        for slot in &comps.0 {
            let satisfied = slot
                .iter()
                .any(|entry| count_available(world, available, &entry.item_id) >= entry.count);
            if !satisfied {
                let needed: Vec<String> = slot
                    .iter()
                    .map(|e| format!("{} x{}", e.item_id, e.count))
                    .collect();
                return Err(format!("Need: {}", needed.join(" OR ")));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Component consumption
// ---------------------------------------------------------------------------

/// Deduct `needed` items of `type_id` from `available`.
/// Decrements `StackCount`; despawns the entity when the stack reaches zero.
pub fn consume_items(world: &mut World, available: &[Entity], type_id: &str, mut needed: u32) {
    for &e in available {
        if needed == 0 {
            break;
        }
        let matches = world
            .get::<ItemTypeId>(e)
            .map(|t| t.0.as_str() == type_id)
            .unwrap_or(false);
        if !matches {
            continue;
        }
        let stack = world.get::<StackCount>(e).map(|s| s.get()).unwrap_or(1);
        if stack <= needed {
            needed -= stack;
            // Read relationship info before despawning so we can clean up the
            // owner's Inventory.invlets (which is NOT a Bevy relationship and
            // therefore won't be cleaned automatically).
            let invlet_char = world.get::<Invlet>(e).map(|i| i.0);
            let container = world.get::<InsideContainer>(e).map(|ic| ic.0);
            world.despawn(e);
            if let (Some(c), Some(cont)) = (invlet_char, container) {
                if let Some(mut inv) = world.get_mut::<Inventory>(cont) {
                    inv.invlets.remove(&c);
                }
            }
        } else {
            world.entity_mut(e).insert(StackCount::new(stack - needed));
            needed = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Player lookup
// ---------------------------------------------------------------------------

/// Return the entity of the dev-world player, if any.
pub fn find_dev_player(world: &mut World) -> Option<Entity> {
    let mut q = world.query_filtered::<Entity, With<DevPlayer>>();
    q.iter(world).next()
}

// ---------------------------------------------------------------------------
// Craft execution
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// start_craft — begin an in-progress craft
// ---------------------------------------------------------------------------

/// Validate requirements, consume components, and spawn an `InProgressCraft`
/// entity in `player`'s inventory.  The result item is produced later by
/// `continue_crafts` once enough AP has been invested.
///
/// Returns the `InProgressCraft` entity on success.
pub fn start_craft(
    world: &mut World,
    player: Entity,
    recipe_entity: Entity,
) -> Result<Entity, String> {
    let available = collect_available_items(world, player);
    check_can_craft(world, recipe_entity, &available)?;

    // Build consume plan from available components.
    let consume_plan: Vec<(String, u32)> = world
        .get::<RecipeComponents>(recipe_entity)
        .map(|comps| {
            comps
                .0
                .iter()
                .filter_map(|slot| {
                    slot.iter()
                        .find(|entry| {
                            count_available(world, &available, &entry.item_id) >= entry.count
                        })
                        .map(|entry| (entry.item_id.clone(), entry.count))
                })
                .collect()
        })
        .unwrap_or_default();

    for (type_id, count) in &consume_plan {
        consume_items(world, &available, type_id, *count);
    }

    // Gather result metadata.
    let result_id = world
        .get::<RecipeResult>(recipe_entity)
        .map(|r| r.0.clone())
        .ok_or_else(|| "Recipe has no result".to_string())?;

    let result_count = world
        .get::<RecipeResultCount>(recipe_entity)
        .map(|c| c.0)
        .unwrap_or(1);

    // Look up the display name from the definition world.
    let result_name = world
        .get_resource::<DefinitionWorld>()
        .and_then(|dw| dw.entity_by_str(&result_id))
        .and_then(|de| world.get::<ItemName>(de).map(|n| n.0.clone()))
        .unwrap_or_else(|| result_id.clone());

    // RecipeTime is in turns; multiply by 100 for AP (speed=100 baseline).
    let ap_total = world
        .get::<RecipeTime>(recipe_entity)
        .map(|t| (t.0 as i32 * 100).max(100))
        .unwrap_or(100);

    // Spawn the in-progress entity into the player's inventory.
    let craft_entity = world
        .spawn((
            InProgressCraft {
                recipe_entity,
                result_id: result_id.clone(),
                result_name,
                result_count,
                ap_total,
                ap_spent: 0,
            },
            ItemTypeId(format!("craft:in_progress:{result_id}")),
            InsideContainer(player),
        ))
        .id();

    if let Some(mut inv) = world.get_mut::<Inventory>(player) {
        inv.needs_invlet.insert(craft_entity);
    }

    Ok(craft_entity)
}

// ---------------------------------------------------------------------------
// complete_craft — finish an in-progress craft
// ---------------------------------------------------------------------------

/// Despawn `craft_entity`, spawn the result item in `player`'s inventory.
fn complete_craft(world: &mut World, player: Entity, craft_entity: Entity) {
    let (result_id, result_count) = {
        let Some(craft) = world.get::<InProgressCraft>(craft_entity) else {
            return;
        };
        (craft.result_id.clone(), craft.result_count)
    };

    // Remove from inventory and despawn.
    let invlet_char = world.get::<Invlet>(craft_entity).map(|i| i.0);
    if let Some(c) = invlet_char {
        if let Some(mut inv) = world.get_mut::<Inventory>(player) {
            inv.invlets.remove(&c);
        }
    }
    world.despawn(craft_entity);

    // Spawn the result item.
    let player_pos = world
        .get::<WorldPosition>(player)
        .map(|wp| wp.0)
        .unwrap_or_else(|| WorldPos::new(0, 0, ZLevel::new(0)));

    let def_entity = world
        .get_resource::<DefinitionWorld>()
        .and_then(|dw| dw.entity_by_str(&result_id));

    if let Some(def_entity) = def_entity {
        let crafted = spawn_item_from_def(world, def_entity, player_pos, result_count);

        if world.get::<ItemTypeId>(crafted).is_none() {
            world
                .entity_mut(crafted)
                .insert(ItemTypeId(result_id.clone()));
        }

        world
            .entity_mut(crafted)
            .remove::<WorldPosition>()
            .insert(InsideContainer(player));

        if let Some(mut inv) = world.get_mut::<Inventory>(player) {
            inv.needs_invlet.insert(crafted);
        }

        tracing::info!("Craft complete: {}", result_id);
    }
}

// ---------------------------------------------------------------------------
// continue_crafts — tick in-progress crafts each turn
// ---------------------------------------------------------------------------

/// Each game turn, spend the player's available AP on any in-progress crafts
/// held in their inventory.  When a craft's `ap_spent` reaches `ap_total`,
/// the result item is spawned and the in-progress entity is despawned.
///
/// Runs as an exclusive system (needs `&mut World`) so it can call
/// `complete_craft`, which itself needs to mutate the world.
pub fn continue_crafts(world: &mut World) {
    let Some(player) = find_dev_player(world) else {
        return;
    };

    // Collect in-progress crafts that are inside the player's inventory.
    let craft_entities: Vec<Entity> = {
        let mut q = world.query::<(Entity, &InsideContainer)>();
        q.iter(world)
            .filter_map(|(e, ic)| {
                if ic.0 == player && world.get::<InProgressCraft>(e).is_some() {
                    Some(e)
                } else {
                    None
                }
            })
            .collect()
    };

    for craft_e in craft_entities {
        // Check the player has enough AP for one craft tick.
        let current_ap = world
            .get::<ActionPoints>(player)
            .map(|ap| ap.current)
            .unwrap_or(0);
        if current_ap < AP_COST_CRAFT_TICK {
            break;
        }

        // Spend AP.
        if let Some(mut ap) = world.get_mut::<ActionPoints>(player) {
            ap.spend(AP_COST_CRAFT_TICK);
        }

        // Advance the craft.
        let is_done = {
            let Some(mut craft) = world.get_mut::<InProgressCraft>(craft_e) else {
                continue;
            };
            craft.ap_spent += AP_COST_CRAFT_TICK;
            craft.is_complete()
        };

        if is_done {
            complete_craft(world, player, craft_e);
        }
    }
}

// ---------------------------------------------------------------------------
// do_craft — immediate craft (legacy helper, used by tests)
// ---------------------------------------------------------------------------

/// Validate requirements, consume components, and spawn the result item
/// directly into `player`'s inventory (no AP cost / in-progress step).
///
/// Prefer `start_craft` for gameplay; this exists for tests and one-shot
/// dev commands where AP tracking is not needed.
pub fn do_craft(
    world: &mut World,
    player: Entity,
    recipe_entity: Entity,
) -> Result<Entity, String> {
    let available = collect_available_items(world, player);
    check_can_craft(world, recipe_entity, &available)?;

    // Plan: which alternative to consume for each component slot
    let consume_plan: Vec<(String, u32)> = world
        .get::<RecipeComponents>(recipe_entity)
        .map(|comps| {
            comps
                .0
                .iter()
                .filter_map(|slot| {
                    slot.iter()
                        .find(|entry| {
                            count_available(world, &available, &entry.item_id) >= entry.count
                        })
                        .map(|entry| (entry.item_id.clone(), entry.count))
                })
                .collect()
        })
        .unwrap_or_default();

    // Consume components (live reads from world — handles partial stacks correctly)
    for (type_id, count) in &consume_plan {
        consume_items(world, &available, type_id, *count);
    }

    // Resolve result item def
    let result_id = world
        .get::<RecipeResult>(recipe_entity)
        .map(|r| r.0.clone())
        .ok_or_else(|| "Recipe has no result".to_string())?;

    let result_count = world
        .get::<RecipeResultCount>(recipe_entity)
        .map(|c| c.0)
        .unwrap_or(1);

    let def_entity = world
        .get_resource::<DefinitionWorld>()
        .and_then(|dw| dw.entity_by_str(&result_id))
        .ok_or_else(|| format!("Unknown item def: {}", result_id))?;

    let player_pos = world
        .get::<WorldPosition>(player)
        .map(|wp| wp.0)
        .unwrap_or_else(|| WorldPos::new(0, 0, ZLevel::new(0)));

    // Clone def entity into a runtime item
    let crafted = spawn_item_from_def(world, def_entity, player_pos, result_count);

    // Ensure ItemTypeId is present for crafting/quality checks on next open
    if world.get::<ItemTypeId>(crafted).is_none() {
        world.entity_mut(crafted).insert(ItemTypeId(result_id));
    }

    // Move into player inventory (remove ground position, add containment)
    world
        .entity_mut(crafted)
        .remove::<WorldPosition>()
        .insert(InsideContainer(player));

    if let Some(mut inv) = world.get_mut::<Inventory>(player) {
        inv.needs_invlet.insert(crafted);
    }

    Ok(crafted)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::components::def::{DefStrId, RecipeComponentEntry};
    use crate::core::components::item::{CurrentCharges, DefOrigin};
    use crate::sim::test_utils::TestBed;

    fn setup(t: &mut TestBed) {
        t.register::<DefOrigin>();
        t.register::<DefStrId>();
        t.register::<ItemName>();
        t.register::<ItemTypeId>();
        t.register::<StackCount>();
        t.register::<CurrentCharges>();
        t.register::<Invlet>();
        t.register::<Inventory>();
        t.register::<InsideContainer>();
        t.register::<ContainerContents>();
        t.register::<WorldPosition>();
        t.register::<RecipeComponents>();
        t.register::<RecipeQualities>();
        t.register::<RecipeResult>();
        t.register::<RecipeResultCount>();
        t.register::<RecipeTime>();
    }

    /// Helper: spawn an item with ItemTypeId and StackCount in the global world
    /// (no player inventory, just for count_available / check_can_craft tests).
    fn make_item(t: &mut TestBed, type_id: &str, count: u32) -> Entity {
        t.spawn((ItemTypeId(type_id.to_string()), StackCount::new(count)))
    }

    // ── count_available ────────────────────────────────────────────────

    #[test]
    fn count_available_single_item() {
        let mut t = TestBed::new();
        setup(&mut t);

        let item = make_item(&mut t, "string_6", 6);
        let available = vec![item];

        let n = count_available(t.world(), &available, "string_6");
        assert_eq!(n, 6, "single item with StackCount(6) should return 6");
    }

    #[test]
    fn count_available_multiple_items() {
        let mut t = TestBed::new();
        setup(&mut t);

        // 6 separate items, each StackCount(1)
        let items: Vec<Entity> = (0..6).map(|_| make_item(&mut t, "string_6", 1)).collect();

        let n = count_available(t.world(), &items, "string_6");
        assert_eq!(n, 6, "six items each with StackCount(1) should sum to 6");
    }

    #[test]
    fn count_available_mixed_stacks() {
        let mut t = TestBed::new();
        setup(&mut t);

        let items = vec![
            make_item(&mut t, "string_6", 4),
            make_item(&mut t, "string_6", 2),
        ];

        let n = count_available(t.world(), &items, "string_6");
        assert_eq!(n, 6, "4 + 2 should sum to 6");
    }

    #[test]
    fn count_available_wrong_type_not_counted() {
        let mut t = TestBed::new();
        setup(&mut t);

        let items = vec![
            make_item(&mut t, "string_6", 6),
            make_item(&mut t, "rope_6", 1),
        ];

        let n = count_available(t.world(), &items, "string_6");
        assert_eq!(n, 6, "rope_6 should not be counted for string_6");
    }

    #[test]
    fn count_available_empty_when_missing() {
        let mut t = TestBed::new();
        setup(&mut t);

        let n = count_available(t.world(), &[], "string_6");
        assert_eq!(n, 0, "empty list should return 0");
    }

    // ── check_can_craft ───────────────────────────────────────────────

    /// Create a minimal recipe entity.
    fn make_recipe(
        t: &mut TestBed,
        result_id: &str,
        components: Vec<Vec<RecipeComponentEntry>>,
    ) -> Entity {
        let e = t.spawn((
            RecipeResult(result_id.to_string()),
            RecipeResultCount(1),
            RecipeTime(10),
        ));
        if !components.is_empty() {
            t.world_mut()
                .entity_mut(e)
                .insert(RecipeComponents(components));
        }
        e
    }

    #[test]
    fn check_can_craft_enough_ingredients() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Recipe: string_36 requires 6× string_6
        let recipe = make_recipe(
            &mut t,
            "string_36",
            vec![vec![RecipeComponentEntry {
                item_id: "string_6".into(),
                count: 6,
                recovered: false,
            }]],
        );

        // Player inventory: 6× short string (one stack of 6)
        let item = make_item(&mut t, "string_6", 6);
        let available = vec![item];

        let result = check_can_craft(t.world(), recipe, &available);
        assert!(
            result.is_ok(),
            "should be craftable with 6 short strings, got: {:?}",
            result
        );
    }

    #[test]
    fn check_can_craft_not_enough_ingredients() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Recipe: string_36 requires 6× string_6
        let recipe = make_recipe(
            &mut t,
            "string_36",
            vec![vec![RecipeComponentEntry {
                item_id: "string_6".into(),
                count: 6,
                recovered: false,
            }]],
        );

        // Only 5 short strings available
        let available = vec![make_item(&mut t, "string_6", 5)];

        let result = check_can_craft(t.world(), recipe, &available);
        assert!(
            result.is_err(),
            "should NOT be craftable with only 5 short strings"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("string_6"),
            "error should mention the missing component, got: {}",
            err
        );
    }

    #[test]
    fn check_can_craft_single_stacked_item_covers_requirement() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Recipe requiring exactly 6 of string_6
        let recipe = make_recipe(
            &mut t,
            "long_string",
            vec![vec![RecipeComponentEntry {
                item_id: "string_6".into(),
                count: 6,
                recovered: false,
            }]],
        );

        // A single entity with StackCount(6) — simulates the merged-stack
        // case from assign_invlets_system.
        let available = vec![make_item(&mut t, "string_6", 6)];

        let result = check_can_craft(t.world(), recipe, &available);
        assert!(
            result.is_ok(),
            "single entity with StackCount(6) should satisfy 6× requirement, got: {:?}",
            result
        );
    }

    #[test]
    fn check_can_craft_multiple_stacks_sum_to_requirement() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Recipe requiring 6 of string_6
        let recipe = make_recipe(
            &mut t,
            "long_string",
            vec![vec![RecipeComponentEntry {
                item_id: "string_6".into(),
                count: 6,
                recovered: false,
            }]],
        );

        // Two entities with StackCounts 4 and 2 respectively
        let available = vec![
            make_item(&mut t, "string_6", 4),
            make_item(&mut t, "string_6", 2),
        ];

        let result = check_can_craft(t.world(), recipe, &available);
        assert!(
            result.is_ok(),
            "two stacks (4+2) should satisfy 6× requirement, got: {:?}",
            result
        );
    }

    #[test]
    fn check_can_craft_multiple_alternatives() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Recipe with alternatives: 6 string_6 OR 6 cordage_6
        let recipe = make_recipe(
            &mut t,
            "long_string",
            vec![vec![
                RecipeComponentEntry {
                    item_id: "string_6".into(),
                    count: 6,
                    recovered: false,
                },
                RecipeComponentEntry {
                    item_id: "cordage_6".into(),
                    count: 6,
                    recovered: false,
                },
            ]],
        );

        // Player has cordage_6 but not string_6
        let available = vec![make_item(&mut t, "cordage_6", 6)];

        let result = check_can_craft(t.world(), recipe, &available);
        assert!(
            result.is_ok(),
            "should be craftable with alternative cordage_6, got: {:?}",
            result
        );
    }

    #[test]
    fn check_can_craft_no_components_needed() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Recipe with no component requirements
        let recipe = make_recipe(&mut t, "pebble", vec![]);

        let result = check_can_craft(t.world(), recipe, &[]);
        assert!(
            result.is_ok(),
            "recipe with no components should always be craftable"
        );
    }

    // ── collect_available_items with Inventory ─────────────────────────

    #[test]
    fn collect_items_from_inventory_fallback() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Create a player with Inventory but no ContainerContents
        let player = t.spawn((DevPlayer, Inventory::default()));

        // Add an item directly to Inventory (simulating command not yet flushed)
        let item = make_item(&mut t, "string_6", 6);
        {
            let world = t.world_mut();
            let mut inv = world.get_mut::<Inventory>(player).unwrap();
            inv.needs_invlet.insert(item);
        }

        let items = collect_available_items(t.world_mut(), player);
        assert!(
            items.contains(&item),
            "collect_available_items should find items from Inventory fallback"
        );
    }

    #[test]
    fn collect_items_from_container_contents_primary() {
        let mut t = TestBed::new();
        setup(&mut t);

        // Create a player
        let player = t.spawn((DevPlayer, Inventory::default()));

        // Create an item with InsideContainer(player) — relationship hooks
        // auto-add ContainerContents to the player.
        let item = make_item(&mut t, "string_6", 3);
        t.world_mut()
            .entity_mut(item)
            .insert(InsideContainer(player));

        let items = collect_available_items(t.world_mut(), player);
        assert!(
            items.contains(&item),
            "collect_available_items should find items via ContainerContents"
        );
    }
}
