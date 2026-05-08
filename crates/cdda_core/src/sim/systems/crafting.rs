//! Crafting system — recipe validation, component consumption, and craft execution.
//!
//! Validates whether a creature can craft a given recipe (skill check,
//! tool check, component check), calculates craft time, and consumes
//! required components from inventory. The actual craft execution
//! (spawning the result item) is handled by the spawning system.

use bevy_ecs::prelude::*;
use crate::{RecipeId, Time};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether a creature can craft a given recipe.
///
/// Validates:
/// - Skill requirements (`SkillSet` level >= recipe required level)
/// - Tool availability (required tools exist in inventory)
/// - Component availability (enough `StackCount` of each component)
///
/// Returns `Ok(())` if the creature can craft, or an error message
/// describing the first unmet requirement.
pub fn can_craft(
    world: &World,
    creature: Entity,
    recipe_id: RecipeId,
) -> Result<(), String> {
    let _ = (world, creature, recipe_id);
    todo!("craft validation: check skills, tools, components via DefRegistry")
}

/// Calculate the time required to craft an item.
///
/// Base time from recipe data, modified by skill level (higher skill
/// = faster) and tool quality (good tools = faster).
///
/// `skill_level` is the relevant skill level for this recipe (0 = untrained).
pub fn calculate_craft_time(
    skill_level: u32,
    has_required_tools: bool,
) -> Time {
    let _ = (skill_level, has_required_tools);
    todo!("craft time formula: base time / (1 + skill/10) with tool bonus")
}

/// Consume the required components for a recipe from the creature's
/// inventory.
///
/// Searches all containers (via `InsideContainer` + `ContainerContents`
/// relationships) owned by the creature, deducts `StackCount` for each
/// required component. Returns `Err` if components are missing (should
/// be checked via `can_craft` first).
pub fn consume_components(
    world: &mut World,
    creature: Entity,
    recipe_id: RecipeId,
) -> Result<(), String> {
    let _ = (world, creature, recipe_id);
    todo!("component consumption: find items in inventory, decrement StackCount, despawn if zero")
}

/// Get all recipes the creature has the skill to craft.
///
/// Filters the global recipe registry by comparing each recipe's
/// skill requirements against the creature's `SkillSet`.
pub fn available_recipes(
    world: &World,
    creature: Entity,
) -> Vec<RecipeId> {
    let _ = (world, creature);
    todo!("filter recipes by skill: iterate DefRegistry.recipes, check SkillSet")
}
