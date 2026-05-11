//! Crafting inventory tests — ported from Cataclysm-DDA-master:
//!   tests/temp_crafting_inv_test.cpp
//!
//! CDDA `temp_crafting_inventory` maps to our helper functions:
//!   `count_available(world, items, type_id)` → `inv.amount_of(type_id)`
//!   `has_quality(world, items, quality_id, level)` → `inv.has_quality(qual, level)`
//!   `check_can_craft(world, recipe, items)` → recipe requirement satisfaction
//!
//! We test the underlying functions directly rather than through a wrapper type
//! because our system passes item entity slices rather than a heap-allocated inventory.

use bevy_ecs::prelude::*;
use cdda_core::core::components::item::{ItemQualities, ItemTypeId, StackCount};
use cdda_core::crafting::systems::{
    check_can_craft, count_available, has_quality,
};
use cdda_core::core::components::def::{RecipeComponentEntry, RecipeComponents, RecipeQualities};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn spawn_item(test: &mut TestBed, type_id: &str, count: u32) -> Entity {
    let e = test.spawn((
        ItemTypeId(type_id.to_string()),
        StackCount::new(count),
    ));
    e
}

fn spawn_quality_item(test: &mut TestBed, type_id: &str, qualities: Vec<(&str, i32)>) -> Entity {
    test.spawn((
        ItemTypeId(type_id.to_string()),
        StackCount::new(1),
        ItemQualities(qualities.into_iter().map(|(q, l)| (q.to_string(), l)).collect()),
    ))
}

// ---------------------------------------------------------------------------
// count_available — analogue of temp_crafting_inventory::amount_of
// ---------------------------------------------------------------------------

/// CDDA: `inv.size() == 0` — empty inventory has no items.
/// Our: `count_available(world, &[], type_id) == 0`.
#[test]
fn count_available_is_zero_for_empty_inventory() {
    let mut test = TestBed::new();
    let world = test.world_mut();
    let available: Vec<Entity> = vec![];
    assert_eq!(count_available(world, &available, "test_gum"), 0);
}

/// CDDA: after `inv.add_item_ref(gum)`, `inv.amount_of(itype_test_gum) == 1`.
#[test]
fn count_available_finds_single_item() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let item = spawn_item(&mut test, "test_gum", 1);
    let world = test.world_mut();
    assert_eq!(count_available(world, &[item], "test_gum"), 1);
}

/// CDDA: `inv.has_amount(itype_test_gum, 1) == true`.
#[test]
fn count_available_stack_of_3_satisfies_need_for_2() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let item = spawn_item(&mut test, "test_gum", 3);
    let world = test.world_mut();
    assert!(count_available(world, &[item], "test_gum") >= 2);
}

/// Querying for a different type_id returns zero even when inventory is non-empty.
#[test]
fn count_available_wrong_type_returns_zero() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let item = spawn_item(&mut test, "test_gum", 5);
    let world = test.world_mut();
    assert_eq!(count_available(world, &[item], "test_fire_ax"), 0);
}

/// Multiple stacks of the same type are summed.
#[test]
fn count_available_sums_multiple_stacks() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let a = spawn_item(&mut test, "test_gum", 2);
    let b = spawn_item(&mut test, "test_gum", 3);
    let world = test.world_mut();
    assert_eq!(count_available(world, &[a, b], "test_gum"), 5);
}

/// Items without a StackCount default to a stack of 1.
#[test]
fn count_available_item_without_stack_count_defaults_to_1() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let item = test.spawn((ItemTypeId("test_gum".to_string()),));
    let world = test.world_mut();
    assert_eq!(count_available(world, &[item], "test_gum"), 1);
}

// ---------------------------------------------------------------------------
// has_quality — analogue of temp_crafting_inventory::has_quality
// ---------------------------------------------------------------------------

/// CDDA: `inv.has_quality(qual_HAMMER, 1) == true` after adding a halligan bar.
#[test]
fn has_quality_finds_matching_quality_at_required_level() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let item = spawn_quality_item(&mut test, "test_halligan", vec![("HAMMER", 2), ("DIG", 1)]);
    let world = test.world_mut();

    // Level 1 is present (item has HAMMER 2 which is >= 1)
    assert!(has_quality(world, &[item], "HAMMER", 1));
    // Level 2 is present exactly
    assert!(has_quality(world, &[item], "HAMMER", 2));
    // Level 3 is NOT present
    assert!(!has_quality(world, &[item], "HAMMER", 3));
}

/// CDDA: `inv.has_quality(qual_DIG, 1) == true`.
#[test]
fn has_quality_finds_secondary_quality() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let item = spawn_quality_item(&mut test, "test_halligan", vec![("HAMMER", 2), ("DIG", 1)]);
    let world = test.world_mut();
    assert!(has_quality(world, &[item], "DIG", 1));
}

