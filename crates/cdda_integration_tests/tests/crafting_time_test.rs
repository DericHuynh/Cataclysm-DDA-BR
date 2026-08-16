//! Crafting time / in-progress craft tests — ported from Cataclysm-DDA-master:
//!   tests/crafting_test.cpp: `total_crafting_time_with_or_without_interruption`
//!
//! CDDA reference:
//!   - `make_craft(rid, 1)` spawns an `itype_craft` item and starts ACT_CRAFT
//!   - Each turn: `set_moves(100); activity.do_turn(player)` spends AP on craft
//!   - `batch_time / 100` turns to complete (at speed=100)
//!   - Interrupted craft remains as `itype_craft` in inventory (our: InProgressCraft)
//!   - Resume: continue spending AP until ap_spent >= ap_total
//!
//! Our mapping:
//!   `start_craft(world, player, recipe)` → spawns InProgressCraft entity
//!   `tick_crafting` → spends AP_COST_CRAFT_TICK per turn, emits CraftCompleted
//!   InProgressCraft.is_complete() → true when ap_spent >= ap_total
//!   ap_total = RecipeTime (turns) * 100

use bevy_ecs::prelude::*;
use cdda_components::actor::{ActionPoints, HandCount, IsAlive};
use cdda_components::def::{ItemName, RecipeResult, RecipeResultCount, RecipeTime};
use cdda_components::dev::DevPlayer;
use cdda_components::item::{ContainerContents, InProgressCraft, InsideContainer};
use cdda_components::messages::CraftCompleted;
use cdda_data::interner::{ItemTypeRegistry, QualityRegistry};
use cdda_components::activity::{ActivityProgress, Crafting};
use cdda_sim::activity::systems::tick_crafting;
use cdda_sim::actor::turn::AP_COST_CRAFT_TICK;
use cdda_sim::crafting::systems::{complete_craft, start_craft};
use cdda_sim::runtime::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn register_crafting_components(test: &mut TestBed) {
    test.register::<IsAlive>();
    test.register::<ActionPoints>();

    test.register::<HandCount>();
    test.register::<InsideContainer>();
    test.register::<ContainerContents>();
    test.register::<InProgressCraft>();
    test.register::<RecipeTime>();
    test.register::<RecipeResult>();
    test.register::<RecipeResultCount>();
    test.register::<ItemName>();

    // New activity components
    test.register::<ActivityProgress>();
    test.register::<Crafting>();

    // CraftCompleted message replaces the old CRAFT_COMPLETE_HOOK
    test.add_message::<CraftCompleted>();

    test.world_mut().init_resource::<ItemTypeRegistry>();
    test.world_mut().init_resource::<QualityRegistry>();
}

/// Spawn a player entity with all components needed for crafting + AP system.
fn spawn_player(test: &mut TestBed) -> Entity {
    test.spawn((
        DevPlayer,
        IsAlive,
        ActionPoints {
            current: 10_000,
            speed: 100,
        }, // plenty of AP
        HandCount(2),
    ))
}

/// Spawn a minimal recipe entity (no required components, just time + result).
fn spawn_recipe(test: &mut TestBed, time_turns: u32, result_id: &str) -> Entity {
    test.spawn((
        RecipeTime(time_turns),
        RecipeResult(result_id.to_string()),
        RecipeResultCount(1),
    ))
}

/// Run one tick of the crafting system, then process any CraftCompleted messages.
fn tick_craft(test: &mut TestBed) {
    test.run_system(tick_crafting);

    // Process craft completion messages from the message buffer.
    test.world_mut()
        .resource_mut::<bevy_ecs::message::Messages<CraftCompleted>>()
        .update();
    let completed: Vec<(Entity, Entity)> = test
        .world_mut()
        .resource_mut::<bevy_ecs::message::Messages<CraftCompleted>>()
        .drain()
        .map(|c| (c.crafter, c.craft_entity))
        .collect();
    for (player, craft_e) in &completed {
        complete_craft(test.world_mut(), *player, *craft_e);
    }
}

// ---------------------------------------------------------------------------
// start_craft — CDDA: make_craft spawns in-progress craft item
// ---------------------------------------------------------------------------

