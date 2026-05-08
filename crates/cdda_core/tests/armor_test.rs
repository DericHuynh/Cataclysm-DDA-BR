//! Armour and coverage tests — translated from CDDA's armor balance tests.
//!
//! Tests ArmourData component creation and pure-function formulas for
//! effective coverage, encumbrance, warmth, damage reduction, and
//! environmental protection.

use cdda_core::sim::def_components::{
    ArmourData, ArmourPart, ItemFlagList, ItemInsulation, ItemMaterials, ItemName, ItemPrice,
    ItemStackSize, ItemVolume, ItemWeight,
};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Helper: pure formula functions (as declared in the test plan)
// ---------------------------------------------------------------------------

/// Effective coverage considering multiple overlapping layers.
/// CDDA: each layer's coverage is checked independently.
/// The chance that an attack hits uncovered area = product of (1 - coverage_i/100) for all layers.
fn effective_coverage(layers: &[u8]) -> f64 {
    let mut uncovered = 1.0f64;
    for &cov in layers {
        uncovered *= 1.0 - (cov as f64 / 100.0);
    }
    1.0 - uncovered
}

/// Encumbrance from multiple layers.
fn total_encumbrance(parts: &[i32]) -> i32 {
    parts.iter().sum()
}

/// Average warmth across covered body parts.
fn average_warmth(parts: &[i32]) -> f64 {
    if parts.is_empty() {
        0.0
    } else {
        parts.iter().sum::<i32>() as f64 / parts.len() as f64
    }
}

/// Damage reduction from armor material thickness.
/// Each point of thickness reduces incoming damage by a multiplier.
fn thickness_damage_reduction(thickness: f32, incoming_damage: f32) -> f32 {
    let reduction = (thickness * 0.5).min(0.9); // cap at 90% reduction
    incoming_damage * (1.0 - reduction)
}

/// Environmental protection roll: if env_protection >= hazard_level, full protection.
fn env_protection_roll(env_protection: u32, hazard_level: u32) -> bool {
    env_protection >= hazard_level
}

// ---------------------------------------------------------------------------
// ArmourData component tests
// ---------------------------------------------------------------------------

#[test]
fn armour_single_part() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("cotton_tshirt".to_string()),
        ItemWeight(150),
        ItemVolume(250),
        ItemPrice {
            price: 500,
            price_postapoc: 500,
        },
        ItemInsulation(1),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 90,
                encumbrance: 2,
                warmth: 10,
                material: vec![("cotton".to_string(), 1.0)],
            }],
            material_thickness: 1.0,
            env_protection: [0, 0, 0, 0, 0],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert_eq!(armour.parts.len(), 1);
    assert_eq!(armour.parts[0].body_part, "torso");
    assert_eq!(armour.parts[0].coverage, 90);
    assert_eq!(armour.parts[0].encumbrance, 2);
    assert_eq!(armour.parts[0].warmth, 10);
    assert_eq!(armour.parts[0].material.len(), 1);
    assert_eq!(armour.parts[0].material[0].0, "cotton");
    assert!((armour.parts[0].material[0].1 - 1.0).abs() < f32::EPSILON);
}

#[test]
fn armour_multi_part() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("leather_jacket".to_string()),
        ItemWeight(800),
        ItemVolume(2000),
        ItemPrice {
            price: 8000,
            price_postapoc: 8000,
        },
        ItemInsulation(3),
        ItemStackSize(1),
        ArmourData {
            parts: vec![
                ArmourPart {
                    body_part: "torso".to_string(),
                    coverage: 90,
                    encumbrance: 2,
                    warmth: 20,
                    material: vec![("leather".to_string(), 1.0)],
                },
                ArmourPart {
                    body_part: "arm_l".to_string(),
                    coverage: 60,
                    encumbrance: 1,
                    warmth: 15,
                    material: vec![("leather".to_string(), 1.0)],
                },
            ],
            material_thickness: 1.5,
            env_protection: [0, 0, 0, 0, 0],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert_eq!(armour.parts.len(), 2);
    assert_eq!(armour.parts[0].body_part, "torso");
    assert_eq!(armour.parts[1].body_part, "arm_l");
    assert_eq!(armour.parts[1].coverage, 60);
    assert_eq!(armour.parts[1].encumbrance, 1);
}

