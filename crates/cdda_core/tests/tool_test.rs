//! Tool, book, and gun mod tests — component data and utility formulas.

use cdda_core::core::components::def::{BookData, GunModData, ToolData};
use cdda_core::sim::test_utils::TestBed;

// ---------------------------------------------------------------------------
// Reading time formula
// ---------------------------------------------------------------------------

/// Turns to read a book (depends on int, book time, and chapters).
fn reading_time(book_time: u32, intelligence: u8, chapters: u32) -> u32 {
    let int_factor = 12.0 / intelligence.max(1) as f64;
    let time_per_chapter = (book_time as f64 * int_factor).round() as u32;
    if chapters > 0 {
        time_per_chapter * chapters
    } else {
        time_per_chapter * 10 // no chapter system = ~10x
    }
}

/// Learning chance (depends on int vs book's required int).
fn learning_chance(character_intelligence: u8, book_intelligence: u8) -> f64 {
    if character_intelligence >= book_intelligence {
        1.0
    } else {
        character_intelligence as f64 / book_intelligence as f64
    }
}

/// Tool charge usage (minimum 1).
fn charge_usage(charges_per_use: i32) -> i32 {
    charges_per_use.max(1)
}

// ---------------------------------------------------------------------------
// ToolData component tests
// ---------------------------------------------------------------------------

#[test]
fn tool_charges() {
    let mut test = TestBed::new();
    test.register::<ToolData>();

    let e = test.spawn((ToolData {
        max_charges: 100,
        charges_per_use: 1,
        turns_per_charge: 1,
        ammo_type: None,
        revert_to: None,
        power_draw: None,
    },));
    let tool = test.get::<ToolData>(e).unwrap();
    assert_eq!(tool.max_charges, 100);
    assert_eq!(tool.charges_per_use, 1);
    assert_eq!(tool.turns_per_charge, 1);
    assert!(tool.ammo_type.is_none());
    assert!(tool.revert_to.is_none());
    assert!(tool.power_draw.is_none());
}

#[test]
fn tool_revert_to() {
    let mut test = TestBed::new();
    test.register::<ToolData>();

    let e = test.spawn((ToolData {
        max_charges: 1,
        charges_per_use: 1,
        turns_per_charge: 1,
        ammo_type: None,
        revert_to: Some("plastic_bottle_empty".to_string()),
        power_draw: None,
    },));
    let tool = test.get::<ToolData>(e).unwrap();
    assert_eq!(tool.max_charges, 1);
    assert_eq!(tool.revert_to.as_deref(), Some("plastic_bottle_empty"));
}

#[test]
fn tool_ammo_type() {
    let mut test = TestBed::new();
    test.register::<ToolData>();

    let e = test.spawn((ToolData {
        max_charges: 500,
        charges_per_use: 1,
        turns_per_charge: 1,
        ammo_type: Some("battery".to_string()),
        revert_to: None,
        power_draw: Some("2000 W".to_string()),
    },));
    let tool = test.get::<ToolData>(e).unwrap();
    assert_eq!(tool.max_charges, 500);
    assert_eq!(tool.ammo_type.as_deref(), Some("battery"));
    assert_eq!(tool.power_draw.as_deref(), Some("2000 W"));
}

#[test]
fn tool_no_charges() {
    let mut test = TestBed::new();
    test.register::<ToolData>();

    let e = test.spawn((ToolData {
        max_charges: 0,
        charges_per_use: 0,
        turns_per_charge: 0,
        ammo_type: None,
        revert_to: None,
        power_draw: None,
    },));
    let tool = test.get::<ToolData>(e).unwrap();
    assert_eq!(tool.max_charges, 0);
    assert_eq!(tool.charges_per_use, 0);
}

#[test]
fn tool_multiple_use() {
    let mut test = TestBed::new();
    test.register::<ToolData>();

    let e = test.spawn((ToolData {
        max_charges: 100,
        charges_per_use: 10,
        turns_per_charge: 1,
        ammo_type: None,
        revert_to: None,
        power_draw: None,
    },));
    let tool = test.get::<ToolData>(e).unwrap();
    assert_eq!(tool.charges_per_use, 10);
}

#[test]
fn tool_charging_time() {
    let mut test = TestBed::new();
    test.register::<ToolData>();

    let e = test.spawn((ToolData {
        max_charges: 100,
        charges_per_use: 1,
        turns_per_charge: 5,
        ammo_type: None,
        revert_to: None,
        power_draw: None,
    },));
    let tool = test.get::<ToolData>(e).unwrap();
    assert_eq!(tool.turns_per_charge, 5);
}