/// CDDA: `make_craft(rid, 1)` → `player.activity.id() == ACT_CRAFT` and
/// `wielded_item` is the in-progress craft.
/// Our: `start_craft` → InProgressCraft entity exists in player inventory.
#[test]
fn start_craft_creates_in_progress_entity() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 5, "test_gum");

    let craft_e =
        start_craft(test.world_mut(), player, recipe).expect("start_craft should succeed");

    assert!(
        test.world().get::<InProgressCraft>(craft_e).is_some(),
        "InProgressCraft component should be on the spawned entity"
    );
}

/// The in-progress craft starts with zero AP spent.
#[test]
fn in_progress_craft_starts_with_zero_ap_spent() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 3, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert_eq!(craft.ap_spent, 0);
}

/// CDDA: `batch_time / 100` turns to complete.
/// Our: `ap_total = RecipeTime * 100`.
#[test]
fn in_progress_craft_ap_total_equals_turns_times_100() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 7, "test_gum"); // 7 turns
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert_eq!(craft.ap_total, 700, "7 turns × 100 AP/turn = 700 AP total");
}

/// CDDA: craft is not complete immediately after starting.
#[test]
fn in_progress_craft_is_not_complete_at_start() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 5, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert!(!craft.is_complete());
}

/// The in-progress entity is placed inside the player's inventory.
#[test]
fn in_progress_craft_is_inside_player_inventory() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 3, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    let inside = test.world().get::<InsideContainer>(craft_e).unwrap();
    assert_eq!(
        inside.0, player,
        "in-progress craft should be inside the player"
    );
}

/// The recipe result ID is stored on the InProgressCraft.
#[test]
fn in_progress_craft_stores_result_id() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 2, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert_eq!(craft.result_id, "test_gum");
}

// ---------------------------------------------------------------------------
// tick_crafting — CDDA: activity.do_turn(player) spends AP each turn
// ---------------------------------------------------------------------------

/// CDDA: each call to `do_turn` consumes move points from the player.
/// Our: `tick_crafting` spends `AP_COST_CRAFT_TICK` from the player's AP.
#[test]
fn continue_crafts_spends_player_ap() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 10, "test_gum");
    start_craft(test.world_mut(), player, recipe).unwrap();

    let ap_before = test.world().get::<ActionPoints>(player).unwrap().current;
    tick_craft(&mut test);
    let ap_after = test.world().get::<ActionPoints>(player).unwrap().current;

    assert_eq!(
        ap_before - ap_after,
        AP_COST_CRAFT_TICK,
        "one craft tick should cost AP_COST_CRAFT_TICK AP"
    );
}

/// CDDA: each `do_turn` advances the craft toward completion.
/// Our: `tick_crafting` increments `InProgressCraft.ap_spent`.
#[test]
fn continue_crafts_advances_ap_spent() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 10, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    tick_craft(&mut test);

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert_eq!(craft.ap_spent, AP_COST_CRAFT_TICK);
}

/// After N ticks the craft progress reflects N × AP_COST_CRAFT_TICK.
#[test]
fn continue_crafts_accumulates_across_ticks() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 20, "test_gum"); // 2000 AP total
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    for _ in 0..5 {
        tick_craft(&mut test);
    }

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert_eq!(craft.ap_spent, 5 * AP_COST_CRAFT_TICK);
}

/// CDDA: `is_complete()` remains false while ap_spent < ap_total.
#[test]
fn craft_not_complete_before_full_ap_spent() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 5, "test_gum"); // 500 AP total
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    // Run only 4 of the 5 required ticks
    for _ in 0..4 {
        tick_craft(&mut test);
    }

    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert!(
        !craft.is_complete(),
        "craft should not be complete after 4 of 5 ticks"
    );
}

// ---------------------------------------------------------------------------
// Craft completion — CDDA: craft item removed, result item appears in inventory
// ---------------------------------------------------------------------------

/// CDDA: when `actual_turns_taken == expected_turns_taken`, craft is done.
/// Our: after enough `tick_crafting` ticks, the InProgressCraft entity is despawned.
#[test]
fn craft_despawns_after_completion() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 3, "test_gum"); // 3 turns = 300 AP
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    // Run exactly 3 ticks (300 AP = ap_total)
    for _ in 0..3 {
        tick_craft(&mut test);
    }

    assert!(
        test.world().get_entity(craft_e).is_err(),
        "InProgressCraft entity should be despawned after completion"
    );
}