#[test]
fn armour_full_coverage() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("full_body_armour".to_string()),
        ItemWeight(5000),
        ItemVolume(8000),
        ItemPrice {
            price: 50000,
            price_postapoc: 50000,
        },
        ItemInsulation(5),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 100,
                encumbrance: 5,
                warmth: 30,
                material: vec![("steel".to_string(), 1.0)],
            }],
            material_thickness: 3.0,
            env_protection: [5, 5, 0, 0, 10],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert_eq!(armour.parts[0].coverage, 100);
}

#[test]
fn armour_no_coverage() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("decorative_ribbon".to_string()),
        ItemWeight(5),
        ItemVolume(5),
        ItemPrice {
            price: 50,
            price_postapoc: 50,
        },
        ItemInsulation(0),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 0,
                encumbrance: 0,
                warmth: 0,
                material: vec![("cotton".to_string(), 1.0)],
            }],
            material_thickness: 0.1,
            env_protection: [0, 0, 0, 0, 0],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert_eq!(armour.parts[0].coverage, 0);
}

#[test]
fn armour_encumbrance() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("heavy_plate_armour".to_string()),
        ItemWeight(12000),
        ItemVolume(25000),
        ItemPrice {
            price: 100000,
            price_postapoc: 100000,
        },
        ItemInsulation(8),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 95,
                encumbrance: 20,
                warmth: 40,
                material: vec![("steel".to_string(), 1.0)],
            }],
            material_thickness: 4.0,
            env_protection: [5, 8, 3, 0, 12],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert_eq!(armour.parts[0].encumbrance, 20);
}

#[test]
fn armour_warmth() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    // Cold-weather gear: high warmth
    let e_cold = test.spawn((
        ItemName("winter_coat".to_string()),
        ItemWeight(1200),
        ItemVolume(3000),
        ItemPrice {
            price: 15000,
            price_postapoc: 15000,
        },
        ItemInsulation(7),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 95,
                encumbrance: 8,
                warmth: 40,
                material: vec![("wool".to_string(), 1.0)],
            }],
            material_thickness: 2.0,
            env_protection: [0, 0, 0, 0, 0],
        },
    ));

    let armour_cold = test.get::<ArmourData>(e_cold).unwrap();
    assert_eq!(armour_cold.parts[0].warmth, 40);

    // Summer gear: zero warmth
    let e_summer = test.spawn((
        ItemName("summer_shirt".to_string()),
        ItemWeight(100),
        ItemVolume(200),
        ItemPrice {
            price: 300,
            price_postapoc: 300,
        },
        ItemInsulation(0),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 70,
                encumbrance: 0,
                warmth: 0,
                material: vec![("cotton".to_string(), 1.0)],
            }],
            material_thickness: 0.5,
            env_protection: [0, 0, 0, 0, 0],
        },
    ));

    let armour_summer = test.get::<ArmourData>(e_summer).unwrap();
    assert_eq!(armour_summer.parts[0].warmth, 0);
}

#[test]
fn armour_material_thickness() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("steel_and_kevlar_vest".to_string()),
        ItemWeight(3000),
        ItemVolume(4000),
        ItemPrice {
            price: 40000,
            price_postapoc: 40000,
        },
        ItemInsulation(4),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 85,
                encumbrance: 10,
                warmth: 25,
                material: vec![("steel".to_string(), 0.5), ("kevlar".to_string(), 0.5)],
            }],
            material_thickness: 2.5,
            env_protection: [2, 1, 0, 0, 5],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert!((armour.material_thickness - 2.5).abs() < f32::EPSILON);
    assert_eq!(armour.parts[0].material.len(), 2);
    assert_eq!(armour.parts[0].material[0].0, "steel");
    assert!((armour.parts[0].material[0].1 - 0.5).abs() < f32::EPSILON);
    assert_eq!(armour.parts[0].material[1].0, "kevlar");
    assert!((armour.parts[0].material[1].1 - 0.5).abs() < f32::EPSILON);
}