// ---------------------------------------------------------------------------
// BookData component tests
// ---------------------------------------------------------------------------

#[test]
fn book_skill_teaching() {
    let mut test = TestBed::new();
    test.register::<BookData>();

    let e = test.spawn((BookData {
        skill: "melee".to_string(),
        required_level: 0,
        max_level: 3,
        fun: 1,
        intelligence: 8,
        time: 18000,
        chapters: 0,
    },));
    let book = test.get::<BookData>(e).unwrap();
    assert_eq!(book.skill, "melee");
    assert_eq!(book.required_level, 0);
    assert_eq!(book.max_level, 3);
    assert_eq!(book.fun, 1);
    assert_eq!(book.intelligence, 8);
    assert_eq!(book.time, 18000);
    assert_eq!(book.chapters, 0);
}

#[test]
fn book_high_requirement() {
    let mut test = TestBed::new();
    test.register::<BookData>();

    let e = test.spawn((BookData {
        skill: "electronics".to_string(),
        required_level: 8,
        max_level: 10,
        fun: -1,
        intelligence: 14,
        time: 36000,
        chapters: 3,
    },));
    let book = test.get::<BookData>(e).unwrap();
    assert_eq!(book.required_level, 8);
    assert_eq!(book.max_level, 10);
    assert_eq!(book.intelligence, 14);
}

#[test]
fn book_high_fun() {
    let mut test = TestBed::new();
    test.register::<BookData>();

    let e = test.spawn((BookData {
        skill: "cooking".to_string(),
        required_level: 0,
        max_level: 2,
        fun: 5,
        intelligence: 6,
        time: 12000,
        chapters: 0,
    },));
    let book = test.get::<BookData>(e).unwrap();
    assert_eq!(book.fun, 5);
}

#[test]
fn book_negative_fun() {
    let mut test = TestBed::new();
    test.register::<BookData>();

    let e = test.spawn((BookData {
        skill: "computer_science".to_string(),
        required_level: 4,
        max_level: 6,
        fun: -2,
        intelligence: 12,
        time: 24000,
        chapters: 4,
    },));
    let book = test.get::<BookData>(e).unwrap();
    assert_eq!(book.fun, -2);
}

#[test]
fn book_chapters() {
    let mut test = TestBed::new();
    test.register::<BookData>();

    let e = test.spawn((BookData {
        skill: "fabrication".to_string(),
        required_level: 2,
        max_level: 4,
        fun: 0,
        intelligence: 9,
        time: 15000,
        chapters: 5,
    },));
    let book = test.get::<BookData>(e).unwrap();
    assert_eq!(book.chapters, 5);
}

#[test]
fn book_infinite_chapters() {
    let mut test = TestBed::new();
    test.register::<BookData>();

    let e = test.spawn((BookData {
        skill: "survival".to_string(),
        required_level: 1,
        max_level: 3,
        fun: 2,
        intelligence: 7,
        time: 20000,
        chapters: 0,
    },));
    let book = test.get::<BookData>(e).unwrap();
    assert_eq!(book.chapters, 0);
}

// ---------------------------------------------------------------------------
// GunModData test
// ---------------------------------------------------------------------------

#[test]
fn gun_mod_install_time() {
    let mut test = TestBed::new();
    test.register::<GunModData>();

    let e = test.spawn((GunModData {
        install_time: 30000,
    },));
    let gm = test.get::<GunModData>(e).unwrap();
    assert_eq!(gm.install_time, 30000);
}

// ---------------------------------------------------------------------------
// Formula tests
// ---------------------------------------------------------------------------

#[test]
fn reading_time_test() {
    // book_time=18000, int=8, chapters=0
    // int_factor = 12/8 = 1.5
    // time_per_chapter = 18000 * 1.5 = 27000
    // chapters=0 → 27000 * 10 = 270000

    let book_time: u32 = 18000;
    let intelligence: u8 = 8;
    let chapters: u32 = 0;
    let int_factor = 12.0 / intelligence.max(1) as f64;
    let time_per_chapter = (book_time as f64 * int_factor).round() as u32;
    // Per the user's expected value: 18000 * 12/8 = 27000 (time_per_chapter, not total)
    assert_eq!(time_per_chapter, 27000);

    let total = reading_time(book_time, intelligence, chapters);
    assert_eq!(total, 270000);
}

#[test]
fn learning_chance_sufficient() {
    let chance = learning_chance(10, 8);
    assert!((chance - 1.0).abs() < f64::EPSILON);
}

#[test]
fn charge_usage_test() {
    assert_eq!(charge_usage(1), 1);
    assert_eq!(charge_usage(10), 10);
    assert_eq!(charge_usage(0), 1);
}
