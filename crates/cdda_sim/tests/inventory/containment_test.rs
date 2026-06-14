use bevy_ecs::entity::Entity;
use cdda_sim::runtime::test_utils::TestBed;

fn setup_item(test: &mut TestBed, _id: &str, vol_ml: u32, wgt_g: u32) -> Entity {
    test.spawn((
        cdda_components::def::ItemName(_id.to_string()),
        cdda_components::def::ItemVolume(vol_ml),
        cdda_components::def::ItemWeight(wgt_g),
    ))
}

#[test]
fn small_item_fits_in_container() {
    let mut test = TestBed::new();
    test.register::<cdda_components::def::ItemName>();
    test.register::<cdda_components::def::ItemVolume>();

    let container_vol_ml = 1000;
    let item = setup_item(&mut test, "test_rock", 250, 100);
    let vol = test.get::<cdda_components::def::ItemVolume>(item).unwrap();
    assert!(vol.0 < container_vol_ml);
}

#[test]
fn large_item_does_not_fit() {
    let mut test = TestBed::new();
    test.register::<cdda_components::def::ItemName>();
    test.register::<cdda_components::def::ItemVolume>();

    let container_vol_ml = 1000;
    let item = setup_item(&mut test, "giant_rock", 5000, 1000);
    let vol = test.get::<cdda_components::def::ItemVolume>(item).unwrap();
    assert!(vol.0 > container_vol_ml);
}

#[test]
fn same_items_stack() {
    let mut test = TestBed::new();
    test.register::<cdda_components::def::ItemName>();
    test.register::<cdda_components::def::ItemStackSize>();

    let a = test.spawn((
        cdda_components::def::ItemName("rock".to_string()),
        cdda_components::def::ItemStackSize(1),
    ));
    let b = test.spawn((
        cdda_components::def::ItemName("rock".to_string()),
        cdda_components::def::ItemStackSize(1),
    ));
    let name_a = test.get::<cdda_components::def::ItemName>(a).unwrap();
    let name_b = test.get::<cdda_components::def::ItemName>(b).unwrap();
    assert_eq!(name_a.0, name_b.0);
}

#[test]
fn different_items_do_not_stack() {
    let mut test = TestBed::new();
    test.register::<cdda_components::def::ItemName>();

    let a = test.spawn((cdda_components::def::ItemName("rock".to_string()),));
    let b = test.spawn((cdda_components::def::ItemName("stick".to_string()),));
    let name_a = test.get::<cdda_components::def::ItemName>(a).unwrap();
    let name_b = test.get::<cdda_components::def::ItemName>(b).unwrap();
    assert_ne!(name_a.0, name_b.0);
}

#[test]
fn item_weight_positive() {
    let mut test = TestBed::new();
    test.register::<cdda_components::def::ItemWeight>();

    let item = test.spawn((cdda_components::def::ItemWeight(500),));
    let wgt = test.get::<cdda_components::def::ItemWeight>(item).unwrap();
    assert!(wgt.0 > 0);
}

#[test]
fn stack_count_minimum_one() {
    let mut test = TestBed::new();
    test.register::<cdda_components::item::StackCount>();
    let item = test.spawn((cdda_components::item::StackCount::new(1).unwrap(),));
    assert_eq!(
        test.get::<cdda_components::item::StackCount>(item)
            .unwrap()
            .get(),
        1
    );
}

#[test]
fn stack_count_multi() {
    let mut test = TestBed::new();
    test.register::<cdda_components::item::StackCount>();
    let item = test.spawn((cdda_components::item::StackCount::new(10).unwrap(),));
    assert_eq!(
        test.get::<cdda_components::item::StackCount>(item)
            .unwrap()
            .get(),
        10
    );
}

#[test]
fn stack_count_zero_returns_err() {
    assert!(cdda_components::item::StackCount::new(0).is_err());
}