#[test]
fn armour_env_protection() {
    let mut test = TestBed::new();
    test.register::<ArmourData>();
    test.register::<ItemName>();
    test.register::<ItemWeight>();
    test.register::<ItemVolume>();
    test.register::<ItemPrice>();
    test.register::<ItemInsulation>();
    test.register::<ItemFlagList>();
    test.register::<ItemMaterials>();
    test.register::<ItemStackSize>();

    let e = test.spawn((
        ItemName("hazmat_suit".to_string()),
        ItemWeight(2000),
        ItemVolume(5000),
        ItemPrice {
            price: 30000,
            price_postapoc: 30000,
        },
        ItemInsulation(6),
        ItemStackSize(1),
        ArmourData {
            parts: vec![ArmourPart {
                body_part: "torso".to_string(),
                coverage: 100,
                encumbrance: 12,
                warmth: 30,
                material: vec![("plastic".to_string(), 1.0)],
            }],
            material_thickness: 1.0,
            env_protection: [5, 3, 0, 0, 8],
        },
    ));

    let armour = test.get::<ArmourData>(e).unwrap();
    assert_eq!(armour.env_protection[0], 5); // acid
    assert_eq!(armour.env_protection[1], 3); // fire
    assert_eq!(armour.env_protection[2], 0); // electrical
    assert_eq!(armour.env_protection[3], 0); // radiation
    assert_eq!(armour.env_protection[4], 8); // all
}

// ---------------------------------------------------------------------------
// Pure function formula tests
// ---------------------------------------------------------------------------

#[test]
fn effective_coverage_single() {
    let result = effective_coverage(&[90]);
    assert!((result - 0.9).abs() < f64::EPSILON);
}

#[test]
fn effective_coverage_two_layers() {
    let result = effective_coverage(&[90, 60]);
    // 1 - (0.1 * 0.4) = 1 - 0.04 = 0.96
    assert!((result - 0.96).abs() < f64::EPSILON);
}

#[test]
fn effective_coverage_all() {
    let result = effective_coverage(&[100, 50]);
    // 1 - (0.0 * 0.5) = 1 - 0.0 = 1.0
    assert!((result - 1.0).abs() < f64::EPSILON);
}

#[test]
fn effective_coverage_no_protection() {
    let result = effective_coverage(&[0]);
    // 1 - (1.0) = 0.0
    assert!((result - 0.0).abs() < f64::EPSILON);
}

#[test]
fn total_encumbrance_single() {
    let result = total_encumbrance(&[2]);
    assert_eq!(result, 2);
}

#[test]
fn total_encumbrance_stacked() {
    let result = total_encumbrance(&[2, 3, 5]);
    assert_eq!(result, 10);
}

#[test]
fn average_warmth_test() {
    let result = average_warmth(&[10, 20, 30]);
    assert!((result - 20.0).abs() < f64::EPSILON);
}

#[test]
fn thickness_damage_reduction_test() {
    // thickness 2 => reduction = min(2*0.5, 0.9) = min(1.0, 0.9) = 0.9
    // damage 100 * (1 - 0.9) = 100 * 0.1 = 10
    let result = thickness_damage_reduction(2.0, 100.0);
    assert!((result - 10.0).abs() < 1e-5);
}

#[test]
fn env_protection_sufficient() {
    assert!(env_protection_roll(8, 5));
}

#[test]
fn env_protection_insufficient() {
    assert!(!env_protection_roll(3, 5));
}