/// CDDA: `inv.has_quality(qual_AXE) == false` when no axe-quality item is present.
#[test]
fn has_quality_returns_false_for_missing_quality() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let item = spawn_quality_item(&mut test, "test_halligan", vec![("HAMMER", 2), ("DIG", 1)]);
    let world = test.world_mut();
    assert!(!has_quality(world, &[item], "AXE", 1));
}

/// CDDA: `inv.has_quality(qual_AXE) == true` after adding a fire axe.
#[test]
fn has_quality_returns_true_after_adding_quality_item() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let halligan = spawn_quality_item(&mut test, "test_halligan", vec![("HAMMER", 2), ("DIG", 1)]);
    let fire_ax = spawn_quality_item(&mut test, "test_fire_ax", vec![("AXE", 3)]);
    let world = test.world_mut();

    assert!(!has_quality(world, &[halligan], "AXE", 1));
    assert!(has_quality(world, &[halligan, fire_ax], "AXE", 1));
}

/// CDDA: `inv.max_quality(qual_PRY) == 4` — our system tests level satisfaction.
#[test]
fn has_quality_respects_level_threshold() {
    let mut test = TestBed::new();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let item = spawn_quality_item(&mut test, "test_halligan", vec![("PRY", 4)]);
    let world = test.world_mut();

    assert!(has_quality(world, &[item], "PRY", 1));
    assert!(has_quality(world, &[item], "PRY", 4));
    assert!(!has_quality(world, &[item], "PRY", 5));
}

// ---------------------------------------------------------------------------
// check_can_craft — analogue of requirement_data::can_make_with_inventory
// ---------------------------------------------------------------------------

fn spawn_recipe_no_requirements(test: &mut TestBed) -> Entity {
    test.spawn(())
}

fn spawn_recipe_with_component(test: &mut TestBed, type_id: &str, count: u32) -> Entity {
    let slot: Vec<RecipeComponentEntry> = vec![RecipeComponentEntry {
        item_id: type_id.to_string(),
        count,
        recovered: false,
    }];
    test.spawn((RecipeComponents(vec![slot]),))
}

fn spawn_recipe_with_quality(test: &mut TestBed, quality_id: &str, min_level: u32) -> Entity {
    test.spawn((RecipeQualities(vec![(quality_id.to_string(), min_level)]),))
}

/// A recipe with no requirements is always craftable.
#[test]
fn check_can_craft_passes_with_no_requirements() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();

    let recipe = spawn_recipe_no_requirements(&mut test);
    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[]).is_ok());
}

/// CDDA: has the required item → crafting is possible.
#[test]
fn check_can_craft_passes_when_item_available() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let recipe = spawn_recipe_with_component(&mut test, "test_gum", 1);
    let item = spawn_item(&mut test, "test_gum", 1);

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[item]).is_ok());
}

/// CDDA: missing the required item → crafting fails.
#[test]
fn check_can_craft_fails_when_item_missing() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let recipe = spawn_recipe_with_component(&mut test, "test_gum", 2);
    let item = spawn_item(&mut test, "test_gum", 1); // only 1, need 2

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[item]).is_err());
}

/// CDDA: insufficient item count → crafting fails.
#[test]
fn check_can_craft_fails_when_count_insufficient() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let recipe = spawn_recipe_with_component(&mut test, "test_gum", 5);
    let item = spawn_item(&mut test, "test_gum", 3);

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[item]).is_err());
}

/// Having sufficient stacked quantity satisfies the requirement.
#[test]
fn check_can_craft_passes_with_stacked_items() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();

    let recipe = spawn_recipe_with_component(&mut test, "test_gum", 3);
    let a = spawn_item(&mut test, "test_gum", 2);
    let b = spawn_item(&mut test, "test_gum", 2);

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[a, b]).is_ok());
}

/// Missing a required quality → crafting fails.
#[test]
fn check_can_craft_fails_when_quality_missing() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let recipe = spawn_recipe_with_quality(&mut test, "HAMMER", 1);
    let item = spawn_quality_item(&mut test, "test_gum", vec![]); // no qualities

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[item]).is_err());
}

/// Present quality at required level → crafting passes.
#[test]
fn check_can_craft_passes_when_quality_met() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let recipe = spawn_recipe_with_quality(&mut test, "HAMMER", 2);
    let tool = spawn_quality_item(&mut test, "test_halligan", vec![("HAMMER", 2)]);

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[tool]).is_ok());
}

/// Quality at insufficient level → crafting fails.
#[test]
fn check_can_craft_fails_when_quality_level_too_low() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();
    test.register::<RecipeQualities>();
    test.register::<ItemTypeId>();
    test.register::<StackCount>();
    test.register::<ItemQualities>();

    let recipe = spawn_recipe_with_quality(&mut test, "HAMMER", 3);
    let tool = spawn_quality_item(&mut test, "test_halligan", vec![("HAMMER", 2)]);

    let world = test.world_mut();
    assert!(check_can_craft(world, recipe, &[tool]).is_err());
}
