//! Food, drink, and drug component tests — covering nutritional values,
//! drug effects, and pure formula helpers.
//!
//! Translated from CDDA's various comestible/drug balance tests.

use cdda_components::def::{DrugData, FoodData};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// FoodData component tests
// ---------------------------------------------------------------------------

#[test]
fn food_data_fields() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 250,
        quench: 0,
        fun: 2,
        healthy: 1,
        stim: 0,
        spoils_in: 28800,
        comestible_type: "FOOD".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.calories, 250);
    assert_eq!(food.quench, 0);
    assert_eq!(food.fun, 2);
    assert_eq!(food.healthy, 1);
    assert_eq!(food.stim, 0);
    assert_eq!(food.spoils_in, 28800);
    assert_eq!(food.comestible_type, "FOOD");
}

#[test]
fn food_drink_type() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 100,
        quench: 40,
        fun: 1,
        healthy: 0,
        stim: 0,
        spoils_in: 0,
        comestible_type: "DRINK".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.comestible_type, "DRINK");
    assert_eq!(food.quench, 40);
}

#[test]
fn food_medicine() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 0,
        quench: 0,
        fun: -1,
        healthy: 2,
        stim: 0,
        spoils_in: 0,
        comestible_type: "MED".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.comestible_type, "MED");
    assert_eq!(food.fun, -1);
    assert_eq!(food.healthy, 2);
}

#[test]
fn food_zero_calories() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 0,
        quench: 0,
        fun: 0,
        healthy: 0,
        stim: 0,
        spoils_in: 0,
        comestible_type: "FOOD".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.calories, 0);
}

#[test]
fn food_negative_fun() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 200,
        quench: 0,
        fun: -5,
        healthy: 0,
        stim: 0,
        spoils_in: 14400,
        comestible_type: "FOOD".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.fun, -5);
}

#[test]
fn food_negative_healthy() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 500,
        quench: 0,
        fun: 3,
        healthy: -3,
        stim: 0,
        spoils_in: 43200,
        comestible_type: "FOOD".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.healthy, -3);
}

#[test]
fn food_long_spoilage() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 100,
        quench: 0,
        fun: 0,
        healthy: 0,
        stim: 0,
        spoils_in: 432000,
        comestible_type: "FOOD".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.spoils_in, 432000);
}

#[test]
fn food_no_spoilage() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 100,
        quench: 0,
        fun: 0,
        healthy: 0,
        stim: 0,
        spoils_in: 0,
        comestible_type: "FOOD".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.spoils_in, 0);
}

#[test]
fn food_high_stim() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 0,
        quench: 0,
        fun: 0,
        healthy: 0,
        stim: 10,
        spoils_in: 0,
        comestible_type: "DRINK".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.stim, 10);
}

#[test]
fn food_high_quench() {
    let mut test = TestBed::new();
    test.register::<FoodData>();
    let e = test.spawn((FoodData {
        calories: 0,
        quench: 80,
        fun: 0,
        healthy: 0,
        stim: 0,
        spoils_in: 0,
        comestible_type: "DRINK".to_string(),
    },));
    let food = test.get::<FoodData>(e).unwrap();
    assert_eq!(food.quench, 80);
}

// ---------------------------------------------------------------------------
// DrugData component tests
// ---------------------------------------------------------------------------

#[test]
fn drug_data_fields() {
    let mut test = TestBed::new();
    test.register::<DrugData>();
    let e = test.spawn((DrugData {
        effects: vec!["hallu".to_string(), "pkill".to_string()],
        duration: 600,
        addiction_potential: 10,
    },));
    let drug = test.get::<DrugData>(e).unwrap();
    assert_eq!(drug.effects, vec!["hallu", "pkill"]);
    assert_eq!(drug.duration, 600);
    assert_eq!(drug.addiction_potential, 10);
}

#[test]
fn drug_no_effects() {
    let mut test = TestBed::new();
    test.register::<DrugData>();
    let e = test.spawn((DrugData {
        effects: vec![],
        duration: 0,
        addiction_potential: 0,
    },));
    let drug = test.get::<DrugData>(e).unwrap();
    assert!(drug.effects.is_empty());
    assert_eq!(drug.duration, 0);
    assert_eq!(drug.addiction_potential, 0);
}

