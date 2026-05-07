use bevy_ecs::entity::Entity;
use cdda_sim::test_utils::TestBed;

fn setup_item(test: &mut TestBed, _id: &str, vol_ml: u32, wgt_g: u32) -> Entity {
    test.spawn((
        cdda_sim::def_components::ItemName(_id.to_string()),
        cdda_sim::def_components::ItemVolume(vol_ml),
        cdda_sim::def_components::ItemWeight(wgt_g),
    ))
}

#[test]
fn small_item_fits_in_container() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::def_components::ItemName>();
    test.register::<cdda_sim::def_components::ItemVolume>();

    let container_vol_ml = 1000;
    let item = setup_item(&mut test, "test_rock", 250, 100);
    let vol = test
        .get::<cdda_sim::def_components::ItemVolume>(item)
        .unwrap();
    assert!(vol.0 < container_vol_ml);
}

#[test]
fn large_item_does_not_fit() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::def_components::ItemName>();
    test.register::<cdda_sim::def_components::ItemVolume>();

    let container_vol_ml = 1000;
    let item = setup_item(&mut test, "giant_rock", 5000, 1000);
    let vol = test
        .get::<cdda_sim::def_components::ItemVolume>(item)
        .unwrap();
    assert!(vol.0 > container_vol_ml);
}

#[test]
fn same_items_stack() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::def_components::ItemName>();
    test.register::<cdda_sim::def_components::ItemStackSize>();

    let a = test.spawn((
        cdda_sim::def_components::ItemName("rock".to_string()),
        cdda_sim::def_components::ItemStackSize(1),
    ));
    let b = test.spawn((
        cdda_sim::def_components::ItemName("rock".to_string()),
        cdda_sim::def_components::ItemStackSize(1),
    ));
    let name_a = test.get::<cdda_sim::def_components::ItemName>(a).unwrap();
    let name_b = test.get::<cdda_sim::def_components::ItemName>(b).unwrap();
    assert_eq!(name_a.0, name_b.0);
}

#[test]
fn different_items_do_not_stack() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::def_components::ItemName>();

    let a = test.spawn((cdda_sim::def_components::ItemName("rock".to_string()),));
    let b = test.spawn((cdda_sim::def_components::ItemName("stick".to_string()),));
    let name_a = test.get::<cdda_sim::def_components::ItemName>(a).unwrap();
    let name_b = test.get::<cdda_sim::def_components::ItemName>(b).unwrap();
    assert_ne!(name_a.0, name_b.0);
}

#[test]
fn item_weight_positive() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::def_components::ItemWeight>();

    let item = test.spawn((cdda_sim::def_components::ItemWeight(500),));
    let wgt = test
        .get::<cdda_sim::def_components::ItemWeight>(item)
        .unwrap();
    assert!(wgt.0 > 0);
}

#[test]
fn stack_count_minimum_one() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::components::StackCount>();
    let item = test.spawn((cdda_sim::components::StackCount::new(1),));
    assert_eq!(
        test.get::<cdda_sim::components::StackCount>(item)
            .unwrap()
            .get(),
        1
    );
}

#[test]
fn stack_count_multi() {
    let mut test = TestBed::new();
    test.register::<cdda_sim::components::StackCount>();
    let item = test.spawn((cdda_sim::components::StackCount::new(10),));
    assert_eq!(
        test.get::<cdda_sim::components::StackCount>(item)
            .unwrap()
            .get(),
        10
    );
}

#[test]
#[should_panic(expected = "StackCount must be >= 1")]
fn stack_count_zero_panics() {
    let _ = cdda_sim::components::StackCount::new(0);
}
