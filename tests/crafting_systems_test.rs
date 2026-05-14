//! Tests for `cdda_crafting::systems` — recipe validation,
//! component counting, ingredient consumption, and stacked-item crafting.

use bevy_ecs::prelude::*;

use cdda_components::def::{
    DefStrId, ItemName, RecipeComponentEntry, RecipeComponents, RecipeQualities, RecipeResult,
    RecipeResultCount, RecipeTime,
};
use cdda_components::dev::DevPlayer;
use cdda_components::item::{
    ContainerContents, CurrentCharges, DefOrigin, InsideContainer, Invlet, ItemTypeId, StackCount,
    WieldedBy, WieldedItems,
};
use cdda_components::sim::WorldPosition;
use cdda_crafting::systems::{
    check_can_craft, collect_available_items, consume_items, count_available,
};
use cdda_sim::test_utils::TestBed;

fn setup(t: &mut TestBed) {
    t.register::<DefOrigin>();
    t.register::<DefStrId>();
    t.register::<ItemName>();
    t.register::<ItemTypeId>();
    t.register::<StackCount>();
    t.register::<CurrentCharges>();
    t.register::<Invlet>();
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
fn collect_items_from_container_contents_direct() {
    let mut t = TestBed::new();
    setup(&mut t);

    // Create a player
    let player = t.spawn((DevPlayer,));

    // Add an item with InsideContainer(player) — no Inventory needed
    let item = make_item(&mut t, "string_6", 6);
    t.world_mut()
        .entity_mut(item)
        .insert(InsideContainer(player));

    let items = collect_available_items(t.world_mut(), player);
    assert!(
        items.contains(&item),
        "collect_available_items should find items via ContainerContents relationship"
    );
}

#[test]
fn collect_items_from_container_contents_primary() {
    let mut t = TestBed::new();
    setup(&mut t);

    // Create a player
    let player = t.spawn((DevPlayer,));

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

// ── Stacked item crafting (the "can't craft with qty:6 short string" bug) ──

fn setup_with_wield(t: &mut TestBed) {
    setup(t);
    t.register::<WieldedBy>();
    t.register::<WieldedItems>();
}

/// Merged stack (StackCount 6) satisfies a recipe requiring 6.
#[test]
fn craft_with_merged_inventory_stack() {
    let mut t = TestBed::new();
    setup_with_wield(&mut t);

    let player = t.spawn((DevPlayer,));

    // One merged entity of 6 short strings in the player's inventory.
    let stack = make_item(&mut t, "string_6", 6);
    t.world_mut().entity_mut(stack).insert(Invlet('a'));
    t.world_mut()
        .entity_mut(stack)
        .insert(InsideContainer(player));

    let recipe = make_recipe(
        &mut t,
        "string_36",
        vec![vec![RecipeComponentEntry {
            item_id: "string_6".into(),
            count: 6,
            recovered: false,
        }]],
    );

    let available = collect_available_items(t.world_mut(), player);
    let result = check_can_craft(t.world(), recipe, &available);
    assert!(
        result.is_ok(),
        "should craft with merged stack of 6: {:?}",
        result
    );
}

/// When consume_items despawns a wielded item, the entity is removed
/// and all relationships/components (WieldedBy, Invlet) are cleaned up automatically.
#[test]
fn consume_items_despawns_wielded_item() {
    let mut t = TestBed::new();
    setup_with_wield(&mut t);

    let player = t.spawn((DevPlayer,));

    // Item is in hands (WieldedBy).
    let item = make_item(&mut t, "string_6", 6);
    t.world_mut()
        .entity_mut(item)
        .insert((Invlet('a'), WieldedBy(player)));

    let available = vec![item];
    consume_items(t.world_mut(), &available, "string_6", 6);

    // Item must be despawned.
    assert!(
        !t.world().entities().contains(item),
        "consumed item should be despawned"
    );
    // The entity no longer exists, so no stale references can remain.
}

/// After consuming a wielded item, the inventory has no stale entity that
/// would cause count_available to return 0 on the next craft check.
#[test]
fn no_stale_invlet_after_consuming_wielded_item() {
    let mut t = TestBed::new();
    setup_with_wield(&mut t);

    let player = t.spawn((DevPlayer,));

    let item = make_item(&mut t, "string_6", 6);
    t.world_mut()
        .entity_mut(item)
        .insert((Invlet('a'), WieldedBy(player)));

    // Consume all 6.
    let available = vec![item];
    consume_items(t.world_mut(), &available, "string_6", 6);

    // A subsequent collect_available_items must return an empty list
    // since the item was despawned.
    let available_after = collect_available_items(t.world_mut(), player);
    let count = count_available(t.world(), &available_after, "string_6");
    assert_eq!(count, 0, "no string_6 should remain after consuming all");
}