#[test]
fn drug_multiple_effects() {
    let mut test = TestBed::new();
    test.register::<DrugData>();
    let e = test.spawn((DrugData {
        effects: vec![
            "pkill".to_string(),
            "hallu".to_string(),
            "adrenaline".to_string(),
        ],
        duration: 300,
        addiction_potential: 5,
    },));
    let drug = test.get::<DrugData>(e).unwrap();
    assert_eq!(drug.effects.len(), 3);
    assert_eq!(drug.effects[0], "pkill");
    assert_eq!(drug.effects[1], "hallu");
    assert_eq!(drug.effects[2], "adrenaline");
}

#[test]
fn drug_high_addiction() {
    let mut test = TestBed::new();
    test.register::<DrugData>();
    let e = test.spawn((DrugData {
        effects: vec!["pkill".to_string()],
        duration: 1200,
        addiction_potential: 50,
    },));
    let drug = test.get::<DrugData>(e).unwrap();
    assert_eq!(drug.addiction_potential, 50);
}

// ---------------------------------------------------------------------------
// Pure function formula tests
// ---------------------------------------------------------------------------

/// CDDA-style daily calorie requirement.
/// Base is roughly 2000 kcal/day, adjusted by stats/activity.
/// Simplified: "light activity" = 2000, "moderate" = 2500,
/// "heavy" = 3000, "very heavy" = 3500.
fn daily_calorie_needs(activity_level: &str) -> i32 {
    match activity_level {
        "light" => 2000,
        "moderate" => 2500,
        "heavy" => 3000,
        "very_heavy" => 3500,
        _ => 2000,
    }
}

/// Satiety value from calories and volume.
/// CDDA: 1 kJ ≈ 1 satiety, with a secondary volume-based cap.
fn satiety_from_food(calories: i32, volume_ml: u32) -> i32 {
    let from_calories = calories / 4; // rough: 250 cal = ~62 satiety
    let volume_cap = (volume_ml / 2) as i32;
    from_calories.min(volume_cap)
}

/// Quench effectiveness depends on stomach capacity.
/// A full stomach (already high satiety) reduces effective quench.
fn effective_quench(quench: i32, current_satiety: i32, max_satiety: i32) -> i32 {
    let remaining_capacity = (max_satiety - current_satiety).max(0);
    quench.min(remaining_capacity)
}

/// Fun bonus decays based on how recently you ate the same food.
/// This avoids infinite morale stacking from eating the same thing repeatedly.
fn fun_with_diminishing_returns(base_fun: i32, times_eaten_recently: u32) -> i32 {
    if times_eaten_recently == 0 {
        base_fun
    } else {
        (base_fun as f64 / (1.0 + times_eaten_recently as f64 * 0.5)).round() as i32
    }
}

#[test]
fn daily_calorie_needs_test() {
    assert_eq!(daily_calorie_needs("light"), 2000);
    assert_eq!(daily_calorie_needs("moderate"), 2500);
    assert_eq!(daily_calorie_needs("heavy"), 3000);
    assert_eq!(daily_calorie_needs("very_heavy"), 3500);
}

#[test]
fn satiety_from_food_test() {
    // 250 cal, 500 ml → 250/4 = 62 satiety, cap = 250, min = 62
    assert_eq!(satiety_from_food(250, 500), 62);
}

#[test]
fn effective_quench_test() {
    // quench=40, satiety 800 of 1000 → room for 200 → full quench 40
    assert_eq!(effective_quench(40, 800, 1000), 40);
    // quench=40, satiety 980 of 1000 → room for 20 → quench capped at 20
    assert_eq!(effective_quench(40, 980, 1000), 20);
}

#[test]
fn fun_diminishing_returns() {
    // base=10, eaten 0 times → 10
    assert_eq!(fun_with_diminishing_returns(10, 0), 10);
    // eaten 2 times → 10 / (1 + 2 * 0.5) = 10 / 2 = 5
    assert_eq!(fun_with_diminishing_returns(10, 2), 5);
    // eaten 5 times → 10 / (1 + 5 * 0.5) = 10 / 3.5 ≈ 2.857 → rounds to 3
    assert_eq!(fun_with_diminishing_returns(10, 5), 3);
}
