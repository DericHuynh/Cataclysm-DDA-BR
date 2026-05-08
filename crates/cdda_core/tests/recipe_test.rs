//! Tests for `def_components` recipe definition components.
//!
//! These are pure component construction and inspection tests — no systems
//! are executed.  Each test spawns an entity with one or more recipe
//! components, reads them back, and asserts field values.

use cdda_core::core::components::def::*;
use cdda_core::sim::test_utils::TestBed;

// ============================================================================
// Individual component tests
// ============================================================================

#[test]
fn recipe_skill_used() {
    let mut test = TestBed::new();
    test.register::<RecipeSkillUsed>();

    let entity = test.spawn((RecipeSkillUsed("fabrication".into()),));
    let skill = test.get::<RecipeSkillUsed>(entity).unwrap();
    assert_eq!(skill.0, "fabrication");
}

#[test]
fn recipe_difficulty() {
    let mut test = TestBed::new();
    test.register::<RecipeDifficulty>();

    let entity = test.spawn((RecipeDifficulty(5),));
    let diff = test.get::<RecipeDifficulty>(entity).unwrap();
    assert_eq!(diff.0, 5);
}

#[test]
fn recipe_time_base() {
    let mut test = TestBed::new();
    test.register::<RecipeTime>();

    // 60000 turns = 1 minute in CDDA turn units
    let entity = test.spawn((RecipeTime(60000),));
    let time = test.get::<RecipeTime>(entity).unwrap();
    assert_eq!(time.0, 60000);
}

#[test]
fn recipe_autolearn_true() {
    let mut test = TestBed::new();
    test.register::<RecipeAutolearn>();

    let entity = test.spawn((RecipeAutolearn(true),));
    let autolearn = test.get::<RecipeAutolearn>(entity).unwrap();
    assert!(autolearn.0);
}

#[test]
fn recipe_autolearn_false() {
    let mut test = TestBed::new();
    test.register::<RecipeAutolearn>();

    let entity = test.spawn((RecipeAutolearn(false),));
    let autolearn = test.get::<RecipeAutolearn>(entity).unwrap();
    assert!(!autolearn.0);
}

#[test]
fn recipe_result_fields() {
    let mut test = TestBed::new();
    test.register::<RecipeResult>();
    test.register::<RecipeResultCount>();
    test.register::<RecipeResultCharges>();

    let entity = test.spawn((
        RecipeResult("nail".into()),
        RecipeResultCount(1),
        RecipeResultCharges(0),
    ));
    let result = test.get::<RecipeResult>(entity).unwrap();
    assert_eq!(result.0, "nail");
    let count = test.get::<RecipeResultCount>(entity).unwrap();
    assert_eq!(count.0, 1);
    let charges = test.get::<RecipeResultCharges>(entity).unwrap();
    assert_eq!(charges.0, 0);
}

#[test]
fn recipe_components_single_alternative() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();

    let entity = test.spawn((RecipeComponents(vec![vec![
        RecipeComponentEntry {
            item_id: "screw".into(),
            count: 1,
            recovered: true,
        },
        RecipeComponentEntry {
            item_id: "nail".into(),
            count: 2,
            recovered: false,
        },
    ]]),));
    let components = test.get::<RecipeComponents>(entity).unwrap();
    assert_eq!(components.0.len(), 1); // one alternative
    assert_eq!(components.0[0].len(), 2); // two items in that alternative
    assert_eq!(components.0[0][0].item_id, "screw");
    assert_eq!(components.0[0][0].count, 1);
    assert!(components.0[0][0].recovered);
    assert_eq!(components.0[0][1].item_id, "nail");
    assert_eq!(components.0[0][1].count, 2);
    assert!(!components.0[0][1].recovered);
}

#[test]
fn recipe_components_multiple_alternatives() {
    let mut test = TestBed::new();
    test.register::<RecipeComponents>();

    let entity = test.spawn((RecipeComponents(vec![
        vec![RecipeComponentEntry {
            item_id: "steel_lump".into(),
            count: 1,
            recovered: false,
        }],
        vec![RecipeComponentEntry {
            item_id: "pipe".into(),
            count: 1,
            recovered: false,
        }],
    ]),));
    let components = test.get::<RecipeComponents>(entity).unwrap();
    assert_eq!(components.0.len(), 2); // two alternatives
    assert_eq!(components.0[0][0].item_id, "steel_lump");
    assert_eq!(components.0[1][0].item_id, "pipe");
}