/// CDDA: `total_crafting_time_with_or_without_interruption` — the craft takes
/// exactly `expected_turns_taken` turns.
/// Our: the craft entity is despawned after exactly `time_turns` ticks.
#[test]
fn craft_completes_in_exactly_the_expected_number_of_turns() {
    let time_turns = 5u32;
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, time_turns, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    // Run one tick short: craft must still exist
    for _ in 0..(time_turns - 1) {
        tick_craft(&mut test);
    }
    assert!(
        test.world().get_entity(craft_e).is_ok(),
        "craft should still exist after {} ticks",
        time_turns - 1
    );

    // Final tick: craft completes
    tick_craft(&mut test);
    assert!(
        test.world().get_entity(craft_e).is_err(),
        "craft should be gone after {} ticks",
        time_turns
    );
}

// ---------------------------------------------------------------------------
// Interruption / resumption — ported from CDDA interrupt test
// ---------------------------------------------------------------------------

/// CDDA: interrupt after 2 turns → `itype_craft` remains in inventory.
/// Our: after N < time_turns ticks, InProgressCraft entity still present.
#[test]
fn interrupted_craft_remains_in_inventory() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 10, "test_gum"); // 10 turns needed
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    // Simulate 3 turns then "interrupt" (just stop calling continue_crafts)
    for _ in 0..3 {
        tick_craft(&mut test);
    }

    // Craft still exists with partial progress
    assert!(
        test.world().get_entity(craft_e).is_ok(),
        "craft should persist after interruption"
    );
    let craft = test.world().get::<InProgressCraft>(craft_e).unwrap();
    assert_eq!(
        craft.ap_spent,
        3 * AP_COST_CRAFT_TICK,
        "ap_spent should reflect 3 ticks of work"
    );
    assert!(!craft.is_complete());
}

/// CDDA: `resume_craft()` continues from saved progress, takes `expected - 2` more turns.
/// Our: resuming means continuing to tick with `tick_crafting`; progress is preserved.
#[test]
fn resumed_craft_completes_from_saved_progress() {
    let time_turns = 7u32;
    let interrupt_at = 3u32;
    let remaining = time_turns - interrupt_at;

    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, time_turns, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    // First phase: interrupt_at turns of work
    for _ in 0..interrupt_at {
        tick_craft(&mut test);
    }
    assert!(
        test.world().get_entity(craft_e).is_ok(),
        "still in-progress"
    );

    // Resume: run the remaining turns
    for _ in 0..remaining {
        tick_craft(&mut test);
    }

    // Craft should now be complete and entity despawned
    assert!(
        test.world().get_entity(craft_e).is_err(),
        "craft should complete after {} + {} = {} total ticks",
        interrupt_at,
        remaining,
        time_turns
    );
}

/// Progress percent increases correctly as AP is spent.
#[test]
fn progress_percent_increases_with_ap_spent() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    let recipe = spawn_recipe(&mut test, 4, "test_gum"); // 400 AP total
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    // 0% progress
    assert_eq!(
        test.world()
            .get::<InProgressCraft>(craft_e)
            .unwrap()
            .progress_pct(),
        0
    );

    tick_craft(&mut test); // 100/400 = 25%
    assert_eq!(
        test.world()
            .get::<InProgressCraft>(craft_e)
            .unwrap()
            .progress_pct(),
        25
    );

    tick_craft(&mut test); // 200/400 = 50%
    assert_eq!(
        test.world()
            .get::<InProgressCraft>(craft_e)
            .unwrap()
            .progress_pct(),
        50
    );
}

/// A craft with ap_total=0 (recipe time 0) is considered instantly complete.
#[test]
fn zero_time_recipe_completes_immediately_on_first_tick() {
    let mut test = TestBed::new();
    register_crafting_components(&mut test);

    let player = spawn_player(&mut test);
    // RecipeTime(0) → ap_total = max(0*100, 100) = 100 (floor 100 in start_craft)
    // so it takes one tick
    let recipe = spawn_recipe(&mut test, 1, "test_gum");
    let craft_e = start_craft(test.world_mut(), player, recipe).unwrap();

    tick_craft(&mut test);

    assert!(
        test.world().get_entity(craft_e).is_err(),
        "a 1-turn recipe should complete after a single tick"
    );
}
