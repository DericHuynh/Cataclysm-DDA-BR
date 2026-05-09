//! Crafting system — recipe validation, component consumption, and craft execution.

use bevy_ecs::prelude::*;

use crate::core::components::def::{
    RecipeComponents, RecipeQualities, RecipeResult, RecipeResultCount,
};
use crate::core::components::item::{
    ContainerContents, InsideContainer, Inventory, ItemQualities, ItemTypeId, StackCount,
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
/// items in the player's inventory + items on the ground in the same OMT tile.
pub fn collect_available_items(world: &mut World, player: Entity) -> Vec<Entity> {
    let player_pos = world.get::<WorldPosition>(player).map(|wp| wp.0);

    let mut items: Vec<Entity> = Vec::new();

    // Items in inventory via ContainerContents relationship
    if let Some(cc) = world.get::<ContainerContents>(player) {
        items.extend(cc.iter());
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
            world.despawn(e);
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

/// Validate requirements, consume components, and spawn the result item
/// directly into `player`'s inventory.
///
/// Returns the spawned item entity on success.
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