#[test]
fn recipe_tools_single() {
    let mut test = TestBed::new();
    test.register::<RecipeTools>();

    let entity = test.spawn((RecipeTools(vec![vec![RecipeToolEntry {
        item_id: "hammer".into(),
        amount: 0, // not consumed
    }]]),));
    let tools = test.get::<RecipeTools>(entity).unwrap();
    assert_eq!(tools.0.len(), 1); // one alternative
    assert_eq!(tools.0[0].len(), 1); // one tool
    assert_eq!(tools.0[0][0].item_id, "hammer");
    assert_eq!(tools.0[0][0].amount, 0);
}

#[test]
fn recipe_qualities_multiple() {
    let mut test = TestBed::new();
    test.register::<RecipeQualities>();

    let entity = test.spawn((RecipeQualities(vec![
        ("HAMMER".into(), 1),
        ("SAW_M".into(), 1),
    ]),));
    let qualities = test.get::<RecipeQualities>(entity).unwrap();
    assert_eq!(qualities.0.len(), 2);
    assert_eq!(qualities.0[0].0, "HAMMER");
    assert_eq!(qualities.0[0].1, 1);
    assert_eq!(qualities.0[1].0, "SAW_M");
    assert_eq!(qualities.0[1].1, 1);
}

#[test]
fn recipe_byproducts() {
    let mut test = TestBed::new();
    test.register::<RecipeByproducts>();

    let entity = test.spawn((RecipeByproducts(vec![
        RecipeByproduct {
            item_id: "scrap".into(),
            count: 1,
        },
        RecipeByproduct {
            item_id: "rag".into(),
            count: 5,
        },
    ]),));
    let byproducts = test.get::<RecipeByproducts>(entity).unwrap();
    assert_eq!(byproducts.0.len(), 2);
    assert_eq!(byproducts.0[0].item_id, "scrap");
    assert_eq!(byproducts.0[0].count, 1);
    assert_eq!(byproducts.0[1].item_id, "rag");
    assert_eq!(byproducts.0[1].count, 5);
}

#[test]
fn recipe_all_types_on_one_entity() {
    let mut test = TestBed::new();
    test.register::<RecipeSkillUsed>();
    test.register::<RecipeDifficulty>();
    test.register::<RecipeTime>();
    test.register::<RecipeAutolearn>();
    test.register::<RecipeResult>();
    test.register::<RecipeComponents>();
    test.register::<RecipeTools>();
    test.register::<RecipeQualities>();
    test.register::<RecipeCategory>();
    test.register::<RecipeFlags>();

    let entity = test.spawn((
        RecipeSkillUsed("fabrication".into()),
        RecipeDifficulty(7),
        RecipeTime(120000),
        RecipeAutolearn(true),
        RecipeResult("metal_plate".into()),
        RecipeComponents(vec![vec![RecipeComponentEntry {
            item_id: "steel_lump".into(),
            count: 2,
            recovered: false,
        }]]),
        RecipeTools(vec![vec![RecipeToolEntry {
            item_id: "hammer".into(),
            amount: 0,
        }]]),
        RecipeQualities(vec![("HAMMER".into(), 3)]),
        RecipeCategory("CC_ARMOR".into()),
        RecipeFlags(vec!["BLIND_EASY".into()]),
    ));

    // Verify every field is accessible
    assert_eq!(
        test.get::<RecipeSkillUsed>(entity).unwrap().0,
        "fabrication"
    );
    assert_eq!(test.get::<RecipeDifficulty>(entity).unwrap().0, 7);
    assert_eq!(test.get::<RecipeTime>(entity).unwrap().0, 120000);
    assert!(test.get::<RecipeAutolearn>(entity).unwrap().0);
    assert_eq!(
        test.get::<RecipeResult>(entity).unwrap().0,
        "metal_plate"
    );
    assert_eq!(test.get::<RecipeComponents>(entity).unwrap().0[0][0].item_id, "steel_lump");
    assert_eq!(test.get::<RecipeComponents>(entity).unwrap().0[0][0].count, 2);
    assert_eq!(test.get::<RecipeTools>(entity).unwrap().0[0][0].item_id, "hammer");
    assert_eq!(
        test.get::<RecipeQualities>(entity).unwrap().0[0].0,
        "HAMMER"
    );
    assert_eq!(test.get::<RecipeQualities>(entity).unwrap().0[0].1, 3);
    assert_eq!(test.get::<RecipeCategory>(entity).unwrap().0, "CC_ARMOR");
    assert_eq!(
        test.get::<RecipeFlags>(entity).unwrap().0[0],
        "BLIND_EASY"
    );
}
